/// sharing.rs — Tauri commands for fetching, fulfilling, and failing image orders,
/// plus the ad-hoc share dialog flow (Plan 5).
///
/// Async pattern: never hold a MutexGuard across an .await point.
/// Steps that need the DB are wrapped in a short-lived `{ let db = ...; ... }` block
/// so the guard is dropped before any async HTTP/S3 work begins.
use crate::auth::current_session;
use crate::db::AppState;
use crate::export::{create_zip, resize_image_to_path};
use crate::models::{CreateShareLinkResult, FulfillResult, Order, OrdersResponse};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use aws_sdk_s3::{
    config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region},
    primitives::ByteStream,
    Client as S3Client,
};
use std::path::{Path, PathBuf};

// ── S3 client builder ────────────────────────────────────────────────────────

fn build_s3_client(endpoint: &str, region: &str, access_key: &str, secret_key: &str) -> S3Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "wnp");
    let config = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .credentials_provider(creds)
        .endpoint_url(endpoint)
        .force_path_style(true)
        .build();
    S3Client::from_conf(config)
}

// ── Settings helpers ─────────────────────────────────────────────────────────

fn read_setting(db: &rusqlite::Connection, key: &str) -> Result<String, String> {
    db.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .map_err(|_| format!("Setting '{}' is not configured", key))
    .and_then(|v| {
        if v.is_empty() {
            Err(format!("Setting '{}' is empty", key))
        } else {
            Ok(v)
        }
    })
}

fn read_setting_opt(db: &rusqlite::Connection, key: &str) -> Option<String> {
    db.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|s| !s.is_empty())
}

// ── HTTP client builder ──────────────────────────────────────────────────────

