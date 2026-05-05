use std::path::{Path, PathBuf};
use std::time::Instant;

use image::imageops::FilterType;

use crate::auth;
use crate::db::{get_thumbnail_cache_dir, AppState};
use crate::models::{ThumbnailRequest, ThumbnailResult};

const THUMBNAIL_SIZE: u32 = 300;

/// Return the thumbnail path for a given image ID.
pub fn thumbnail_path_for_id(id: i64) -> PathBuf {
    get_thumbnail_cache_dir().join(format!("{}.jpg", id))
}

// ============================================================
// On-demand thumbnail generation
// ============================================================
//
// Plan 13 made thumbnail generation primarily a background-worker job
// (see background_jobs.rs). This command stays around for the visible-
// images priority path: the grid's thumbnailQueue.ts calls it for items
// that just scrolled into view so they get thumbnails ahead of the
// FIFO worker order.

/// Generate full-quality (300px, Lanczos3) thumbnails for a batch of image IDs.
/// Updates `thumbnail_state` so the background worker doesn't reprocess
/// the same images.
#[tauri::command]
pub async fn generate_full_thumbnails(
    request: ThumbnailRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ThumbnailResult, String> {
    auth::require_session(&state)?;
    let start = Instant::now();

    if request.image_ids.is_empty() {
        return Ok(ThumbnailResult {
            extracted: 0,
            fallback_generated: 0,
            failed: 0,
            duration_ms: 0,
        });
    }

    // Fetch file paths for the requested IDs.
    // Build an IN clause with literal IDs (safe: IDs are i64, not user strings).
    let id_list = request
        .image_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let images: Vec<(i64, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "SELECT id, file_path FROM images
             WHERE id IN ({})
               AND thumbnail_state = 'pending'",
            id_list
        );
        let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut generated: u64 = 0;
    let mut failed: u64 = 0;

    for (id, file_path) in &images {
        let thumb_path = thumbnail_path_for_id(*id);

        match generate_thumbnail_for_file(file_path, &thumb_path) {
            Ok(()) => {
                update_thumbnail_done(&state, *id, &thumb_path)?;
                generated += 1;
            }
            Err(e) => {
                log::warn!("thumbnail generation failed for {}: {}", file_path, e);
                update_thumbnail_failed(&state, *id, &e)?;
                failed += 1;
            }
        }
    }

    // Visible-priority work resolves rows outside the background worker
    // loop, so push a progress snapshot now — otherwise the footer
    // indicator wouldn't reflect these completions until the worker's
    // next 5s poll.
    crate::background_jobs::emit_progress(&app, false);

    Ok(ThumbnailResult {
        extracted: 0,
        fallback_generated: generated,
        failed,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ============================================================
// Shared Helpers
// ============================================================

/// Resize an image file to fit within THUMBNAIL_SIZE×THUMBNAIL_SIZE,
/// maintaining aspect ratio, and save as JPEG to thumb_path.
///
/// `image::open` defaults to a 512MB allocation cap, which uncompressed
/// archival TIFFs blow past routinely. We trust the source files (no
/// untrusted-input vector — these come from the user's own archive) so
/// run with limits disabled.
///
/// Public so the Plan 13 background worker can call into it directly.
pub fn generate_thumbnail_for_file(file_path: &str, thumb_path: &Path) -> Result<(), String> {
    let mut reader = image::ImageReader::open(file_path)
        .map_err(|e| format!("Failed to open image {}: {}", file_path, e))?
        .with_guessed_format()
        .map_err(|e| format!("Failed to guess format for {}: {}", file_path, e))?;
    reader.no_limits();
    let img = reader
        .decode()
        .map_err(|e| format!("Failed to decode image {}: {}", file_path, e))?;

    let thumb = img.resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, FilterType::Lanczos3);

    thumb
        .save_with_format(thumb_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to save thumbnail: {}", e))?;

    Ok(())
}

fn update_thumbnail_done(
    state: &tauri::State<AppState>,
    image_id: i64,
    thumb_path: &Path,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE images
         SET thumbnail_path = ?1,
             thumbnail_generated = 1,
             thumbnail_state = 'done',
             thumbnail_error = NULL
         WHERE id = ?2",
        rusqlite::params![thumb_path.to_string_lossy().to_string(), image_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn update_thumbnail_failed(
    state: &tauri::State<AppState>,
    image_id: i64,
    error: &str,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE images
         SET thumbnail_state = 'failed', thumbnail_error = ?1
         WHERE id = ?2",
        rusqlite::params![error, image_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
