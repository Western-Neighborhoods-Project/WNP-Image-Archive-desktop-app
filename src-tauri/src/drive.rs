// Drive monitoring — Plan 6.
//
// Watches the volume that contains the configured `source_directory` setting,
// emits Tauri events whenever its mount state or stats change, and exposes
// a few small commands for the frontend (read current state, force a retry,
// reveal in Finder).
//
// Architecture:
//
//   ┌─────────────────────────────────────────────────────────┐
//   │ background poller thread (std::thread::spawn)           │
//   │   tick every 1s — cheap mount probe (Path::exists)      │
//   │   every 15 ticks — refresh full stats (free space + DB) │
//   │   on transition or stats refresh — emit drive:status    │
//   └────────────────────────┬────────────────────────────────┘
//                            │ updates
//                            ▼
//                  AppState.drive_state (Mutex)
//                            │ read by
//                            ▼
//          get_drive_status, retry_drive_connection
//
// We keep the cached state in AppState so commands can return the current
// snapshot synchronously without waiting for the next tick. The poller is
// the only long-lived writer; commands either read it or trigger a
// synchronous re-compute.

use crate::db::AppState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

/// 1s × 15 = 15s between full stats refreshes (matches the polling interval
/// the user picked in Plan 6's resolved-decisions table).
const STATS_REFRESH_TICKS: u64 = 15;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot of the archive drive's current state. Camel-cased so it lands
/// in the Svelte side as a natural object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    /// True when `source_directory` exists and is a directory. False at boot
    /// before the first probe and any time the directory is missing.
    pub connected: bool,
    /// The configured `source_directory` setting. None if setup hasn't run.
    pub source_directory: Option<String>,
    /// `/Volumes/<name>` if the source dir is on an external volume; None
    /// if it's on internal storage (in which case we still monitor it but
    /// can't surface a "drive name" separate from the path).
    pub mount_point: Option<String>,
    /// Pretty drive label (basename of mount_point).
    pub label: Option<String>,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    /// When the drive was first detected as mounted in this session. Resets
    /// when the drive is unmounted then reconnected.
    pub mounted_at_ms: Option<i64>,
    /// When the stats fields above were last refreshed. The poller refreshes
    /// every 15s when connected.
    pub last_stats_at_ms: Option<i64>,
    pub image_count: Option<i64>,
    /// e.g. `{"jpg": 12000, "tiff": 5000, "(unknown)": 3}` — useful for the
    /// indicator popover's format-mix breakdown.
    pub format_mix: HashMap<String, i64>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Walk the path looking for `/Volumes/<name>` — that's our mount point.
/// Returns None for paths not on an external volume (which the caller
/// treats as "internal storage, no separate mount").
fn derive_mount_point(path: &Path) -> Option<PathBuf> {
    let mut iter = path.components();
    iter.next()?; // skip root component (`/`)
    let volumes_component = iter.next()?;
    if let std::path::Component::Normal(s) = volumes_component {
        if s == "Volumes" {
            if let Some(std::path::Component::Normal(name)) = iter.next() {
                let mut p = PathBuf::from("/Volumes");
                p.push(name);
                return Some(p);
            }
        }
    }
    None
}

fn read_source_directory(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'source_directory'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|s| !s.is_empty())
}

/// Returns (image_count, format_mix). Both are best-effort — if the query
/// errors, we degrade gracefully.
fn query_image_stats(conn: &Connection) -> (Option<i64>, HashMap<String, i64>) {
    let count: Option<i64> = conn
        .query_row("SELECT COUNT(*) FROM images", [], |r| r.get(0))
        .ok();

    let mut mix: HashMap<String, i64> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT COALESCE(format, '(unknown)') as fmt, COUNT(*) FROM images GROUP BY fmt",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))) {
            for row in rows.flatten() {
                mix.insert(row.0, row.1);
            }
        }
    }

    (count, mix)
}

