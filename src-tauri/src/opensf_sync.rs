// OpenSFHistory metadata sync (Plan 9).
//
// Read-only one-way sync from the OpenSFHistory Laravel API into the
// local catalog. The API is the source of truth for shared metadata
// (title, description, location, dates, etc.); local DB also stores
// a few app-only fields (internal_notes, donor, acquisition_date,
// keywords, archival_collection) that the API doesn't know about.
//
// Behaviour:
//   1. Frontend calls `sync_image_from_opensf(image_id, force)` on
//      detail-view mount.
//   2. If `last_synced_at` is within 5 min and !force, we return the
//      existing local record without an HTTP call.
//   3. Otherwise GET <api_url>/photos/<catalog_number>, map fields,
//      UPDATE the local DB, set `last_synced_at = now()`, return the
//      refreshed record.
//   4. Any error (network, 404, JSON parse) is non-fatal: we log and
//      return whatever's in local DB so the detail view keeps working.

use crate::auth;
use crate::db::{read_setting, read_setting_opt, AppState};
use crate::http::{build_authed_client, join_url};
use crate::models::ImageRecord;
use crate::queries::{row_to_image_record, IMAGE_SELECT_COLS};
use rusqlite::Connection;
use serde::Deserialize;

/// 5 minute cache TTL.
const SYNC_TTL_SECONDS: i64 = 300;

// ── API response ────────────────────────────────────────────────────────────

/// The API wraps the photo in a `data` envelope per Laravel's resource
/// convention: `{ "data": { ... } }`.
#[derive(Debug, Deserialize)]
struct OpenSfPhotoEnvelope {
    data: OpenSfPhotoResponse,
}

/// Mirrors the JSON shape from `GET /photo/{catalog_number}`. All
/// fields are optional except `catalog_number` so partial responses
/// don't fail deserialization. Unknown fields are silently ignored.
///
/// `location` and `photosets` are typed as `serde_json::Value` because
/// the API can return them in multiple shapes:
///   - location: `"San Francisco, CA"` (string) OR `{lat, lng}` (object)
///   - photosets: `[]` (empty array) OR `{id: title}` (object)
/// Other ambiguous fields (year as int-or-string) use the same approach.
#[derive(Debug, Deserialize, Default)]
struct OpenSfPhotoResponse {
    #[allow(dead_code)]
    catalog_number: String,
    title: Option<String>,
    caption: Option<String>,
    description: Option<String>,
    page_url: Option<String>,
    date_taken: Option<String>,
    year: Option<serde_json::Value>,
    location: Option<serde_json::Value>,
    dimensions: Option<String>,
    format: Option<String>,
    contributor: Option<String>,
    publisher: Option<String>,
    citation: Option<String>,
    copyright: Option<String>,
    download_permitted: Option<bool>,
    neighborhoods: Option<Vec<String>>,
    photosets: Option<serde_json::Value>,
    collections: Option<Vec<String>>,
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn read_image_by_id(conn: &Connection, image_id: i64) -> Result<ImageRecord, String> {
    let sql = format!("SELECT {} FROM images WHERE id = ?1", IMAGE_SELECT_COLS);
    conn.query_row(&sql, rusqlite::params![image_id], row_to_image_record)
        .map_err(|e| format!("Image {} not found: {}", image_id, e))
}

/// Split a comma-delimited location string into (city, state, country).
/// "San Francisco, CA, USA" → (Some("San Francisco"), Some("CA"), Some("USA"))
/// "San Francisco" → (Some("San Francisco"), None, None)
fn split_location_string(loc: &str) -> (Option<String>, Option<String>, Option<String>) {
    let parts: Vec<String> = loc
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (
        parts.first().cloned(),
        parts.get(1).cloned(),
        parts.get(2).cloned(),
    )
}

/// Resolve the API's `location` field (which can be a string or a
/// `{lat, lng}` object) into local city/state/country columns. The
/// object form has no textual location, so we leave them all None
/// — geocoding could fill them in later if useful.
fn parse_location(
    loc: Option<&serde_json::Value>,
) -> (Option<String>, Option<String>, Option<String>) {
    match loc {
        Some(serde_json::Value::String(s)) if !s.is_empty() => split_location_string(s),
        _ => (None, None, None),
    }
}

/// Drop empty strings so the local DB stays clean (NULL instead of "").
fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}

/// Serialize a photosets value into a stable JSON string. Returns None
/// for the empty-array form (Laravel emits `[]` for empty associative
/// arrays) so we don't store noise.
fn photosets_to_json(v: Option<&serde_json::Value>) -> Option<String> {
    match v? {
        serde_json::Value::Object(map) if !map.is_empty() => {
            serde_json::to_string(&serde_json::Value::Object(map.clone())).ok()
        }
        _ => None,
    }
}

/// Extract the first 4-digit year from a string and produce an ISO date
/// like "1924-01-01". Returns None if no 4 consecutive digits found.
fn year_to_date_start(year: Option<&serde_json::Value>) -> Option<String> {
    let s = match year? {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => return None,
    };
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            if digits.len() == 4 {
                return Some(format!("{}-01-01", digits));
            }
        } else if !digits.is_empty() {
            return None;
        }
    }
    None
}

// ── Command ────────────────────────────────────────────────────────────────

