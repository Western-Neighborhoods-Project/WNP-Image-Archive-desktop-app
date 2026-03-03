/// sharing.rs — Tauri commands for fetching, fulfilling, and failing image orders.
///
/// Async pattern: never hold a MutexGuard across an .await point.
/// Steps that need the DB are wrapped in a short-lived `{ let db = ...; ... }` block
/// so the guard is dropped before any async HTTP/S3 work begins.
use crate::db::AppState;
use crate::export::{create_zip, resize_image_to_path};
use crate::models::{FulfillResult, Order, OrdersResponse};
use aws_sdk_s3::{
    config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region},
    primitives::ByteStream,
    Client as S3Client,
};
use std::path::PathBuf;

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
    let api_url = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        read_setting(&db, "laravel_api_url")?
    };

    let client = reqwest::Client::new();
    let url = format!("{}/api/image-requests", api_url.trim_end_matches('/'));

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
    let (api_url, s3_endpoint, s3_bucket, s3_access_key, s3_secret_key, s3_region, s3_public_base_url, high_px, medium_px, low_px) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        (
            read_setting(&db, "laravel_api_url")?,
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
    let client = reqwest::Client::new();
    let order: Order = {
        let url = format!("{}/api/image-requests/{}", api_url.trim_end_matches('/'), uuid);
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
        "{}/api/image-requests/{}/complete",
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
    let api_url = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        read_setting(&db, "laravel_api_url")?
    };

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/image-requests/{}/fail",
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