/// Compute a fresh DriveStatus. Pure-ish — reads DB and filesystem but
/// doesn't mutate any shared state. The caller is responsible for storing
/// the result back into AppState.drive_state and emitting the event.
///
/// `previous` lets us preserve mounted_at_ms and reuse stats when not
/// refreshing (which is most ticks).
fn compute_drive_state(
    state: &AppState,
    previous: Option<&DriveStatus>,
    refresh_stats: bool,
) -> DriveStatus {
    let Ok(db) = state.db.lock() else {
        // Mutex poisoned — return last good state if we have one.
        return previous.cloned().unwrap_or_default();
    };

    let source_directory = read_source_directory(&db);
    let Some(src) = source_directory else {
        return DriveStatus::default();
    };

    let path = PathBuf::from(&src);
    let connected = path.exists() && path.is_dir();

    if !connected {
        return DriveStatus {
            connected: false,
            source_directory: Some(src),
            ..Default::default()
        };
    }

    let mount_point = derive_mount_point(&path);
    let label = mount_point
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());

    // Was the drive connected at the same path on the previous tick?
    // If so, preserve mounted_at_ms (and reuse stats when not refreshing).
    let same_session = previous
        .map(|p| p.connected && p.source_directory.as_deref() == Some(src.as_str()))
        .unwrap_or(false);

    let mounted_at_ms = if same_session {
        previous.and_then(|p| p.mounted_at_ms)
    } else {
        Some(now_ms())
    };

    let must_refresh = refresh_stats || !same_session;
    let (total_bytes, available_bytes, image_count, format_mix, last_stats_at_ms) = if must_refresh
    {
        let total = fs4::total_space(&path).ok();
        let avail = fs4::available_space(&path).ok();
        let (count, mix) = query_image_stats(&db);
        (total, avail, count, mix, Some(now_ms()))
    } else {
        // Carry forward — same_session is true, so previous is Some.
        let p = previous.expect("same_session implies previous is Some");
        (
            p.total_bytes,
            p.available_bytes,
            p.image_count,
            p.format_mix.clone(),
            p.last_stats_at_ms,
        )
    };

    DriveStatus {
        connected: true,
        source_directory: Some(src),
        mount_point: mount_point.map(|p| p.to_string_lossy().to_string()),
        label,
        total_bytes,
        available_bytes,
        mounted_at_ms,
        last_stats_at_ms,
        image_count,
        format_mix,
    }
}

/// Emits the new state into AppState + on the Tauri event bus when it
/// differs from the previous tick (or when stats just refreshed).
fn maybe_emit(
    app: &AppHandle,
    state: &AppState,
    previous: &DriveStatus,
    new_state: DriveStatus,
    stats_refreshed: bool,
    initial: bool,
) {
    let connected_changed = previous.connected != new_state.connected
        || previous.source_directory != new_state.source_directory;

    if !(connected_changed || stats_refreshed || initial) {
        return;
    }

    if let Ok(mut g) = state.drive_state.lock() {
        *g = new_state.clone();
    }
    let _ = app.emit("drive:status", &new_state);
}

/// Spawn the background poller. Called once during Tauri setup.
///
/// Computes the initial state synchronously before returning so any
/// command handler that fires before the first thread tick (e.g. the
/// frontend's `getDriveStatus` on boot) sees an up-to-date snapshot.
/// Then a background thread takes over for continuous polling.
pub fn spawn_drive_poller(app: AppHandle) {
    // Synchronous initial probe — populates AppState.drive_state and
    // emits the first event so the frontend can hydrate immediately.
    {
        let state = app.state::<AppState>();
        let previous = state
            .drive_state
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        let new_state = compute_drive_state(&state, Some(&previous), true);
        let initial = true;
        let stats_refreshed = new_state.connected;
        maybe_emit(&app, &state, &previous, new_state, stats_refreshed, initial);
    }

    // Background loop continues from tick=1 (tick=0 was the synchronous
    // probe above).
    std::thread::spawn(move || {
        let mut tick: u64 = 1;
        loop {
            std::thread::sleep(POLL_INTERVAL);

            let state = app.state::<AppState>();
            let previous = state
                .drive_state
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            let refresh_stats = tick % STATS_REFRESH_TICKS == 0;
            let new_state = compute_drive_state(&state, Some(&previous), refresh_stats);

            let stats_refreshed = refresh_stats && new_state.connected;
            maybe_emit(&app, &state, &previous, new_state, stats_refreshed, false);

            tick = tick.wrapping_add(1);
        }
    });
}

// ── Commands ───────────────────────────────────────────────────────────────

/// Returns the cached snapshot. Cheap; doesn't probe disk.
#[tauri::command]
pub fn get_drive_status(state: State<AppState>) -> DriveStatus {
    state
        .drive_state
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Forces an immediate re-probe + stats refresh. Used by the "Retry"
/// button on the disconnected screen.
#[tauri::command]
pub fn retry_drive_connection(app: AppHandle, state: State<AppState>) -> DriveStatus {
    let previous = state
        .drive_state
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let new_state = compute_drive_state(&state, Some(&previous), true);
    // Always emit on a manual retry so the UI refreshes even if state
    // didn't change (gives the button visible feedback).
    if let Ok(mut g) = state.drive_state.lock() {
        *g = new_state.clone();
    }
    let _ = app.emit("drive:status", &new_state);
    new_state
}

/// Open the drive (or source directory if it's not on a /Volumes mount)
/// in Finder. Used by the indicator popover's "Reveal in Finder" button.
#[tauri::command]
pub fn reveal_drive_in_finder(state: State<AppState>) -> Result<(), String> {
    let status = state
        .drive_state
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let target = status
        .mount_point
        .clone()
        .or(status.source_directory.clone())
        .ok_or_else(|| "No drive path configured".to_string())?;

    std::process::Command::new("open")
        .arg(&target)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}