/// Sync a single image's metadata from the OpenSFHistory API.
///
/// Returns the updated `ImageRecord` (or the existing one unchanged if
/// the cache is still fresh / the API call fails). Never errors on the
/// network path — falls back to local data so the detail view keeps
/// rendering.
#[tauri::command]
pub async fn sync_image_from_opensf(
    image_id: i64,
    force: bool,
    state: tauri::State<'_, AppState>,
) -> Result<ImageRecord, String> {
    auth::require_session(&state)?;
    // 1. Read settings + image's catalog_number + cache age in one short lock.
    let (api_url, api_token, catalog_number, elapsed_secs) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let api_url = read_setting(&db, "laravel_api_url")?;
        let api_token = read_setting_opt(&db, "laravel_api_token");
        let (catalog_number, elapsed_secs) = db
            .query_row(
                "SELECT catalog_number,
                        CASE
                          WHEN last_synced_at IS NULL THEN NULL
                          ELSE (strftime('%s','now') - strftime('%s', last_synced_at))
                        END
                 FROM images WHERE id = ?1",
                rusqlite::params![image_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                    ))
                },
            )
            .map_err(|_| format!("Image {} not found", image_id))?;
        (api_url, api_token, catalog_number, elapsed_secs)
    };

    // 2. Cache check: if we synced within the TTL and the caller didn't
    //    request a forced refresh, skip the HTTP call.
    if !force {
        if let Some(secs) = elapsed_secs {
            if secs < SYNC_TTL_SECONDS {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                return read_image_by_id(&db, image_id);
            }
        }
    }

    // 3. Fetch from the API. Any error → fall back to existing local row.
    let client = build_authed_client(api_token.as_deref());
    let url = match join_url(&api_url, &["photos", &catalog_number]) {
        Ok(u) => u,
        Err(_) => {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            return read_image_by_id(&db, image_id);
        }
    };

    log::debug!("opensf_sync: GET {}", url);

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!(
                "opensf_sync: network error for {} ({}): {}",
                catalog_number,
                url,
                e
            );
            let db = state.db.lock().map_err(|e| e.to_string())?;
            return read_image_by_id(&db, image_id);
        }
    };

    if !resp.status().is_success() {
        // 404 etc. — image not in OpenSFHistory or auth failure. Surface
        // nothing alarming; just return what's local.
        log::debug!(
            "opensf_sync: {} returned {} for {}",
            url,
            resp.status(),
            catalog_number
        );
        let db = state.db.lock().map_err(|e| e.to_string())?;
        return read_image_by_id(&db, image_id);
    }

    // Read body as text first so we can log it on parse failure —
    // makes shape mismatches between API and struct trivially debuggable.
    let body_text = match resp.text().await {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "opensf_sync: failed to read body for {}: {}",
                catalog_number,
                e
            );
            let db = state.db.lock().map_err(|e| e.to_string())?;
            return read_image_by_id(&db, image_id);
        }
    };

    let envelope: OpenSfPhotoEnvelope = match serde_json::from_str(&body_text) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("opensf_sync: bad JSON for {}: {}", catalog_number, e);
            log::debug!(
                "opensf_sync: bad-JSON body for {}: {}",
                catalog_number,
                body_text.chars().take(400).collect::<String>()
            );
            let db = state.db.lock().map_err(|e| e.to_string())?;
            return read_image_by_id(&db, image_id);
        }
    };
    let api_data = envelope.data;
    log::debug!("opensf_sync: synced {} successfully", catalog_number);

    // 4. Map API → local columns. Empty strings normalize to NULL so
    //    the local DB stays clean.
    let (city, state_part, country) = parse_location(api_data.location.as_ref());
    let date_start = year_to_date_start(api_data.year.as_ref());
    let download_permitted_int: Option<i64> = api_data.download_permitted.map(|b| if b { 1 } else { 0 });
    let neighborhoods_json = api_data
        .neighborhoods
        .as_ref()
        .filter(|v| !v.is_empty())
        .and_then(|v| serde_json::to_string(v).ok());
    let photosets_json = photosets_to_json(api_data.photosets.as_ref());
    let collections_json = api_data
        .collections
        .as_ref()
        .filter(|v| !v.is_empty())
        .and_then(|v| serde_json::to_string(v).ok());

    // 5. Persist to local DB. Single UPDATE — don't touch local-only
    //    columns (donor, acquisition_date, archival_collection,
    //    internal_notes, keywords, date_end) since the API has no
    //    equivalent and they're user-managed locally.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE images SET
                title              = ?1,
                caption            = ?2,
                description        = ?3,
                city               = ?4,
                state              = ?5,
                country            = ?6,
                date_display       = ?7,
                date_start         = COALESCE(?8, date_start),
                photographer       = ?9,
                usage_rights       = ?10,
                dimensions         = ?11,
                format             = ?12,
                publisher          = ?13,
                citation           = ?14,
                download_permitted = ?15,
                neighborhoods      = ?16,
                photosets          = ?17,
                osf_collections    = ?18,
                osf_page_url       = ?19,
                last_synced_at     = datetime('now'),
                updated_at         = datetime('now')
             WHERE id = ?20",
            rusqlite::params![
                non_empty(api_data.title),
                non_empty(api_data.caption),
                non_empty(api_data.description),
                city,
                state_part,
                country,
                non_empty(api_data.date_taken),
                date_start,
                non_empty(api_data.contributor),
                non_empty(api_data.copyright),
                non_empty(api_data.dimensions),
                non_empty(api_data.format),
                non_empty(api_data.publisher),
                non_empty(api_data.citation),
                download_permitted_int,
                neighborhoods_json,
                photosets_json,
                collections_json,
                non_empty(api_data.page_url),
                image_id,
            ],
        )
        .map_err(|e| format!("Failed to write synced metadata: {}", e))?;

        read_image_by_id(&db, image_id)
    }
}