/// Build a reqwest client with sensible defaults for talking to the
/// OpenSFHistory Laravel API. If `token` is provided, every request the
/// returned client makes will carry an `Authorization: Bearer <token>`
/// header. Also sets `Accept: application/json` so Laravel returns JSON
/// rather than HTML error pages.
fn build_authed_client(token: Option<&str>) -> reqwest::Client {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    if let Some(t) = token {
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", t)) {
            headers.insert(AUTHORIZATION, val);
        }
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ── Resolution tier → pixel dimension ───────────────────────────────────────

fn resolution_px(res_str: &str, high_px: u32, medium_px: u32, low_px: u32) -> u32 {
    match res_str {
        "high" => high_px,
        "medium" => medium_px,
        "low" => low_px,
        _ => medium_px,
    }
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Fetch pending orders from the Laravel API.
#[tauri::command]
pub async fn fetch_orders(state: tauri::State<'_, AppState>) -> Result<OrdersResponse, String> {
    let (api_url, api_token) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        (
            read_setting(&db, "laravel_api_url")?,
            read_setting_opt(&db, "laravel_api_token"),
        )
    };

    let client = build_authed_client(api_token.as_deref());
    let url = format!("{}/image-requests", api_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API returned {}", resp.status()));
    }

    resp.json::<OrdersResponse>()
        .await
        .map_err(|e| format!("Failed to parse orders response: {}", e))
}

/// Fulfill an order: resize images → zip → upload to S3 → notify Laravel → log usage.
#[tauri::command]
pub async fn fulfill_order(
    uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<FulfillResult, String> {
    // ── 1. Read settings (release DB lock before async work) ─────────────────
    let (api_url, api_token, s3_endpoint, s3_bucket, s3_access_key, s3_secret_key, s3_region, s3_public_base_url, high_px, medium_px, low_px) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        (
            read_setting(&db, "laravel_api_url")?,
            read_setting_opt(&db, "laravel_api_token"),
            read_setting(&db, "s3_endpoint")?,
            read_setting(&db, "s3_bucket")?,
            read_setting(&db, "s3_access_key")?,
            read_setting(&db, "s3_secret_key")?,
            read_setting_opt(&db, "s3_region").unwrap_or_else(|| "auto".to_string()),
            read_setting(&db, "s3_public_base_url")?,
            read_setting_opt(&db, "resolution_high_px").and_then(|v| v.parse().ok()).unwrap_or(2048_u32),
            read_setting_opt(&db, "resolution_medium_px").and_then(|v| v.parse().ok()).unwrap_or(1600_u32),
            read_setting_opt(&db, "resolution_low_px").and_then(|v| v.parse().ok()).unwrap_or(800_u32),
        )
    };

    // ── 2. Fetch order details from API ──────────────────────────────────────
    let client = build_authed_client(api_token.as_deref());
    let order: Order = {
        let url = format!("{}/image-requests/{}", api_url.trim_end_matches('/'), uuid);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch order: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("API returned {} for order {}", resp.status(), uuid));
        }

        resp.json::<Order>()
            .await
            .map_err(|e| format!("Failed to parse order: {}", e))?
    };

    // ── 3. Resolve file paths from local DB ──────────────────────────────────
    // (catalog_number, file_path, resolution)
    let items: Vec<(String, String, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for item in &order.items {
            match db.query_row(
                "SELECT file_path FROM images WHERE catalog_number = ?1 LIMIT 1",
                rusqlite::params![item.catalog_number],
                |row| row.get::<_, String>(0),
            ) {
                Ok(path) => result.push((item.catalog_number.clone(), path, item.resolution.clone())),
                Err(_) => {
                    return Err(format!(
                        "Catalog number '{}' not found — cannot fulfill order",
                        item.catalog_number
                    ));
                }
            }
        }
        result
    };

    // ── 4. Resize images into a temp dir ─────────────────────────────────────
    let temp_dir = std::env::temp_dir().join(format!("wnp_order_{}", uuid));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let mut resized: Vec<PathBuf> = Vec::new();

    for (catalog_number, file_path, resolution) in &items {
        let max_dim = resolution_px(resolution, high_px, medium_px, low_px);
        let dest = temp_dir.join(format!("{}.jpg", catalog_number));
        if let Err(e) = resize_image_to_path(std::path::Path::new(file_path), &dest, max_dim, 90) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(e);
        }
        resized.push(dest);
    }

    // ── 5. Create zip archive ─────────────────────────────────────────────────
    let zip_path = temp_dir.join(format!("{}.zip", uuid));
    if let Err(e) = create_zip(&resized, &zip_path) {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(e);
    }

    // ── 6. Upload to S3 ───────────────────────────────────────────────────────
    let s3_key = format!("orders/{}.zip", uuid);
    let zip_url = format!(
        "{}/{}",
        s3_public_base_url.trim_end_matches('/'),
        s3_key
    );

    let zip_bytes = match std::fs::read(&zip_path) {
        Ok(b) => b,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!("Failed to read zip: {}", e));
        }
    };

    let s3 = build_s3_client(&s3_endpoint, &s3_region, &s3_access_key, &s3_secret_key);

    if let Err(e) = s3
        .put_object()
        .bucket(&s3_bucket)
        .key(&s3_key)
        .content_type("application/zip")
        .body(ByteStream::from(zip_bytes))
        .send()
        .await
    {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("S3 upload failed: {}", e));
    }

    // ── 7. Notify Laravel ─────────────────────────────────────────────────────
    let complete_url = format!(
        "{}/image-requests/{}/complete",
        api_url.trim_end_matches('/'),
        uuid
    );

    if let Err(e) = client
        .post(&complete_url)
        .json(&serde_json::json!({ "zip_url": zip_url }))
        .send()
        .await
    {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("Failed to notify API: {}", e));
    }

    // ── 8. Insert usage_log entries ───────────────────────────────────────────
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Collect image IDs first, then insert in a transaction
        let mut log_entries: Vec<(i64, String)> = Vec::new(); // (image_id, resolution)
        for (catalog_number, _file_path, resolution) in &items {
            if let Ok(image_id) = db.query_row(
                "SELECT id FROM images WHERE catalog_number = ?1 LIMIT 1",
                rusqlite::params![catalog_number],
                |row| row.get::<_, i64>(0),
            ) {
                log_entries.push((image_id, resolution.clone()));
            }
        }

        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        for (image_id, resolution) in &log_entries {
            tx.execute(
                "INSERT INTO usage_log (image_id, recipient_email, recipient_name, resolution_sent)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![image_id, order.email, order.name, resolution],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    // ── 9. Clean up temp files ────────────────────────────────────────────────
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(FulfillResult {
        uuid,
        zip_url,
        items_fulfilled: items.len(),
    })
}

/// Mark an order as failed via the Laravel API.
#[tauri::command]
pub async fn fail_order(
    uuid: String,
    reason: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (api_url, api_token) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        (
            read_setting(&db, "laravel_api_url")?,
            read_setting_opt(&db, "laravel_api_token"),
        )
    };

    let client = build_authed_client(api_token.as_deref());
    let url = format!(
        "{}/image-requests/{}/fail",
        api_url.trim_end_matches('/'),
        uuid
    );

    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "reason": reason }))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API returned {}", resp.status()));
    }

    Ok(())
}

