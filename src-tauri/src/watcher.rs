//! File-system watcher (Plan 12).
//!
//! Watches every registered source directory for changes and lets the
//! frontend know when something happened so it can re-scan + refresh
//! the sidebar tree. Uses notify-debouncer-mini so we get a single
//! callback per "burst" of file-system events (FSEvents fires many
//! events for a single change on macOS).
//!
//! Strategy:
//! - Spawn one debouncer for the lifetime of the app.
//! - On startup: add_path for every existing source directory.
//! - source_directories::add_source_directory / remove_source_directory
//!   call into add_path / remove_path here so the watch set tracks
//!   user changes.
//! - When the debouncer fires, look up which source(s) the changed
//!   paths belong to, then emit a Tauri event "library:filesystem-changed"
//!   with the affected source IDs. The frontend handles re-scanning
//!   and thumbnail extraction (it already has admin auth).

use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::AppState;

/// Debounce window: collect FS events for this long before firing the
/// callback. macOS FSEvents can fire dozens of events for a single file
/// drop; 2s catches all of them while staying responsive enough.
const DEBOUNCE_WINDOW: Duration = Duration::from_secs(2);

pub struct WatcherHandle {
    inner: Mutex<Debouncer<RecommendedWatcher>>,
}

impl WatcherHandle {
    pub fn add_path(&self, path: &str) -> Result<(), String> {
        let mut g = self.inner.lock().map_err(|e| e.to_string())?;
        g.watcher()
            .watch(Path::new(path), RecursiveMode::Recursive)
            .map_err(|e| format!("watch({}) failed: {}", path, e))
    }

    pub fn remove_path(&self, path: &str) -> Result<(), String> {
        let mut g = self.inner.lock().map_err(|e| e.to_string())?;
        g.watcher()
            .unwatch(Path::new(path))
            .map_err(|e| format!("unwatch({}) failed: {}", path, e))
    }
}

/// Spawn a debouncing watcher and seed it with every currently-registered
/// source directory. Called once during Tauri setup. The returned handle
/// is stored in AppState so per-source CRUD commands can extend the
/// watch set on the fly.
pub fn spawn_watcher(app: AppHandle) -> Result<WatcherHandle, String> {
    let app_for_callback = app.clone();
    let debouncer = new_debouncer(
        DEBOUNCE_WINDOW,
        move |res: notify_debouncer_mini::DebounceEventResult| match res {
            Ok(events) => handle_events(&app_for_callback, events),
            Err(e) => log::warn!("watcher: notify error: {}", e),
        },
    )
    .map_err(|e| format!("Failed to start file watcher: {}", e))?;

    let handle = WatcherHandle {
        inner: Mutex::new(debouncer),
    };

    // Seed the watch set with every existing source directory.
    let paths: Vec<String> = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare("SELECT path FROM source_directories")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    log::info!(
        "watcher: seeding with {} source path(s): {:?}",
        paths.len(),
        paths
    );
    for path in paths {
        match handle.add_path(&path) {
            Ok(()) => log::info!("watcher: watching {}", path),
            Err(e) => log::warn!("watcher: initial add_path failed for {}: {}", path, e),
        }
    }

    Ok(handle)
}

fn handle_events(app: &AppHandle, events: Vec<DebouncedEvent>) {
    if events.is_empty() {
        return;
    }

    // Skip the burst entirely if every event is for a dotfile (.DS_Store
    // is the big offender on macOS — Finder rewrites it constantly).
    // Anything visible to the user falls through.
    let has_real_event = events.iter().any(|e| {
        e.path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| !n.starts_with('.'))
            .unwrap_or(false)
    });
    if !has_real_event {
        log::debug!(
            "watcher: ignoring {} dotfile-only event(s)",
            events.len()
        );
        return;
    }

    let sample = events.first().map(|e| e.path.to_string_lossy().to_string());
    log::info!(
        "watcher: {} debounced event(s); sample path: {:?}",
        events.len(),
        sample
    );

    // List every registered source. We could try to scope the rescan to
    // just the sources whose paths are a prefix of an event path, but on
    // macOS that match is fragile (symlink resolution, /private aliases,
    // case-folding). Re-scanning every source on any event is cheap —
    // scan_directory is INSERT OR IGNORE — and saves a class of bugs.
    let state = app.state::<AppState>();
    let source_ids: Vec<i64> = match state.db.lock() {
        Ok(db) => {
            let Ok(mut stmt) = db.prepare("SELECT id FROM source_directories") else {
                return;
            };
            let Ok(rows) = stmt.query_map([], |r| r.get::<_, i64>(0)) else {
                return;
            };
            rows.filter_map(|r| r.ok()).collect()
        }
        Err(_) => return,
    };
    if source_ids.is_empty() {
        return;
    }

    log::info!(
        "watcher: emitting library:filesystem-changed for sources {:?}",
        source_ids
    );
    if let Err(e) = app.emit("library:filesystem-changed", &source_ids) {
        log::warn!("watcher: failed to emit event: {}", e);
    }
}
