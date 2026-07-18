use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use walkdir::WalkDir;

use crate::auth;
use crate::db::AppState;
use crate::models::{ScanResult, ScanStats};
use crate::source_directories;

/// Supported image file extensions (checked case-insensitively).
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "tif", "tiff", "png", "gif", "bmp", "webp",
];

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Catalog numbers flow into S3 keys, audit logs, and (eventually) URLs.
/// The dangerous classes are path separators, traversal sequences, control
/// characters, and null bytes — those let malicious input escape the file
/// path or break logging. Spaces, parens, apostrophes, and the rest of
/// printable ASCII are fine because URLs/S3 keys percent-encode them and
/// the audit log is plain text.
fn sanitize_catalog_number(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return None;
    }
    if trimmed.starts_with('.') || trimmed.contains("..") {
        return None;
    }
    if trimmed
        .chars()
        .any(|c| c == '/' || c == '\\' || c == '\0' || c.is_control())
    {
        return None;
    }
    Some(trimmed.to_string())
}

/// Format a SystemTime as an ISO 8601 string.
fn format_modified(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Simple ISO-8601-ish format: YYYY-MM-DDTHH:MM:SSZ
    // Use chrono if available; for now produce a Unix timestamp string.
    format!("{}", secs)
}

/// Scan a directory for image files and insert new ones into the database.
///
/// Steps:
/// 1. Resolve the path to a `source_directories` row (creating one if absent
///    so legacy callers from setup keep working).
/// 2. Walk directory recursively with walkdir.
/// 3. For each image, compute its `relative_dir` from the source path and
///    INSERT OR IGNORE into `images` with `source_directory_id` + the new
///    column populated.
#[tauri::command]
pub fn scan_directory(path: String, state: tauri::State<AppState>) -> Result<ScanResult, String> {
    auth::require_admin(&state)?;
    let start = Instant::now();

    let root = Path::new(&path);
    if !root.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    // Resolve / create a source_directories row for this path under a
    // short-lived lock, then release it before walking. The recursive WalkDir
    // + per-file metadata pass hits the NAS, not the DB, and can run for
    // minutes on a large source over SMB — holding the single global DB mutex
    // across it would freeze every other command (queries, login, the drive
    // poller) for the whole scan. Returns the id and the canonical
    // (trailing-slash-trimmed) path.
    let (source_directory_id, source_path) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        source_directories::find_or_create(&db, &path)?
    };

    let mut total_files: u64 = 0;
    let mut new_files: u64 = 0;
    let mut walk_errors: u64 = 0;

    // Collect all image files first (fast pass).
    // Tuple: (file_path, catalog_number, file_size, file_modified, archival_collection, relative_dir)
    let mut image_files: Vec<(String, String, Option<i64>, Option<String>, String, String)> =
        Vec::new();

    for entry_result in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => {
                walk_errors += 1;
                log::warn!("scan: walk error: {}", err);
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let entry_path = entry.path();
        if !is_image_file(entry_path) {
            continue;
        }

        let file_path = entry_path.to_string_lossy().to_string();
        let stem = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let catalog_number = match sanitize_catalog_number(stem) {
            Some(c) => c,
            None => {
                // Bumped from debug → warn so users notice when files are
                // being silently skipped. The sanitizer is conservative
                // by design; if it rejects something legitimate we want
                // visibility.
                log::warn!(
                    "scan: skipping {} (invalid catalog number)",
                    entry_path.display()
                );
                continue;
            }
        };
        total_files += 1;
        let parent_dir = entry_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // relative_dir: directory portion of file_path with the source root
        // stripped. Empty string for files directly under the source.
        let trimmed_source = source_path.trim_end_matches('/');
        let stripped = file_path
            .strip_prefix(trimmed_source)
            .unwrap_or(&file_path)
            .trim_start_matches('/');
        let relative_dir = std::path::Path::new(stripped)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let meta = std::fs::metadata(entry_path).ok();
        let file_size = meta.as_ref().map(|m| m.len() as i64);
        let file_modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(format_modified);

        image_files.push((
            file_path,
            catalog_number,
            file_size,
            file_modified,
            parent_dir,
            relative_dir,
        ));
    }

    // Re-acquire the lock only now that the walk is done and we have rows to
    // write, so the mutex is held for the DB write rather than the network walk.
    let mut db = state.db.lock().map_err(|e| e.to_string())?;

    // Batch insert inside a single transaction for maximum performance
    {
        let tx = db.transaction().map_err(|e| e.to_string())?;

        for (file_path, catalog_number, file_size, file_modified, archival_collection, relative_dir) in
            &image_files
        {
            let rows_changed = tx
                .execute(
                    "INSERT OR IGNORE INTO images
                        (file_path, catalog_number, file_size, file_modified,
                         archival_collection, source_directory_id, relative_dir)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        file_path,
                        catalog_number,
                        file_size,
                        file_modified,
                        if archival_collection.is_empty() {
                            None
                        } else {
                            Some(archival_collection.as_str())
                        },
                        source_directory_id,
                        relative_dir,
                    ],
                )
                .map_err(|e| e.to_string())?;
            if rows_changed > 0 {
                new_files += 1;
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
    }

    // Track the most recently scanned source for backwards compatibility
    // with any legacy reads of `last_scan_time`. The per-source entry
    // `source_directories.path` is the canonical record from now on.
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('last_scan_time', datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ScanResult {
        total_files,
        new_files,
        archive_collections_found: 0,
        scan_duration_ms: duration_ms,
        walk_errors,
        source_directory_id,
    })
}