// ── Plan 5: Ad-hoc share link ───────────────────────────────────────────────

/// Generate 16 hex chars (8 random bytes) for unguessable share keys.
/// Uses argon2's bundled OsRng so we don't need a separate `rand` crate.
fn random_hex_8() -> String {
    let mut bytes = [0u8; 8];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Create a one-off share link for an image. Resizes (if requested),
/// uploads to B2, then POSTs the share metadata + image URL to
/// OpenSFHistory which sends the email via Postmark.
///
/// Per Plan 5 the desktop app does NOT send mail directly — we always
/// proxy through OpenSFHistory's existing Postmark integration.
#[tauri::command]
pub async fn create_share_link(
    image_id: i64,
    recipient_email: String,
    resolution: String, // "low" | "high" | "full"
    purpose: String,
    state: tauri::State<'_, AppState>,
) -> Result<CreateShareLinkResult, String> {
    // ── 1. Read all settings + image record in one short-lived lock ──────────
    let (
        api_url,
        api_token,
        s3_endpoint,
        s3_bucket,
        s3_access_key,
        s3_secret_key,
        s3_region,
        s3_public_base_url,
        share_prefix,
        high_px,
        low_px,
        catalog_number,
        title,
        file_path,
        expires_at,
    ) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let api_url = read_setting(&db, "laravel_api_url")?;
        let api_token = read_setting_opt(&db, "laravel_api_token");
        let s3_endpoint = read_setting(&db, "s3_endpoint")?;
        let s3_bucket = read_setting(&db, "s3_bucket")?;
        let s3_access_key = read_setting(&db, "s3_access_key")?;
        let s3_secret_key = read_setting(&db, "s3_secret_key")?;
        let s3_region =
            read_setting_opt(&db, "s3_region").unwrap_or_else(|| "auto".to_string());
        let s3_public_base_url = read_setting(&db, "s3_public_base_url")?;
        let share_prefix =
            read_setting_opt(&db, "s3_share_prefix").unwrap_or_else(|| "shares".to_string());
        let high_px = read_setting_opt(&db, "resolution_high_px")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2048_u32);
        let low_px = read_setting_opt(&db, "resolution_low_px")
            .and_then(|v| v.parse().ok())
            .unwrap_or(800_u32);

        // Expires-at: configurable via `share_link_expires_days` setting
        // (default 30). Should match whatever B2 lifecycle rule the
        // bucket has on the share prefix. Computed as ISO 8601 UTC via
        // SQLite's strftime so we don't need a date crate.
        let expires_days: i64 = read_setting_opt(&db, "share_link_expires_days")
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        let expires_modifier = format!("+{} days", expires_days);
        let expires_at: String = db
            .query_row(
                "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?1)",
                rusqlite::params![expires_modifier],
                |r| r.get(0),
            )
            .map_err(|e| format!("Failed to compute expires_at: {}", e))?;

        let (catalog_number, title, file_path) = db
            .query_row(
                "SELECT catalog_number, title, file_path FROM images WHERE id = ?1",
                rusqlite::params![image_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|_| format!("Image {} not found", image_id))?;

        (
            api_url,
            api_token,
            s3_endpoint,
            s3_bucket,
            s3_access_key,
            s3_secret_key,
            s3_region,
            s3_public_base_url,
            share_prefix,
            high_px,
            low_px,
            catalog_number,
            title,
            file_path,
            expires_at,
        )
    };

    let sender_username = current_session(&state)
        .map(|s| s.username)
        .unwrap_or_else(|| "local".to_string());

    // ── 2. Prepare temp file (resize if needed) ───────────────────────────────
    let random_hex = random_hex_8();
    let temp_dir = std::env::temp_dir().join(format!("wnp_share_{}", random_hex));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let temp_dest = temp_dir.join(format!("{}.jpg", catalog_number));

    let max_dim_label = match resolution.as_str() {
        "low" => Some(("Low", low_px)),
        "high" => Some(("High", high_px)),
        "full" => None,
        _ => Some(("High", high_px)),
    };

    let resolution_label = match max_dim_label {
        Some((label, px)) => {
            if let Err(e) = resize_image_to_path(Path::new(&file_path), &temp_dest, px, 90) {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(e);
            }
            format!("{} ({}px)", label, px)
        }
        None => {
            // "full" — copy original to temp so we have a single upload path
            // and don't risk uploading the original's actual fs path by mistake.
            if let Err(e) = std::fs::copy(&file_path, &temp_dest) {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Failed to copy original image: {}", e));
            }
            "Full (original)".to_string()
        }
    };

    // ── 3. Upload to B2 ───────────────────────────────────────────────────────
    let s3_key = format!("{}/{}-{}.jpg", share_prefix, catalog_number, random_hex);
    let image_url = format!(
        "{}/{}",
        s3_public_base_url.trim_end_matches('/'),
        s3_key
    );

    let bytes = match std::fs::read(&temp_dest) {
        Ok(b) => b,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!("Failed to read temp image: {}", e));
        }
    };

    let s3 = build_s3_client(&s3_endpoint, &s3_region, &s3_access_key, &s3_secret_key);
    if let Err(e) = s3
        .put_object()
        .bucket(&s3_bucket)
        .key(&s3_key)
        .content_type("image/jpeg")
        .body(ByteStream::from(bytes))
        .send()
        .await
    {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("S3 upload failed: {}", e));
    }

    // ── 4. POST to OpenSFHistory share-links endpoint ─────────────────────────
    let client = build_authed_client(api_token.as_deref());
    let share_url = format!("{}/share-links", api_url.trim_end_matches('/'));
    let post_payload = serde_json::json!({
        "catalog_number": catalog_number,
        "title": title,
        "recipient_email": recipient_email,
        "purpose": purpose,
        "image_url": image_url,
        "resolution_label": resolution_label,
        "sender_username": sender_username,
        // ISO 8601 UTC timestamp matching the bucket's lifecycle rule.
        // Email template can render this however it likes.
        "expires_at": expires_at,
    });

    let resp = client
        .post(&share_url)
        .json(&post_payload)
        .send()
        .await
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp_dir);
            format!("Failed to call OpenSFHistory share-links: {}", e)
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "OpenSFHistory returned {}: {}",
            status,
            body.chars().take(200).collect::<String>()
        ));
    }

    // ── 5. Insert usage_log + audit_log entries ───────────────────────────────
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO usage_log (image_id, recipient_email, purpose, resolution_sent)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![image_id, recipient_email, purpose, resolution_label],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO audit_log (image_id, field_name, old_value, new_value, changed_by)
             VALUES (?1, 'shared', NULL, ?2, ?3)",
            rusqlite::params![image_id, recipient_email, sender_username],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }

    // ── 6. Cleanup ───────────────────────────────────────────────────────────
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(CreateShareLinkResult {
        image_url,
        recipient_email,
        resolution_label,
    })
}
