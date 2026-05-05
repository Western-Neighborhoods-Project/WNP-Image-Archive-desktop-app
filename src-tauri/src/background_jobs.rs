//! Background worker for thumbnail and metadata extraction (Plan 13).
//!
//! Decouples extraction from the scan/import path. Scans return as soon
//! as image rows are inserted; this worker picks up the leftover work
//! (`thumbnail_state = 'pending'`, `metadata_state = 'pending'`) and
//! processes it in batches over time. The frontend listens for
//! `background:progress` events to drive the footer indicator.
//!
//! Lifecycle: one worker thread spawned in `lib.rs::run` setup. Polls
//! every 5s when idle; processes eagerly when work exists. The
//! AtomicBool shutdown flag is wired so a future restart-in-place could
//! signal the loop to exit cleanly (parity with `drive::spawn_drive_poller`).

use crate::auth;
use crate::db::AppState;
use crate::metadata;
use crate::models::{BackgroundProgress, FailureRecord, ImageProgress, JobStateCounts};
use crate::settings::find_exiftool_binary_nodb;
use crate::thumbnails;
use rusqlite::params;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// Thumbnail batch size — modest so the lock isn't held forever and so
/// progress events keep firing for the footer indicator.
const THUMBNAIL_BATCH_SIZE: i64 = 32;
/// Hard cap on concurrent decodes within a batch. image-rs decoding is
/// CPU-bound and fully self-contained per image, so threads buy a
/// near-linear speed-up. Cap is chosen to keep peak memory bounded —
/// each in-flight decode can hold a multi-hundred-MB DynamicImage for
/// large archival TIFFs, so 4× simultaneous gives roughly the
/// memory-vs-throughput sweet spot on typical archives.
const THUMBNAIL_PARALLELISM_CAP: usize = 4;

// ── Worker lifecycle ──────────────────────────────────────────────────────

pub fn spawn_worker(app: AppHandle) -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    std::thread::spawn(move || {
        loop {
            if shutdown_clone.load(Ordering::Relaxed) {
                log::debug!("background_jobs: shutdown signaled, exiting");
                break;
            }
            let did_work = run_one_cycle(&app);
            if !did_work {
                // Emit one last idle-state event so the footer pill flips
                // to "ready" rather than staying on the last "busy" tick.
                emit_progress(&app, false);
                std::thread::sleep(POLL_INTERVAL);
            }
        }
    });
    shutdown
}

fn run_one_cycle(app: &AppHandle) -> bool {
    let metadata_did_work = run_metadata_pass(app).unwrap_or_else(|e| {
        log::warn!("background_jobs: metadata pass failed: {}", e);
        false
    });
    let thumbnail_did_work = run_thumbnail_batch(app).unwrap_or_else(|e| {
        log::warn!("background_jobs: thumbnail batch failed: {}", e);
        false
    });
    metadata_did_work || thumbnail_did_work
}

// ── Metadata pass ─────────────────────────────────────────────────────────

fn run_metadata_pass(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();

    // Distinct source directories that have at least one pending metadata
    // image. Running exiftool over a whole source dir is dramatically
    // faster than per-image subprocess overhead.
    let dirs: Vec<String> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT DISTINCT s.path
                 FROM source_directories s
                 JOIN images i ON i.source_directory_id = s.id
                 WHERE i.metadata_state = 'pending'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    if dirs.is_empty() {
        return Ok(false);
    }

    emit_progress(app, true);
    let exiftool = find_exiftool_binary_nodb();

    for dir in dirs {
        match metadata::extract_metadata_for_directory(&dir, &exiftool) {
            Ok(entries) => apply_metadata_results(&state, &dir, &entries)?,
            Err(e) => {
                log::warn!("background_jobs: exiftool failed for {}: {}", dir, e);
                mark_dir_metadata_failed(&state, &dir, &e)?;
            }
        }
        emit_progress(app, true);
    }

    Ok(true)
}

