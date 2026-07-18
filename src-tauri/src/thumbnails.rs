use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

    // The ids we're about to (re)generate — the still-pending subset. Anything
    // the grid asked for that ISN'T here was already resolved, typically by the
    // background worker finishing it before this grid item registered its
    // listener; those need a catch-up `thumbnail:ready` below or they'd never
    // display until the grid reloaded.
    let fetched_ids: std::collections::HashSet<i64> = images.iter().map(|(id, _)| *id).collect();

    // Decode + resize off the async runtime. These are full-resolution images
    // (a scrolled viewport can queue ~20 multi-hundred-MB TIFFs); running the
    // decode on a tokio worker thread would block it for tens of seconds and
    // starve other async commands (order fetches, OpenSF sync). spawn_blocking
    // moves the CPU-bound work to the blocking pool, and generate_and_persist
    // parallelises it — the same path the background worker uses. It emits a
    // per-image `thumbnail:ready` for each id it (re)generates.
    let app_for_blocking = app.clone();
    let (generated, failed) = tauri::async_runtime::spawn_blocking(move || {
        generate_and_persist(&app_for_blocking, &images)
    })
    .await
    .map_err(|e| format!("thumbnail task failed: {}", e))?;

    // Catch-up: signal the requested ids that were already resolved (so their
    // grid items refresh even though generate_and_persist didn't touch them).
    {
        use tauri::Emitter;
        for id in &request.image_ids {
            if !fetched_ids.contains(id) {
                let _ = app.emit("thumbnail:ready", *id);
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

/// How often, at most, to push a footer-progress snapshot while a batch is
/// being consumed. Per-image `thumbnail:ready` events drive grid visibility;
/// this only throttles the (table-scanning) progress event.
const PROGRESS_THROTTLE: Duration = Duration::from_millis(400);

/// Decode + resize the given `(id, file_path)` images and persist each result
/// as soon as it's ready. Runs entirely on the calling thread — never call it
/// from an async command directly; use `spawn_blocking`. Shared by the
/// on-demand `generate_full_thumbnails` command and the background worker so
/// the decode policy and the done/failed bookkeeping live in exactly one place.
///
/// Decoding runs on up to `THUMBNAIL_PARALLELISM_CAP` producer threads, so at
/// most that many large images are ever in memory at once regardless of batch
/// size. A consumer commits each result the instant it arrives (rather than one
/// transaction after the whole batch), so a single slow or failing image only
/// delays itself — the rest commit and become visible immediately — and emits a
/// `thumbnail:ready` event per success so the grid can refresh just that item.
/// Returns `(generated, failed)`.
pub fn generate_and_persist(app: &tauri::AppHandle, images: &[(i64, String)]) -> (u64, u64) {
    use tauri::{Emitter, Manager};
    if images.is_empty() {
        return (0, 0);
    }

    let parallelism = std::thread::available_parallelism()
        .map(|p| p.get().min(THUMBNAIL_PARALLELISM_CAP))
        .unwrap_or(2)
        .max(1);

    type ThumbResult = (i64, PathBuf, Result<(), String>);
    let (tx, rx) = std::sync::mpsc::channel::<ThumbResult>();

    std::thread::scope(|s| {
        // Producers: each takes a round-robin slice (so one slow image doesn't
        // stall a whole thread's share) and streams results as they finish.
        for offset in 0..parallelism {
            let slice: Vec<&(i64, String)> = images
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % parallelism == offset)
                .map(|(_, item)| item)
                .collect();
            let tx = tx.clone();
            s.spawn(move || {
                for (id, file_path) in slice {
                    let thumb_path = thumbnail_path_for_id(*id);
                    let r = generate_thumbnail_for_file(file_path, &thumb_path);
                    // Send failing only means the consumer is already gone.
                    let _ = tx.send((*id, thumb_path, r));
                }
            });
        }
        // Drop the original sender so the consumer's recv() ends once every
        // producer has finished and dropped its clone.
        drop(tx);

        // Consumer (this thread): commit each result immediately, notify the
        // grid item, and refresh the footer on a throttle. The DB connection is
        // single-writer, so brief per-image locks let other commands interleave
        // rather than waiting behind a whole-batch transaction.
        let state = app.state::<AppState>();
        let mut generated = 0u64;
        let mut failed = 0u64;
        let mut last_progress = Instant::now();

        while let Ok((id, thumb_path, result)) = rx.recv() {
            match &result {
                Ok(()) => {
                    if let Ok(db) = state.db.lock() {
                        let _ = db.execute(
                            "UPDATE images
                             SET thumbnail_path = ?1,
                                 thumbnail_generated = 1,
                                 thumbnail_state = 'done',
                                 thumbnail_error = NULL
                             WHERE id = ?2",
                            rusqlite::params![thumb_path.to_string_lossy().to_string(), id],
                        );
                    }
                    generated += 1;
                    let _ = app.emit("thumbnail:ready", id);
                }
                Err(e) => {
                    log::warn!("thumbnail generation failed for id {}: {}", id, e);
                    if let Ok(db) = state.db.lock() {
                        let _ = db.execute(
                            "UPDATE images
                             SET thumbnail_state = 'failed', thumbnail_error = ?1
                             WHERE id = ?2",
                            rusqlite::params![e, id],
                        );
                    }
                    failed += 1;
                }
            }

            if last_progress.elapsed() >= PROGRESS_THROTTLE {
                crate::background_jobs::emit_progress(app, true);
                last_progress = Instant::now();
            }
        }

        (generated, failed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_thumbnail_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("wnp_thumb_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // A 500x400 source PNG (larger than THUMBNAIL_SIZE so it gets resized).
        let src = dir.join("src.png");
        let buf = image::RgbImage::from_fn(500, 400, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        image::DynamicImage::ImageRgb8(buf)
            .save_with_format(&src, image::ImageFormat::Png)
            .unwrap();

        let thumb = dir.join("out.jpg");
        generate_thumbnail_for_file(src.to_str().unwrap(), &thumb).unwrap();

        // Exists, decodes as a real JPEG, and fits within the thumbnail box.
        assert!(thumb.exists(), "thumbnail should exist");
        let decoded = image::open(&thumb).unwrap();
        assert!(
            decoded.width() <= THUMBNAIL_SIZE && decoded.height() <= THUMBNAIL_SIZE,
            "thumbnail {}x{} exceeds {}",
            decoded.width(),
            decoded.height(),
            THUMBNAIL_SIZE
        );

        // The atomic rename must not leave a temp file behind.
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {:?}", leftovers);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