/// Return high-level stats about the current catalog state.
#[tauri::command]
pub fn get_scan_stats(state: tauri::State<AppState>) -> Result<ScanStats, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_images: u64 = db
        .query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let images_with_thumbnails: u64 = db
        .query_row(
            "SELECT COUNT(*) FROM images WHERE thumbnail_path IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let images_without_metadata: u64 = db
        .query_row(
            "SELECT COUNT(*) FROM images WHERE title IS NULL AND city IS NULL AND date_display IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(ScanStats {
        total_images,
        images_with_thumbnails,
        images_without_metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::sanitize_catalog_number;

    #[test]
    fn accepts_normal_catalog_numbers() {
        assert_eq!(
            sanitize_catalog_number("wnp27.4283"),
            Some("wnp27.4283".into())
        );
        assert_eq!(
            sanitize_catalog_number("WNP83-0001"),
            Some("WNP83-0001".into())
        );
        assert_eq!(sanitize_catalog_number("img_001"), Some("img_001".into()));
    }

    #[test]
    fn rejects_traversal_and_separators() {
        assert_eq!(sanitize_catalog_number("../etc/passwd"), None);
        assert_eq!(sanitize_catalog_number(".hidden"), None);
        assert_eq!(sanitize_catalog_number("a/b"), None);
        assert_eq!(sanitize_catalog_number("a\\b"), None);
        assert_eq!(sanitize_catalog_number(""), None);
        assert_eq!(sanitize_catalog_number(&"a".repeat(129)), None);
        // Control chars (\0, \n, \r, etc) — never legitimate in a
        // filename and could break log lines.
        assert_eq!(sanitize_catalog_number("a\0b"), None);
        assert_eq!(sanitize_catalog_number("line1\nline2"), None);
    }

    #[test]
    fn accepts_filename_punctuation() {
        // Common in real-world archive filenames; users were previously
        // silently skipped because the original allowlist was too tight.
        assert_eq!(
            sanitize_catalog_number("Programs page-drew-bird-0603"),
            Some("Programs page-drew-bird-0603".into())
        );
        assert_eq!(
            sanitize_catalog_number("IMG_1234 (1)"),
            Some("IMG_1234 (1)".into())
        );
        assert_eq!(
            sanitize_catalog_number("O'Connor 1925"),
            Some("O'Connor 1925".into())
        );
    }
}
