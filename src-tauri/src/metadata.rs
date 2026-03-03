use std::process::Command;
use std::time::Instant;

use crate::db::AppState;
use crate::models::{ExtractedMetadata, MetadataImportResult};

// ============================================================
// ExifTool Adapter
// ============================================================

/// Resolve the path to the exiftool binary.
/// Checks app_settings first, then falls back to PATH.
fn get_exiftool_path(db: &rusqlite::Connection) -> String {
    db.query_row(
        "SELECT value FROM app_settings WHERE key = 'exiftool_path'",
        [],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| "exiftool".to_string())
}

/// Parse a JSON array of ExifTool results into ExtractedMetadata structs.
/// Handles the adapter pattern: maps ExifTool field names to our schema.
fn parse_exiftool_output(json: &str) -> Vec<ExtractedMetadata> {
    let Ok(array) = serde_json::from_str::<Vec<serde_json::Value>>(json) else {
        return Vec::new();
    };

    array.into_iter().filter_map(parse_single_entry).collect()
}

/// Map a single ExifTool JSON object to our ExtractedMetadata struct.
/// This is the "exiftool adapter" — add alternative adapters (csv, json export)
/// as separate functions following the same ExtractedMetadata return type.
fn parse_single_entry(obj: serde_json::Value) -> Option<ExtractedMetadata> {
    let map = obj.as_object()?;

    let file_path = map
        .get("SourceFile")
        .and_then(|v| v.as_str())
        .map(str::to_string)?;

    // Title: prefer IPTC ObjectName, fall back to XMP Title
    let title = first_string(map, &["Title", "ObjectName"]);

    // Description: prefer IPTC Caption-Abstract, fall back to XMP Description
    let description = first_string(map, &["Description", "Caption-Abstract", "ImageDescription"]);

    // Location fields
    let city = first_string(map, &["City"]);
    let state = first_string(map, &["Province-State", "State"]);
    let country = first_string(map, &["Country-PrimaryLocationName", "Country"]);

    // Keywords — may be a string or an array in ExifTool JSON output
    let keywords: Option<Vec<String>> = map.get("Keywords").map(|v| match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        serde_json::Value::String(s) => {
            // Comma-separated string
            s.split(',').map(|k| k.trim().to_string()).collect()
        }
        _ => vec![],
    });

    // Date: prefer DateTimeOriginal (EXIF), then CreateDate (EXIF), then DateCreated (IPTC)
    let date_start = first_string(map, &["DateTimeOriginal", "CreateDate", "DateCreated"])
        .map(|d| normalize_date(&d));

    // Photographer / creator
    let photographer = first_string(map, &["Creator", "Artist", "By-line", "Author"]);

    // Usage rights
    let usage_rights = first_string(map, &["CopyrightNotice", "Rights", "Copyright"]);

    Some(ExtractedMetadata {
        file_path,
        title,
        description,
        city,
        state,
        country,
        keywords,
        date_start,
        photographer,
        usage_rights,
    })
}

/// Return the first non-empty string value found among the given keys.
fn first_string(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    for key in keys {
        if let Some(serde_json::Value::String(s)) = map.get(*key) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Attempt to normalize a date string to ISO 8601 (YYYY-MM-DD).
/// ExifTool dates often look like "2023:06:15 10:30:00" or "2023:06:15".
fn normalize_date(s: &str) -> String {
    // ExifTool format: "YYYY:MM:DD HH:MM:SS"
    let date_part = s.split_whitespace().next().unwrap_or(s);
    date_part.replace(':', "-")
}

// ============================================================
// Tauri Commands
// ============================================================

/// Run ExifTool on an entire directory, extract metadata for all images,
/// and update the database. Returns summary statistics.
///
/// This is an async command because it spawns a long-running subprocess.
#[tauri::command]
pub async fn extract_metadata_batch(
    directory: String,
    state: tauri::State<'_, AppState>,
) -> Result<MetadataImportResult, String> {
    let start = Instant::now();

    let exiftool_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_exiftool_path(&db)
    };

    // Run exiftool on the entire directory in one pass:
    // -json: output JSON
    // -r: recursive
    // -fast2: skip MakerNotes (faster, adequate for catalog metadata)
    // -q: quiet (suppress progress messages)
    let output = Command::new(&exiftool_path)
        .args(["-json", "-r", "-fast2", "-q", &directory])
        .output()
        .map_err(|e| {
            format!(
                "Failed to run exiftool ({}): {}. Is exiftool installed? Run: brew install exiftool",
                exiftool_path, e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exiftool exited with error: {}", stderr));
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let entries = parse_exiftool_output(&json);

    let mut processed: u64 = 0;
    let mut updated: u64 = 0;
    let mut errors: u64 = 0;

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;

        for entry in &entries {
            processed += 1;

            // Convert keywords Vec<String> → JSON array string
            let keywords_json = entry.keywords.as_ref().and_then(|kws| {
                if kws.is_empty() {
                    None
                } else {
                    serde_json::to_string(kws).ok()
                }
            });

            let result = tx.execute(
                "UPDATE images SET
                    title            = COALESCE(?2, title),
                    description      = COALESCE(?3, description),
                    city             = COALESCE(?4, city),
                    state            = COALESCE(?5, state),
                    country          = COALESCE(?6, country),
                    keywords         = COALESCE(?7, keywords),
                    date_start       = COALESCE(?8, date_start),
                    photographer     = COALESCE(?9, photographer),
                    usage_rights     = COALESCE(?10, usage_rights),
                    metadata_synced  = 1,
                    updated_at       = datetime('now')
                 WHERE file_path = ?1",
                rusqlite::params![
                    entry.file_path,
                    entry.title,
                    entry.description,
                    entry.city,
                    entry.state,
                    entry.country,
                    keywords_json,
                    entry.date_start,
                    entry.photographer,
                    entry.usage_rights,
                ],
            );

            match result {
                Ok(rows) if rows > 0 => updated += 1,
                Ok(_) => {} // file not in DB yet (scanned after exiftool ran)
                Err(e) => {
                    eprintln!("metadata update error for {}: {}", entry.file_path, e);
                    errors += 1;
                }
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(MetadataImportResult {
        processed,
        updated,
        errors,
        duration_ms,
    })
}

/// Run ExifTool on a single file and return the parsed metadata.
/// Used for refreshing a single image or when a new file is detected.
#[tauri::command]
pub fn extract_metadata_single(file_path: String) -> Result<ExtractedMetadata, String> {
    let output = Command::new("exiftool")
        .args(["-json", "-fast2", &file_path])
        .output()
        .map_err(|e| format!("Failed to run exiftool: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "exiftool failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let mut entries = parse_exiftool_output(&json);

    entries
        .pop()
        .ok_or_else(|| format!("No metadata found for: {}", file_path))
}
