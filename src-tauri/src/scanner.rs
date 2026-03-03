use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use walkdir::WalkDir;

use crate::db::AppState;
use crate::models::{ScanResult, ScanStats};

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
/// 1. Walk directory recursively with walkdir.
/// 2. For each image, collect metadata and INSERT OR IGNORE into `images`.
/// 3. Auto-create archive collections from unique parent directory names.
/// 4. Populate `collection_images` junction for archive collections.
#[tauri::command]
pub fn scan_directory(path: String, state: tauri::State<AppState>) -> Result<ScanResult, String> {
    let start = Instant::now();
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let root = Path::new(&path);
    if !root.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    let mut total_files: u64 = 0;
    let mut new_files: u64 = 0;

    // Collect all image files first (fast pass)
    let mut image_files: Vec<(String, String, Option<i64>, Option<String>, String)> = Vec::new();
    // (file_path, catalog_number, file_size, file_modified, parent_dir_name)

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let entry_path = entry.path();
        if !is_image_file(entry_path) {
            continue;
        }
        total_files += 1;

        let file_path = entry_path.to_string_lossy().to_string();
        let catalog_number = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let parent_dir = entry_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let meta = std::fs::metadata(entry_path).ok();
        let file_size = meta.as_ref().map(|m| m.len() as i64);
        let file_modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(format_modified);

        image_files.push((file_path, catalog_number, file_size, file_modified, parent_dir));
    }

    // Batch insert inside a single transaction for maximum performance
    {
        let tx = db
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;

        for (file_path, catalog_number, file_size, file_modified, archival_collection) in
            &image_files
        {
            let rows_changed = tx
                .execute(
                    "INSERT OR IGNORE INTO images
                        (file_path, catalog_number, file_size, file_modified, archival_collection)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        file_path,
                        catalog_number,
                        file_size,
                        file_modified,
                        if archival_collection.is_empty() {
                            None
                        } else {
                            Some(archival_collection.as_str())
                        }
                    ],
                )
                .map_err(|e| e.to_string())?;
            if rows_changed > 0 {
                new_files += 1;
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
    }

    // Auto-create archive collections from unique parent directory names
    let archive_collections_found = create_archive_collections(&db)?;

    // Store the last scan time and source directory
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('source_directory', ?1)",
        rusqlite::params![path],
    )
    .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('last_scan_time', datetime('now'))",
        [],
    )
    .map_err(|e| e.to_string())?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ScanResult {
        total_files,
        new_files,
        archive_collections_found,
        scan_duration_ms: duration_ms,
    })
}

/// For each unique `archival_collection` value in the images table, ensure a
/// corresponding collection row exists (`source = 'archive'`), then populate
/// the `collection_images` junction table.
///
/// Returns the number of unique archive collections found.
fn create_archive_collections(db: &rusqlite::Connection) -> Result<u64, String> {
    // Get all distinct archival_collection values
    let mut stmt = db
        .prepare(
            "SELECT DISTINCT archival_collection FROM images
             WHERE archival_collection IS NOT NULL AND archival_collection != ''",
        )
        .map_err(|e| e.to_string())?;

    let collection_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let count = collection_names.len() as u64;

    let tx = db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;

    for name in &collection_names {
        // Create the collection if it doesn't already exist
        tx.execute(
            "INSERT OR IGNORE INTO collections (name, source, description)
             VALUES (?1, 'archive', ?2)",
            rusqlite::params![name, format!("Archive folder: {}", name)],
        )
        .map_err(|e| e.to_string())?;

        // Get the collection ID
        let collection_id: i64 = tx
            .query_row(
                "SELECT id FROM collections WHERE name = ?1 AND source = 'archive'",
                rusqlite::params![name],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        // Link all images with this archival_collection to the collection
        tx.execute(
            "INSERT OR IGNORE INTO collection_images (collection_id, image_id)
             SELECT ?1, id FROM images WHERE archival_collection = ?2",
            rusqlite::params![collection_id, name],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Return high-level stats about the current catalog state.
#[tauri::command]
pub fn get_scan_stats(state: tauri::State<AppState>) -> Result<ScanStats, String> {
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
