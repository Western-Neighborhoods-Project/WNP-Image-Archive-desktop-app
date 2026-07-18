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

    // Decode + resize off the async runtime. These are full-resolution images
    // (a scrolled viewport can queue ~20 multi-hundred-MB TIFFs); running the
    // decode on a tokio worker thread would block it for tens of seconds and
    // starve other async commands (order fetches, OpenSF sync). spawn_blocking
    // moves the CPU-bound work to the blocking pool, and generate_and_persist
    // parallelises it — the same path the background worker uses.
    let app_for_blocking = app.clone();
    let (generated, failed) = tauri::async_runtime::spawn_blocking(move || {
        generate_and_persist(&app_for_blocking, &images)
    })
    .await
    .map_err(|e| format!("thumbnail task failed: {}", e))?;

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

    // Write to a unique temp file, then atomically rename into place. The
    // background worker and the on-demand visible-priority path can target the
    // same id concurrently (neither claims rows first); a direct save would let
    // two writers interleave bytes into a corrupt JPEG that then gets marked
    // 'done' and never regenerated. A rename on the same filesystem is atomic,
    // so a reader always sees a complete file and the last writer wins cleanly.
    let counter = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_path = thumb_path.with_extension(format!("tmp{}", counter));
    thumb
        .save_with_format(&tmp_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to save thumbnail: {}", e))?;
    if let Err(e) = std::fs::rename(&tmp_path, thumb_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to finalize thumbnail: {}", e));
    }

    Ok(())
}

/// Monotonic counter for unique thumbnail temp-file names (see the atomic
/// write in generate_thumbnail_for_file). Process-wide so two threads writing
/// the same id never pick the same temp path.
static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Hard cap on concurrent decodes within a batch. image-rs decoding is
/// CPU-bound and self-contained per image, so threads buy a near-linear
/// speed-up. The cap keeps peak memory bounded — each in-flight decode can
/// hold a multi-hundred-MB DynamicImage for large archival TIFFs, so 4×
/// simultaneous is roughly the memory-vs-throughput sweet spot.
const THUMBNAIL_PARALLELISM_CAP: usize = 4;

/// Decode + resize the given `(id, file_path)` images in parallel and persist
/// their done/failed state in a single transaction. Runs entirely on the
/// calling thread — never call it from an async command directly; use
/// `spawn_blocking`. Shared by the on-demand `generate_full_thumbnails` command
/// and the background worker so the decode policy and the done/failed
/// bookkeeping live in exactly one place. Returns `(generated, failed)`.
pub fn generate_and_persist(app: &tauri::AppHandle, images: &[(i64, String)]) -> (u64, u64) {
    use tauri::Manager;
    if images.is_empty() {
        return (0, 0);
    }

    let parallelism = std::thread::available_parallelism()
        .map(|p| p.get().min(THUMBNAIL_PARALLELISM_CAP))
        .unwrap_or(2)
        .max(1);

    // Each thread takes a round-robin slice so one slow image doesn't stall a
    // whole thread's share. std::thread::scope lets the closures borrow into
    // `images` without an 'static bound; DB writes happen serially after the
    // join because the connection is single-writer anyway.
    type ThumbResult = (i64, PathBuf, Result<(), String>);
    let results: Vec<ThumbResult> = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(parallelism);
        for offset in 0..parallelism {
            let slice: Vec<&(i64, String)> = images
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % parallelism == offset)
                .map(|(_, item)| item)
                .collect();
            handles.push(s.spawn(move || {
                slice
                    .into_iter()
                    .map(|(id, file_path)| {
                        let thumb_path = thumbnail_path_for_id(*id);
                        let r = generate_thumbnail_for_file(file_path, &thumb_path);
                        (*id, thumb_path, r)
                    })
                    .collect::<Vec<ThumbResult>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });

    let mut generated = 0u64;
    let mut failed = 0u64;
    let state = app.state::<AppState>();
    if let Ok(mut db) = state.db.lock() {
        if let Ok(tx) = db.transaction() {
            for (id, thumb_path, result) in &results {
                match result {
                    Ok(()) => {
                        let _ = tx.execute(
                            "UPDATE images
                             SET thumbnail_path = ?1,
                                 thumbnail_generated = 1,
                                 thumbnail_state = 'done',
                                 thumbnail_error = NULL
                             WHERE id = ?2",
                            rusqlite::params![thumb_path.to_string_lossy().to_string(), id],
                        );
                        generated += 1;
                    }
                    Err(e) => {
                        log::warn!("thumbnail generation failed for id {}: {}", id, e);
                        let _ = tx.execute(
                            "UPDATE images
                             SET thumbnail_state = 'failed', thumbnail_error = ?1
                             WHERE id = ?2",
                            rusqlite::params![e, id],
                        );
                        failed += 1;
                    }
                }
            }
            let _ = tx.commit();
        }
    }
    (generated, failed)
}
