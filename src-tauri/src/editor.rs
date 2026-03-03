use std::process::Command;
use std::time::Instant;

use crate::db::AppState;
use crate::models::{AuditLogEntry, FilterOptions, MetadataUpdate};
use crate::settings::find_exiftool_binary;

/// Whitelist of editable metadata columns.
/// Prevents SQL injection via field names and restricts editing to user-facing fields.
const EDITABLE_FIELDS: &[&str] = &[
    "title",
    "description",
    "city",
    "state",
    "country",
    "keywords",
    "date_display",
    "date_start",
    "date_end",
    "photographer",
    "donor",
    "acquisition_date",
    "usage_rights",
    "internal_notes",
];

// ============================================================
// Metadata Update
// ============================================================

/// Update image metadata fields and record each change in the audit log.
///
/// The frontend sends only the changed fields (old_value / new_value diff),
/// so we apply each change individually and log it.
#[tauri::command]
pub fn update_image_metadata(
    update: MetadataUpdate,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    if update.changes.is_empty() {
        return Ok(());
    }

    // Validate all field names before touching the DB
    for change in &update.changes {
        if !EDITABLE_FIELDS.contains(&change.field.as_str()) {
            return Err(format!("Field '{}' is not editable", change.field));
        }
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

    for change in &update.changes {
        // Dynamic UPDATE — field name is validated against whitelist above
        let sql = format!(
            "UPDATE images SET {} = ?1, metadata_synced = 0, updated_at = datetime('now') WHERE id = ?2",
            change.field
        );
        tx.execute(&sql, rusqlite::params![change.new_value, update.image_id])
            .map_err(|e| format!("Failed to update field '{}': {}", change.field, e))?;

        // Audit log entry
        tx.execute(
            "INSERT INTO audit_log (image_id, field_name, old_value, new_value, changed_by)
             VALUES (?1, ?2, ?3, ?4, 'local')",
            rusqlite::params![
                update.image_id,
                change.field,
                change.old_value,
                change.new_value,
            ],
        )
        .map_err(|e| format!("Failed to write audit log: {}", e))?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetch the audit log for a single image.
#[tauri::command]
pub fn get_audit_log(
    image_id: i64,
    state: tauri::State<AppState>,
) -> Result<Vec<AuditLogEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, image_id, field_name, old_value, new_value, changed_by, changed_at
             FROM audit_log
             WHERE image_id = ?1
             ORDER BY changed_at DESC
             LIMIT 100",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![image_id], |row| {
            Ok(AuditLogEntry {
                id: row.get(0)?,
                image_id: row.get(1)?,
                field_name: row.get(2)?,
                old_value: row.get(3)?,
                new_value: row.get(4)?,
                changed_by: row.get(5)?,
                changed_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let entries: Vec<AuditLogEntry> = rows.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

// ============================================================
// Write to File
// ============================================================

/// Write the current SQLite metadata for an image back into the file using ExifTool.
///
/// This is deliberately non-fatal: a failure here does not affect the database.
/// The frontend shows an inline error message on failure.
/// Sets metadata_synced = 1 on success.
#[tauri::command]
pub fn write_metadata_to_file(
    image_id: i64,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let start = Instant::now();

    // Read current metadata from DB
    let (file_path, title, description, city, img_state, country, keywords,
         date_start, photographer, usage_rights) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT file_path, title, description, city, state, country, keywords,
                    date_start, photographer, usage_rights
             FROM images WHERE id = ?1",
            rusqlite::params![image_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .map_err(|e| format!("Image not found: {}", e))?
    };

    let exiftool_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        find_exiftool_binary(&db)
    };

    // Build exiftool arguments — only include non-null fields
    let mut args: Vec<String> = vec!["-overwrite_original".to_string(), "-q".to_string()];

    if let Some(v) = &title {
        args.push(format!("-Title={}", v));
        args.push(format!("-ObjectName={}", v));
    }
    if let Some(v) = &description {
        args.push(format!("-Description={}", v));
        args.push(format!("-Caption-Abstract={}", v));
    }
    if let Some(v) = &city {
        args.push(format!("-City={}", v));
    }
    if let Some(v) = &img_state {
        args.push(format!("-Province-State={}", v));
    }
    if let Some(v) = &country {
        args.push(format!("-Country-PrimaryLocationName={}", v));
    }
    if let Some(v) = &photographer {
        args.push(format!("-Creator={}", v));
        args.push(format!("-Artist={}", v));
    }
    if let Some(v) = &usage_rights {
        args.push(format!("-CopyrightNotice={}", v));
    }
    if let Some(v) = &date_start {
        // Convert ISO date back to ExifTool format YYYY:MM:DD
        let exif_date = v.replace('-', ":");
        args.push(format!("-DateTimeOriginal={}", exif_date));
        args.push(format!("-CreateDate={}", exif_date));
    }

    // Keywords — parse JSON array and pass as individual -Keywords args
    if let Some(kw_json) = &keywords {
        if let Ok(kws) = serde_json::from_str::<Vec<String>>(kw_json) {
            for kw in &kws {
                args.push(format!("-Keywords={}", kw));
            }
        }
    }

    args.push(file_path.clone());

    let output = Command::new(&exiftool_path)
        .args(&args)
        .output()
        .map_err(|e| {
            format!(
                "Failed to run exiftool ({}): {}. Install with: brew install exiftool",
                exiftool_path, e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exiftool write failed: {}", stderr));
    }

    // Mark as synced in DB
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "UPDATE images SET metadata_synced = 1 WHERE id = ?1",
            rusqlite::params![image_id],
        )
        .map_err(|e| e.to_string())?;
    }

    let _duration_ms = start.elapsed().as_millis();
    Ok(())
}

// ============================================================
// Recently Viewed
// ============================================================

/// Log that an image was viewed. Inserts or updates the recently_viewed row
/// and prunes to the most recent 30 entries.
#[tauri::command]
pub fn log_image_view(image_id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // UPSERT — update viewed_at if already present
    db.execute(
        "INSERT INTO recently_viewed (image_id, viewed_at)
         VALUES (?1, datetime('now'))
         ON CONFLICT(image_id) DO UPDATE SET viewed_at = datetime('now')",
        rusqlite::params![image_id],
    )
    .map_err(|e| e.to_string())?;

    // Prune to 30 most recent
    db.execute(
        "DELETE FROM recently_viewed WHERE id NOT IN (
             SELECT id FROM recently_viewed ORDER BY viewed_at DESC LIMIT 30
         )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Return the last 30 viewed images with full image records.
#[tauri::command]
pub fn get_recently_viewed(
    state: tauri::State<AppState>,
) -> Result<Vec<crate::models::ImageRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT i.id, i.file_path, i.catalog_number, i.file_size, i.file_modified,
                    i.title, i.description, i.city, i.state, i.country, i.keywords,
                    i.date_display, i.date_start, i.date_end, i.photographer,
                    i.donor, i.acquisition_date, i.archival_collection, i.usage_rights,
                    i.internal_notes, i.thumbnail_path, i.thumbnail_generated,
                    i.metadata_synced, i.created_at, i.updated_at
             FROM recently_viewed rv
             JOIN images i ON i.id = rv.image_id
             ORDER BY rv.viewed_at DESC
             LIMIT 30",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], crate::queries::row_to_image_record)
        .map_err(|e| e.to_string())?;

    let images: Vec<crate::models::ImageRecord> = rows.filter_map(|r| r.ok()).collect();
    Ok(images)
}

// ============================================================
// Filter Options
// ============================================================

/// Return distinct values for filter dropdowns and the year range.
/// Called once on app load and cached by the frontend.
#[tauri::command]
pub fn get_filter_options(state: tauri::State<AppState>) -> Result<FilterOptions, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let cities: Vec<String> = {
        let mut stmt = db
            .prepare("SELECT DISTINCT city FROM images WHERE city IS NOT NULL AND city != '' ORDER BY city")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let photographers: Vec<String> = {
        let mut stmt = db
            .prepare("SELECT DISTINCT photographer FROM images WHERE photographer IS NOT NULL AND photographer != '' ORDER BY photographer")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let (year_min, year_max) = db
        .query_row(
            "SELECT MIN(CAST(SUBSTR(date_start, 1, 4) AS INTEGER)),
                    MAX(CAST(SUBSTR(date_start, 1, 4) AS INTEGER))
             FROM images WHERE date_start IS NOT NULL AND length(date_start) >= 4",
            [],
            |row| Ok((row.get::<_, Option<i32>>(0)?, row.get::<_, Option<i32>>(1)?)),
        )
        .unwrap_or((None, None));

    Ok(FilterOptions {
        cities,
        photographers,
        year_min,
        year_max,
    })
}