/// For every image in `directory` that's marked `metadata_state = 'pending'`,
/// either copy in the parsed fields and mark `done`, or — if exiftool
/// returned no row for that file_path — mark `failed` with a clear error.
fn apply_metadata_results(
    state: &State<AppState>,
    directory: &str,
    entries: &[crate::models::ExtractedMetadata],
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;

    // Collect every pending image under this source path so we can detect
    // ones exiftool didn't return data for.
    let pending: std::collections::HashSet<String> = {
        let mut stmt = tx
            .prepare(
                "SELECT i.file_path
                 FROM images i
                 JOIN source_directories s ON s.id = i.source_directory_id
                 WHERE s.path = ?1 AND i.metadata_state = 'pending'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![directory], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut updated_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for entry in entries {
        if !pending.contains(&entry.file_path) {
            continue;
        }
        updated_paths.insert(entry.file_path.clone());

        let keywords_json = entry.keywords.as_ref().and_then(|kws| {
            if kws.is_empty() {
                None
            } else {
                serde_json::to_string(kws).ok()
            }
        });

        let _ = tx.execute(
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
                metadata_state   = 'done',
                metadata_error   = NULL,
                metadata_synced  = 1,
                updated_at       = datetime('now')
             WHERE file_path = ?1",
            params![
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
    }

    // Anything that was pending but exiftool didn't return a row for
    // (corrupt file, unsupported format, name-mismatched output) gets
    // marked failed so the worker doesn't loop on it forever.
    for path in pending.difference(&updated_paths) {
        let _ = tx.execute(
            "UPDATE images SET
                metadata_state = 'failed',
                metadata_error = 'exiftool returned no data for this file'
             WHERE file_path = ?1",
            params![path],
        );
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn mark_dir_metadata_failed(
    state: &State<AppState>,
    directory: &str,
    error: &str,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE images
         SET metadata_state = 'failed', metadata_error = ?2
         WHERE metadata_state = 'pending'
           AND source_directory_id = (SELECT id FROM source_directories WHERE path = ?1)",
        params![directory, error],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Thumbnail pass ────────────────────────────────────────────────────────

fn run_thumbnail_batch(app: &AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();

    let pending: Vec<(i64, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT id, file_path FROM images
                 WHERE thumbnail_state = 'pending'
                 ORDER BY id ASC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![THUMBNAIL_BATCH_SIZE], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    if pending.is_empty() {
        return Ok(false);
    }

    emit_progress(app, true);

    // Decode + resize the batch in parallel. Each thread processes a
    // round-robin slice of `pending` (so a slow image doesn't stall a
    // single thread for the whole batch). std::thread::scope lets the
    // borrowed closures hold &str references into `pending` without an
    // 'static bound. DB writes go in serially after the join because
    // the connection mutex is single-writer anyway.
    let parallelism = std::thread::available_parallelism()
        .map(|p| p.get().min(THUMBNAIL_PARALLELISM_CAP))
        .unwrap_or(2)
        .max(1);

    type ThumbResult = (i64, std::path::PathBuf, Result<(), String>);
    let results: Vec<ThumbResult> = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(parallelism);
        for offset in 0..parallelism {
            let slice: Vec<&(i64, String)> = pending
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % parallelism == offset)
                .map(|(_, item)| item)
                .collect();
            handles.push(s.spawn(move || {
                slice
                    .into_iter()
                    .map(|(id, file_path)| {
                        let thumb_path = thumbnails::thumbnail_path_for_id(*id);
                        let r = thumbnails::generate_thumbnail_for_file(file_path, &thumb_path);
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

    // Persist results. One transaction so the lock is held briefly.
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
                            params![thumb_path.to_string_lossy().to_string(), id],
                        );
                    }
                    Err(e) => {
                        log::warn!("background_jobs: thumbnail failed for id {}: {}", id, e);
                        let _ = tx.execute(
                            "UPDATE images
                             SET thumbnail_state = 'failed', thumbnail_error = ?1
                             WHERE id = ?2",
                            params![e, id],
                        );
                    }
                }
            }
            let _ = tx.commit();
        }
    }

    emit_progress(app, true);
    Ok(true)
}

// ── Progress event ────────────────────────────────────────────────────────

fn collect_progress(state: &State<AppState>) -> Result<BackgroundProgress, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count = |sql: &str| -> i64 {
        db.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0)
    };
    let (total, resolved, pending) = db
        .query_row(
            "SELECT
                COUNT(*),
                SUM(CASE WHEN thumbnail_state != 'pending'
                          AND metadata_state != 'pending'
                         THEN 1 ELSE 0 END),
                SUM(CASE WHEN thumbnail_state = 'pending'
                           OR metadata_state = 'pending'
                         THEN 1 ELSE 0 END)
             FROM images",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ))
            },
        )
        .unwrap_or((0, 0, 0));
    Ok(BackgroundProgress {
        thumbnails: JobStateCounts {
            pending: count("SELECT COUNT(*) FROM images WHERE thumbnail_state = 'pending'"),
            done: count("SELECT COUNT(*) FROM images WHERE thumbnail_state = 'done'"),
            failed: count("SELECT COUNT(*) FROM images WHERE thumbnail_state = 'failed'"),
        },
        metadata: JobStateCounts {
            pending: count("SELECT COUNT(*) FROM images WHERE metadata_state = 'pending'"),
            done: count("SELECT COUNT(*) FROM images WHERE metadata_state = 'done'"),
            failed: count("SELECT COUNT(*) FROM images WHERE metadata_state = 'failed'"),
        },
        images: ImageProgress {
            total,
            resolved,
            pending,
        },
        busy: false,
    })
}

/// Compute the current progress snapshot and broadcast it on the
/// `background:progress` Tauri event channel. Public so other code paths
/// that resolve work outside the worker (e.g. `generate_full_thumbnails`
/// for visible-priority decode) can keep the footer indicator in sync
/// without waiting for the next worker poll.
pub fn emit_progress(app: &AppHandle, busy: bool) {
    let state = app.state::<AppState>();
    let mut snapshot = match collect_progress(&state) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("background_jobs: collect_progress failed: {}", e);
            return;
        }
    };
    snapshot.busy = busy;
    if let Err(e) = app.emit("background:progress", &snapshot) {
        log::warn!("background_jobs: emit failed: {}", e);
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_background_progress(state: State<AppState>) -> Result<BackgroundProgress, String> {
    auth::require_session(&state)?;
    collect_progress(&state)
}

fn list_failures(state: &State<AppState>, kind: &str, limit: i64) -> Result<Vec<FailureRecord>, String> {
    let (state_col, error_col) = match kind {
        "thumbnails" => ("thumbnail_state", "thumbnail_error"),
        "metadata" => ("metadata_state", "metadata_error"),
        _ => return Err(format!("Unknown failure kind: {}", kind)),
    };
    let sql = format!(
        "SELECT id, catalog_number, file_path, {error_col}
         FROM images
         WHERE {state_col} = 'failed'
         ORDER BY id ASC
         LIMIT ?1"
    );
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(FailureRecord {
                image_id: r.get(0)?,
                catalog_number: r.get(1)?,
                file_path: r.get(2)?,
                error: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn list_thumbnail_failures(
    limit: i64,
    state: State<AppState>,
) -> Result<Vec<FailureRecord>, String> {
    auth::require_session(&state)?;
    list_failures(&state, "thumbnails", limit)
}

#[tauri::command]
pub fn list_metadata_failures(
    limit: i64,
    state: State<AppState>,
) -> Result<Vec<FailureRecord>, String> {
    auth::require_session(&state)?;
    list_failures(&state, "metadata", limit)
}

fn retry_failures(state: &State<AppState>, kind: &str) -> Result<i64, String> {
    let state_col = match kind {
        "thumbnails" => "thumbnail_state",
        "metadata" => "metadata_state",
        _ => return Err(format!("Unknown failure kind: {}", kind)),
    };
    let error_col = match kind {
        "thumbnails" => "thumbnail_error",
        "metadata" => "metadata_error",
        _ => unreachable!(),
    };
    let sql = format!(
        "UPDATE images
         SET {state_col} = 'pending', {error_col} = NULL
         WHERE {state_col} = 'failed'"
    );
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let n = db.execute(&sql, []).map_err(|e| e.to_string())?;
    Ok(n as i64)
}

#[tauri::command]
pub fn retry_failed_thumbnails(state: State<AppState>) -> Result<i64, String> {
    auth::require_session(&state)?;
    retry_failures(&state, "thumbnails")
}

#[tauri::command]
pub fn retry_failed_metadata(state: State<AppState>) -> Result<i64, String> {
    auth::require_session(&state)?;
    retry_failures(&state, "metadata")
}

