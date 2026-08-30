use crate::auth;
use crate::db::{get_thumbnail_cache_dir, AppState};

/// Locate the exiftool binary.
///
/// Resolution order:
/// 1. `exiftool_path` key in app_settings (user override)
/// 2. Common Homebrew install paths on macOS (`/opt/homebrew/bin`, `/usr/local/bin`)
/// 3. Bare `"exiftool"` (relies on PATH — works in dev but may not in bundled app)
///
/// Tauri apps run with a minimal PATH that typically excludes `/opt/homebrew/bin`
/// and `/usr/local/bin`, so we probe those paths explicitly.
pub fn find_exiftool_binary(db: &rusqlite::Connection) -> String {
    // 1. Check user-configured path in app_settings
    if let Ok(path) = db.query_row(
        "SELECT value FROM app_settings WHERE key = 'exiftool_path'",
        [],
        |row| row.get::<_, String>(0),
    ) {
        if !path.is_empty() {
            return path;
        }
    }

    // 2. Probe Homebrew paths
    find_exiftool_binary_nodb()
}

/// Same resolution as `find_exiftool_binary` but without a DB connection.
/// Used by commands that don't take AppState (e.g. `extract_metadata_single`).
pub fn find_exiftool_binary_nodb() -> String {
    for candidate in &["/opt/homebrew/bin/exiftool", "/usr/local/bin/exiftool"] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "exiftool".to_string()
}

/// Settings whose values contain credentials. Surfaced to the rest of the
/// codebase so admin-only commands can split secret reads from public ones.
/// Even with plaintext app_settings storage, the split is useful: editors
/// must never get a path that returns these via the public command, and
/// devtools `invoke('get_setting')` from an editor session is admin-gated
/// for these keys.
const SECRET_KEYS: &[&str] = &[
    "s3_secret_key",
    "s3_access_key",
    "laravel_api_token",
    "github_issues_token",
];

pub fn is_secret(key: &str) -> bool {
    SECRET_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_issues_token_is_secret() {
        assert!(is_secret("github_issues_token"));
    }

    #[test]
    fn debug_reporting_flag_and_repo_are_public() {
        assert!(!is_secret("debug_reporting_enabled"));
        assert!(!is_secret("github_issues_repo"));
    }
}

fn inner_get_setting(
    key: &str,
    state: &tauri::State<AppState>,
) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Read a non-secret setting. Available to any logged-in user. Rejects
/// secret keys outright (callers needing those go through `get_setting`,
/// which requires admin).
#[tauri::command]
pub fn get_public_setting(
    key: String,
    state: tauri::State<AppState>,
) -> Result<Option<String>, String> {
    auth::require_session(&state)?;
    if is_secret(&key) {
        return Err(format!("Setting '{}' is not public", key));
    }
    inner_get_setting(&key, &state)
}

/// Read any setting from the app_settings key-value store. For secret keys
/// requires admin role.
#[tauri::command]
pub fn get_setting(key: String, state: tauri::State<AppState>) -> Result<Option<String>, String> {
    auth::require_session(&state)?;
    if is_secret(&key) {
        auth::require_admin(&state)?;
    }
    inner_get_setting(&key, &state)
}

/// Set a value in the app_settings key-value store.
#[tauri::command]
pub fn set_setting(
    key: String,
    value: String,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    auth::require_admin(&state)?;
    // Reject http:// API URLs — credentials would travel in cleartext.
    // Allow http://localhost for local development.
    if key == "laravel_api_url" && !value.is_empty() {
        let trimmed = value.trim_start();
        let lower = trimmed.to_lowercase();
        if !lower.starts_with("https://")
            && !lower.starts_with("http://localhost")
            && !lower.starts_with("http://127.0.0.1")
        {
            return Err("API URL must use HTTPS (or localhost for development)".to_string());
        }
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Reset the entire catalog: delete all images, collections, thumbnails, and
/// clear relevant app_settings. The original image files are NOT affected.
///
/// After this call the app should navigate back to the setup screen.
#[tauri::command]
pub fn reset_catalog(state: tauri::State<AppState>) -> Result<(), String> {
    auth::require_admin(&state)?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;

    let tx = db.transaction().map_err(|e| e.to_string())?;

    // Clear all data tables. source_directories goes too so the user
    // lands back on the setup screen and can pick a fresh root (or
    // multiple).
    tx.execute_batch(
        "
        DELETE FROM audit_log;
        DELETE FROM usage_log;
        DELETE FROM recently_viewed;
        DELETE FROM collection_images;
        DELETE FROM smart_collections;
        DELETE FROM collections;
        DELETE FROM image_requests;
        DELETE FROM images;
        DELETE FROM images_fts;
        DELETE FROM source_directories;
        ",
    )
    .map_err(|e| e.to_string())?;

    // Clear catalog-related settings (keep API URLs, backup config, etc.)
    tx.execute_batch(
        "
        DELETE FROM app_settings WHERE key IN (
            'source_directory',
            'last_scan_time'
        );
        ",
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    // Delete all cached thumbnails from disk
    let cache_dir = get_thumbnail_cache_dir();
    if cache_dir.exists() {
        for entry in std::fs::read_dir(&cache_dir).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(())
}
