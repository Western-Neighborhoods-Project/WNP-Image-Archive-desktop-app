use std::process::Command;
use std::time::Instant;

use crate::auth;
use crate::db::AppState;
use crate::models::{
    AuditLogEntry, AuditLogGlobalEntry, FilterOptions, MetadataUpdate, RecentActivityEntry,
};
use crate::settings::find_exiftool_binary;

/// Whitelist of editable metadata columns. Prevents SQL injection via
/// field names and restricts editing to user-facing fields.
///
/// Plan 9 update: the OpenSFHistory API is the source of truth for
/// metadata that overlaps with the website (title, description, city,
/// state, country, date_display, date_start, photographer, usage_rights).
/// Those fields have been removed from the whitelist while we're in the
/// read-only sync phase — the desktop app surfaces them as locked
/// inputs. When push-back is wired in a future plan, they're re-added.
///
/// Local-only fields (donor, acquisition_date, internal_notes,
/// keywords, date_end) stay editable indefinitely because they have no
/// OpenSFHistory equivalent.
const EDITABLE_FIELDS: &[&str] = &[
    "keywords",
    "date_end",
    "donor",
    "acquisition_date",
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
    let session = auth::require_session(&state)?;

    if update.changes.is_empty() {
        return Ok(());
    }

    // Validate all field names before touching the DB
    for change in &update.changes {
        if !EDITABLE_FIELDS.contains(&change.field.as_str()) {
            return Err(format!("Field '{}' is not editable", change.field));
        }
    }

    // Plan 10/11: attribute the audit-log entry to the active session.
    let actor = session.username;

    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;

    for change in &update.changes {
        // Dynamic UPDATE — field name is validated against whitelist above
        let sql = format!(
            "UPDATE images SET {} = ?1, metadata_synced = 0, updated_at = datetime('now') WHERE id = ?2",
            change.field
        );
        tx.execute(&sql, rusqlite::params![change.new_value, update.image_id])
            .map_err(|e| format!("Failed to update field '{}': {}", change.field, e))?;

        // Audit log entry — `changed_by` carries the active username.
        tx.execute(
            "INSERT INTO audit_log (image_id, field_name, old_value, new_value, changed_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                update.image_id,
                change.field,
                change.old_value,
                change.new_value,
                actor,
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
    auth::require_session(&state)?;
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

/// Fetch the most recent audit-log entries across all images, joined
/// with the images table for the catalog number. Used by the sidebar
/// ActivityCard.
#[tauri::command]
pub fn get_recent_activity(
    limit: i64,
    state: tauri::State<AppState>,
) -> Result<Vec<RecentActivityEntry>, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT a.id, a.changed_by, i.catalog_number, a.field_name, a.new_value, a.changed_at
             FROM audit_log a
             JOIN images i ON i.id = a.image_id
             ORDER BY a.changed_at DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(RecentActivityEntry {
                id: row.get(0)?,
                changed_by: row.get(1)?,
                catalog_number: row.get(2)?,
                field_name: row.get(3)?,
                new_value: row.get(4)?,
                changed_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let entries: Vec<RecentActivityEntry> = rows.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

/// Global paginated audit-log query for the Audit log view.
///
/// All filter params are optional — pass `None` for an unfiltered query.
/// Date params (`since`, `until`) compare against `audit_log.changed_at`
/// which is stored in SQLite's `'YYYY-MM-DD HH:MM:SS'` text format, so
/// callers should pass strings in the same shape (lexicographic sort
/// matches chronological sort for that format).
#[tauri::command]
pub fn get_audit_log_global(
    field_name: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: i64,
    offset: i64,
    state: tauri::State<AppState>,
) -> Result<Vec<AuditLogGlobalEntry>, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT a.id, a.image_id, i.catalog_number, a.field_name,
                    a.old_value, a.new_value, a.changed_by, a.changed_at
             FROM audit_log a
             JOIN images i ON i.id = a.image_id
             WHERE (?1 IS NULL OR a.field_name = ?1)
               AND (?2 IS NULL OR a.changed_at >= ?2)
               AND (?3 IS NULL OR a.changed_at <= ?3)
             ORDER BY a.changed_at DESC
             LIMIT ?4 OFFSET ?5",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(
            rusqlite::params![field_name, since, until, limit, offset],
            |row| {
                Ok(AuditLogGlobalEntry {
                    id: row.get(0)?,
                    image_id: row.get(1)?,
                    catalog_number: row.get(2)?,
                    field_name: row.get(3)?,
                    old_value: row.get(4)?,
                    new_value: row.get(5)?,
                    changed_by: row.get(6)?,
                    changed_at: row.get(7)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<AuditLogGlobalEntry> = rows.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

/// Export the audit log to a CSV file at `path`. Same filter params as
/// `get_audit_log_global` minus pagination (the export is unbounded).
/// Returns the number of rows written so the UI can show a confirmation.
#[tauri::command]
pub fn export_audit_log_csv(
    field_name: Option<String>,
    since: Option<String>,
    until: Option<String>,
    path: String,
    state: tauri::State<AppState>,
) -> Result<u64, String> {
    auth::require_session(&state)?;

    // Defense in depth: the path comes from a native save dialog in normal use,
    // but this command must not become an arbitrary-file-overwrite primitive for
    // a compromised webview. Require an absolute path ending in .csv so it can't
    // clobber a dotfile, the app's own DB, or a LaunchAgent plist.
    let out_path = std::path::Path::new(&path);
    if !out_path.is_absolute() {
        return Err("Export path must be absolute".to_string());
    }
    let is_csv = out_path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    if !is_csv {
        return Err("Export path must end in .csv".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT a.id, i.catalog_number, a.field_name,
                    a.old_value, a.new_value, a.changed_by, a.changed_at
             FROM audit_log a
             JOIN images i ON i.id = a.image_id
             WHERE (?1 IS NULL OR a.field_name = ?1)
               AND (?2 IS NULL OR a.changed_at >= ?2)
               AND (?3 IS NULL OR a.changed_at <= ?3)
             ORDER BY a.changed_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(
            rusqlite::params![field_name, since, until],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    let mut out = String::new();
    out.push_str("id,catalog_number,field_name,old_value,new_value,changed_by,changed_at\n");

    let mut count: u64 = 0;
    for row in rows {
        let (id, catalog, field, old, new, by, at) = row.map_err(|e| e.to_string())?;
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            id,
            csv_escape(&catalog),
            csv_escape(&field),
            csv_escape(old.as_deref().unwrap_or("")),
            csv_escape(new.as_deref().unwrap_or("")),
            csv_escape(&by),
            csv_escape(&at),
        ));
        count += 1;
    }

    std::fs::write(&path, out).map_err(|e| format!("Failed to write CSV: {}", e))?;
    Ok(count)
}

/// CSV-escape a single field per RFC 4180 + spreadsheet formula-injection
/// hardening (CWE-1236). If the field starts with one of the formula-trigger
/// chars (=, +, -, @, tab, CR), prepend a single quote so Excel/Numbers/
/// LibreOffice render it as text instead of executing it. Then standard
/// RFC 4180 quoting handles the rest.
fn csv_escape(s: &str) -> String {
    let needs_prefix = s
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    let prefixed = if needs_prefix {
        format!("'{}", s)
    } else {
        s.to_string()
    };
    if prefixed.contains(',')
        || prefixed.contains('"')
        || prefixed.contains('\n')
        || prefixed.contains('\r')
    {
        let mut buf = String::with_capacity(prefixed.len() + 2);
        buf.push('"');
        for ch in prefixed.chars() {
            if ch == '"' {
                buf.push('"');
                buf.push('"');
            } else {
                buf.push(ch);
            }
        }
        buf.push('"');
        buf
    } else {
        prefixed
    }
}

#[cfg(test)]
mod tests {
    use super::csv_escape;

    #[test]
    fn formula_triggers_get_prefixed_quote() {
        assert_eq!(csv_escape("=cmd|'/c calc'!A1"), "'=cmd|'/c calc'!A1");
        assert_eq!(csv_escape("+1234"), "'+1234");
        assert_eq!(csv_escape("-5"), "'-5");
        assert_eq!(csv_escape("@SUM(A1)"), "'@SUM(A1)");
    }

    #[test]
    fn formula_trigger_with_csv_chars_gets_both_prefix_and_wrap() {
        // Input has formula trigger AND a comma → quote-wrap kicks in too.
        assert_eq!(csv_escape("=A1,B1"), "\"'=A1,B1\"");
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("San Francisco"), "San Francisco");
    }

    #[test]
    fn rfc4180_still_works() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("she said \"hi\""), "\"she said \"\"hi\"\"\"");
    }
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
    auth::require_session(&state)?;
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

    // `--` tells exiftool to stop parsing flags. Defends against file paths
    // that begin with `-` being misinterpreted as flags.
    args.push("--".to_string());
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
    auth::require_session(&state)?;
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
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Use the canonical IMAGE_SELECT_COLS so this query stays in sync
    // automatically when new columns are added to ImageRecord. Subquery
    // keeps the join out of the column-name namespace (recently_viewed
    // has its own `id`, which would collide with `images.id`).
    let sql = format!(
        "SELECT {} FROM images
         WHERE id IN (SELECT image_id FROM recently_viewed)
         ORDER BY (
             SELECT viewed_at FROM recently_viewed WHERE recently_viewed.image_id = images.id
         ) DESC
         LIMIT 30",
        crate::queries::IMAGE_SELECT_COLS
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;

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
    auth::require_session(&state)?;
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
