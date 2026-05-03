# Plan 7 — Import inbox watcher

> **Status:** ABANDONED 2026-05-02. The OpenSFHistory upload workflow is the source of truth for new images, so a local /_inbox/ watcher is unnecessary. The Import-inbox sidebar entry, view types, chord shortcut, and stub view were removed in the same cleanup. Spec is preserved below as reference in case the workflow ever changes.
>
> **Position in roadmap:** Plan 7 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plans 1 + 6. Uses Plan 6's drive monitor to start/stop the watcher.

## Goal

Continuous file-system watch on `<drive>/_inbox/`. Files dropped onto the drive (outside the app) auto-import as new images grouped into per-batch sets, with per-image "awaiting metadata" status until OpenSFHistory sync (Plan 9) or a manual edit clears the flag.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Batch boundary | **Time gap, 5 min idle.** New file with no preceding file in the last 5 min opens a new batch. Threshold is configurable via Settings (key: `inbox_batch_gap_seconds`, default 300). |
| _inbox location | **Drive root.** `<mount_point>/_inbox` if `source_directory` is on `/Volumes`; falls back to `<source_directory>/_inbox` for internal storage. Auto-created on watcher start if missing. |
| Setup flow vs watcher | **Unchanged.** First-time setup still does one-shot bulk import; those images get `import_batch_id = NULL`. Watcher only handles post-setup files. Existing production rows remain batchless, which is correct. |
| Batch UI scope | **Read-only + per-image edit.** Inbox view shows batches with their images and stubbed "Open in OpenSFHistory ↗" link. Editing a field in Detail view clears that image's `awaiting_metadata` flag. |

## Scope

**Backend:**
- New `import_batches` table; new `import_batch_id` and `awaiting_metadata` columns on `images`. Schema migration via `pragma_table_info` check + `ALTER TABLE ADD COLUMN`.
- New `watcher.rs` module: notify-based recursive watcher on `<drive>/_inbox/`, debounced 2s. Start/stop driven by `driveStatus` (subscribes to mount/unmount transitions).
- New `import_inbox.rs` module: single-file import path (extract EXIF, generate thumbnail, INSERT with `awaiting_metadata=1`) + batch assignment (extend latest open batch if last file <5 min ago, else open new).
- New commands: `get_inbox_batches`, `get_inbox_batch_images`.
- `update_image_metadata` (in `editor.rs`) clears `awaiting_metadata` whenever any field is edited.

**Frontend:**
- Replace `ImportInboxStub` with `ImportInboxView`: PageHeader + two-pane layout (320px BatchList + flexible BatchDetail).
- `BatchList.svelte` — vertical list, latest batch at top, sticky "Today / Yesterday / Older" date headers.
- `BatchDetail.svelte` — header (batch label, file count, "Open in OpenSFHistory ↗" stub) + Lightroom-style image grid.
- `BatchImageCard.svelte` — thumbnail + catalog # + amber chip when awaiting metadata.
- `inboxBatches` store — fed by Tauri events on every new batch / new image.
- Sidebar "Import inbox" entry: show pending-count badge when there are awaiting-metadata images.

## Out of scope

- Plan 9 (OpenSFHistory sync). The "Open in OpenSFHistory" link renders as a disabled stub showing the URL it would open.
- Bulk batch actions ("Mark batch as reviewed", "Delete batch", etc.). Read-only inbox in v1.
- File deletion handling — if files are removed from `_inbox/` outside the app, those rows persist. Out of scope.
- Manual reimport / catch-up of files dropped while watcher was offline — files added during disconnect get picked up on next reconnect via a one-time directory scan.

## Architecture

```
                   /Volumes/<drive>/_inbox/   (filesystem)
                            │
                            ▼ notify CREATE/MODIFY events (debounced 2s)
                  ┌────────────────────┐
                  │   watcher.rs       │
                  │ ─ subscribes to    │
                  │   drive:status     │
                  │ ─ start on mount   │
                  │ ─ stop on unmount  │
                  └─────────┬──────────┘
                            │ for each new file
                            ▼
                  ┌────────────────────┐
                  │  import_inbox.rs   │
                  │ ─ extract EXIF     │
                  │ ─ gen thumbnail    │
                  │ ─ assign to batch  │
                  │ ─ INSERT image     │
                  │ ─ emit inbox:*     │
                  └─────────┬──────────┘
                            │ Tauri events
                            ▼
                ┌──────────────────────┐
                │ inboxBatches store   │
                └─────────┬────────────┘
                          ▼
              ImportInboxView (BatchList + BatchDetail)
```

## Key files

**New (Rust):**
- `src-tauri/src/watcher.rs`
- `src-tauri/src/import_inbox.rs`

**New (Svelte):**
- `src/lib/components/inbox/ImportInboxView.svelte`
- `src/lib/components/inbox/BatchList.svelte`
- `src/lib/components/inbox/BatchDetail.svelte`
- `src/lib/components/inbox/BatchImageCard.svelte`
- `src/lib/commands/inbox.ts`
- `src/lib/stores/inboxBatches.ts`

**Modify:**
- `src-tauri/Cargo.toml` (add `notify`, `notify-debouncer-mini`)
- `src-tauri/sql/schema.sql` (add `import_batches` table; new columns on images)
- `src-tauri/src/db.rs` (`run_migrations`: add `add_column_if_missing` helper + Migration 002 calls)
- `src-tauri/src/lib.rs` (register watcher, register inbox commands)
- `src-tauri/src/editor.rs` (clear `awaiting_metadata` on update)
- `src-tauri/src/scanner.rs` (extract per-file logic if needed by import_inbox)
- `src/routes/+page.svelte` (replace `ImportInboxStub` with real view)
- `src/lib/components/layout/Sidebar.svelte` (pending-count badge on Import inbox item)

## Tasks

Tasks are grouped into 8 phases. Each task names files touched and a verification gate. Run `cargo check` after every Rust task and `bun run check` after every frontend task; commit at the end of each phase to keep the diff bisectable.

---

### Phase A — Schema + dependencies

**A1. Update `src-tauri/sql/schema.sql`**

Add to the `images` CREATE TABLE (immediately before the closing paren):
```sql
-- Plan 7: import inbox watcher
import_batch_id     INTEGER,           -- FK to import_batches.id; NULL for setup-imported rows
awaiting_metadata   INTEGER DEFAULT 0, -- 1 = imported via inbox watcher, no edits yet; cleared on first edit
```
Add two indexes immediately after the existing `idx_images_*` block:
```sql
CREATE INDEX IF NOT EXISTS idx_images_import_batch_id   ON images(import_batch_id);
CREATE INDEX IF NOT EXISTS idx_images_awaiting_metadata ON images(awaiting_metadata);
```
Add a new `import_batches` table just before the `app_settings` section:
```sql
CREATE TABLE IF NOT EXISTS import_batches (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    label        TEXT,                          -- e.g. "Apr 27 — 14 photos"
    source_path  TEXT,                          -- absolute path to _inbox at the time
    created_at   TEXT DEFAULT (datetime('now')),
    closed_at    TEXT,                          -- when the gap-timeout closed the batch
    last_file_at TEXT,                          -- timestamp of most recent file in batch
    total_files  INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_import_batches_created_at ON import_batches(created_at);
```

**A2. Add migration helpers to `src-tauri/src/db.rs`**

Append to `run_migrations` (after the FTS5 trigram block):
```rust
// Migration 002 (Plan 7): inbox watcher columns on images.
add_column_if_missing(conn, "images", "import_batch_id", "INTEGER")?;
add_column_if_missing(conn, "images", "awaiting_metadata", "INTEGER DEFAULT 0")?;
```
Add helper at the end of the file:
```rust
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    type_decl: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!(
        "SELECT 1 FROM pragma_table_info('{}') WHERE name = ?1 LIMIT 1",
        table
    ))?;
    let exists = stmt.query_row(rusqlite::params![column], |_| Ok(())).is_ok();
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column, type_decl
        ))?;
    }
    Ok(())
}
```

**A3. Add `notify` dep to `src-tauri/Cargo.toml`**

```toml
notify = "6"
```
No debouncer crate needed — DIY size-stability check (see Phase D) is simpler than another dep.

**A4. Refactor `src-tauri/src/thumbnails.rs`**

Expose a new helper that the watcher can call without the DB-roundtrip the existing `generate_thumbnail_single` does:
```rust
pub fn generate_thumbnail_for_image(
    state: &tauri::State<AppState>,
    image_id: i64,
    file_path: &str,
) -> Result<PathBuf, String> {
    let thumb_path = thumbnail_path_for_id(image_id);
    generate_thumbnail_for_file(file_path, &thumb_path)?;
    update_thumbnail_db(state, image_id, &thumb_path, true)?;
    Ok(thumb_path)
}
```
Place it immediately after the `Shared Helpers` comment block (above `generate_thumbnail_for_file`).

**Verification (Phase A):** `cargo check` clean. Database file at `~/Library/Application Support/org.wnp.imagearchive/archive_manager.db` will pick up the new columns + table on next launch.

---

### Phase B — Backend models

**B1. Add types to `src-tauri/src/models.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub id: i64,
    pub label: Option<String>,
    pub source_path: Option<String>,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub last_file_at: Option<String>,
    pub total_files: i64,
    pub awaiting_count: i64,  // images in this batch where awaiting_metadata = 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageSummary {
    pub id: i64,
    pub catalog_number: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub awaiting_metadata: bool,
    pub title: Option<String>,
    pub city: Option<String>,
    pub date_display: Option<String>,
}
```

---

### Phase C — Inbox import logic

**C1. Create `src-tauri/src/import_inbox.rs`**

Module organization:

```rust
use crate::db::AppState;
use crate::models::{BatchImageSummary, BatchSummary};
use crate::{metadata, thumbnails};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};

/// Default batch gap. Configurable via app_settings key 'inbox_batch_gap_seconds'.
const DEFAULT_BATCH_GAP_SECONDS: i64 = 300;

/// Resolve where _inbox lives. Reads source_directory and the cached drive_state
/// from AppState. Prefers `<mount_point>/_inbox` for /Volumes paths, falls back
/// to `<source_directory>/_inbox` for internal storage.
pub fn derive_inbox_path(state: &AppState) -> Option<PathBuf> {
    // Try drive's mount_point first
    if let Ok(g) = state.drive_state.lock() {
        if g.connected {
            if let Some(mp) = &g.mount_point {
                return Some(PathBuf::from(mp).join("_inbox"));
            }
            if let Some(src) = &g.source_directory {
                return Some(PathBuf::from(src).join("_inbox"));
            }
        }
    }
    // Drive not connected — read source_directory directly
    let conn = state.db.lock().ok()?;
    let src: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'source_directory'",
            [],
            |r| r.get(0),
        )
        .ok()?;
    Some(PathBuf::from(src).join("_inbox"))
}

/// Read the batch-gap setting (or default 300s).
fn read_batch_gap_seconds(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'inbox_batch_gap_seconds'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(DEFAULT_BATCH_GAP_SECONDS)
}

/// Find an open batch (last_file_at within gap window) or create a new one.
/// Returns batch_id.
fn find_or_create_open_batch(
    conn: &Connection,
    inbox_path: &Path,
) -> rusqlite::Result<i64> {
    let gap = read_batch_gap_seconds(conn);

    // SQL: last batch whose last_file_at is within `gap` seconds of now,
    // and which hasn't been closed.
    let row: Option<i64> = conn
        .query_row(
            "SELECT id FROM import_batches
             WHERE closed_at IS NULL
               AND (strftime('%s','now') - strftime('%s', last_file_at)) < ?1
             ORDER BY id DESC LIMIT 1",
            params![gap],
            |r| r.get(0),
        )
        .ok();

    if let Some(id) = row {
        return Ok(id);
    }

    // Close any stale open batches (last_file_at older than gap)
    conn.execute(
        "UPDATE import_batches
            SET closed_at = datetime('now')
          WHERE closed_at IS NULL
            AND (strftime('%s','now') - strftime('%s', last_file_at)) >= ?1",
        params![gap],
    )?;

    // Open a new batch. Label is derived later when files are added.
    conn.execute(
        "INSERT INTO import_batches (source_path, last_file_at)
         VALUES (?1, datetime('now'))",
        params![inbox_path.to_string_lossy().to_string()],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update a batch's label + last_file_at + total_files counter.
/// Label is "MMM DD — N photos" using the batch's created_at date.
fn refresh_batch_metadata(conn: &Connection, batch_id: i64) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM images WHERE import_batch_id = ?1",
        params![batch_id],
        |r| r.get(0),
    )?;
    // SQLite has limited date formatting — use strftime with %b which is locale-
    // dependent. For clean output, format in Rust later if needed.
    let label = format!("Batch #{} — {} photos", batch_id, count);
    conn.execute(
        "UPDATE import_batches
            SET label = ?1,
                total_files = ?2,
                last_file_at = datetime('now')
          WHERE id = ?3",
        params![label, count, batch_id],
    )?;
    Ok(())
}

/// Insert one new image row with awaiting_metadata=1. Returns new image id.
fn insert_image_row(
    conn: &Connection,
    file_path: &Path,
    batch_id: i64,
) -> rusqlite::Result<i64> {
    let path_str = file_path.to_string_lossy().to_string();
    let catalog_number = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let meta = std::fs::metadata(file_path).ok();
    let file_size = meta.as_ref().map(|m| m.len() as i64);

    conn.execute(
        "INSERT OR IGNORE INTO images
            (file_path, catalog_number, file_size, import_batch_id, awaiting_metadata)
         VALUES (?1, ?2, ?3, ?4, 1)",
        params![path_str, catalog_number, file_size, batch_id],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Top-level entry point called by the watcher for each new file.
/// Best-effort: errors are logged but don't crash the watcher loop.
pub fn process_inbox_file(app: &AppHandle, file_path: &Path) {
    let state = app.state::<AppState>();

    // 1. Insert row + assign batch (atomic in one DB lock)
    let (image_id, batch_id) = {
        let Ok(conn) = state.db.lock() else { return };

        // Must derive inbox_path before insert in case batch needs creation
        let inbox_path = file_path.parent().unwrap_or(Path::new("/"));
        let batch_id = match find_or_create_open_batch(&conn, inbox_path) {
            Ok(id) => id,
            Err(e) => { eprintln!("batch assignment failed: {}", e); return }
        };
        let image_id = match insert_image_row(&conn, file_path, batch_id) {
            Ok(id) => id,
            Err(e) => { eprintln!("image insert failed: {}", e); return }
        };
        if let Err(e) = refresh_batch_metadata(&conn, batch_id) {
            eprintln!("batch metadata update failed: {}", e);
        }
        (image_id, batch_id)
    };

    // 2. Generate thumbnail (releases the DB lock between insert and thumb)
    let path_str = file_path.to_string_lossy().to_string();
    let _ = thumbnails::generate_thumbnail_for_image(&state, image_id, &path_str);

    // 3. Run exiftool to populate basic metadata fields (best-effort)
    if let Ok(meta) = metadata::extract_metadata_single(path_str.clone()) {
        // Apply the parsed metadata via existing editor logic. Note:
        // editor::update_image_metadata clears awaiting_metadata as a side-
        // effect, which we don't want for inbox imports — so write directly
        // to the DB here, NOT via the editor command.
        if let Ok(conn) = state.db.lock() {
            // Map ExtractedMetadata fields to DB columns. Field names
            // depend on what extract_metadata_single returns. See
            // metadata.rs::ExtractedMetadata for the actual struct.
            let _ = conn.execute(
                "UPDATE images SET
                    title = COALESCE(?1, title),
                    description = COALESCE(?2, description),
                    photographer = COALESCE(?3, photographer),
                    date_display = COALESCE(?4, date_display)
                 WHERE id = ?5",
                params![
                    meta.title,
                    meta.description,
                    meta.photographer,
                    meta.date_display,
                    image_id
                ],
            );
        }
    }

    // 4. Emit Tauri event so frontend refreshes
    let _ = app.emit(
        "inbox:updated",
        serde_json::json!({ "batchId": batch_id, "imageId": image_id }),
    );
}

// ── Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_inbox_batches(state: State<AppState>) -> Result<Vec<BatchSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT
            b.id, b.label, b.source_path, b.created_at, b.closed_at,
            b.last_file_at, b.total_files,
            (SELECT COUNT(*) FROM images i
              WHERE i.import_batch_id = b.id AND i.awaiting_metadata = 1) AS awaiting_count
         FROM import_batches b
         ORDER BY b.created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        Ok(BatchSummary {
            id: r.get(0)?,
            label: r.get(1)?,
            source_path: r.get(2)?,
            created_at: r.get(3)?,
            closed_at: r.get(4)?,
            last_file_at: r.get(5)?,
            total_files: r.get(6)?,
            awaiting_count: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn get_inbox_batch_images(
    batch_id: i64,
    state: State<AppState>,
) -> Result<Vec<BatchImageSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, catalog_number, file_path, thumbnail_path,
                awaiting_metadata, title, city, date_display
           FROM images
          WHERE import_batch_id = ?1
          ORDER BY id ASC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![batch_id], |r| {
        Ok(BatchImageSummary {
            id: r.get(0)?,
            catalog_number: r.get(1)?,
            file_path: r.get(2)?,
            thumbnail_path: r.get(3)?,
            awaiting_metadata: r.get::<_, i64>(4)? == 1,
            title: r.get(5)?,
            city: r.get(6)?,
            date_display: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
```

**Notes on tradeoffs:**
- `process_inbox_file` writes metadata directly to the DB (not via `editor::update_image_metadata`) because the editor clears the `awaiting_metadata` flag as a side-effect — and we want EXIF auto-fill to NOT count as "user has reviewed this." Only manual edits in Detail view should clear the flag.
- Batch label format `"Batch #N — M photos"` is plain. Could be improved later to `"Apr 27 — 14 photos"` (requires Rust-side date formatting since SQLite locale handling is unreliable).

---

### Phase D — Watcher

**D1. Create `src-tauri/src/watcher.rs`**

```rust
// Inbox file-system watcher (Plan 7).
//
// Lifecycle: a single long-lived thread handles both watching and processing.
// On each iteration:
//   1. Derive the inbox path (depends on driveStatus + source_directory)
//   2. If unreachable, sleep 5s and retry — drive may be unmounted
//   3. Otherwise: catch-up scan (one-shot directory walk for files that
//      slipped in while we were idle), then start a notify watcher
//   4. Drain notify events, calling import_inbox::process_inbox_file
//      after a stability check (file size unchanged for 1s)
//   5. Loop checks every second whether drive is still connected; on
//      disconnect, drops the watcher and goes back to step 1

use crate::db::AppState;
use crate::import_inbox;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const SUPPORTED_EXTS: &[&str] = &["jpg", "jpeg", "tif", "tiff", "png", "gif", "bmp", "webp"];

fn is_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Wait until the file size has been stable for `quiet_for`.
/// Returns false if the file disappears or stays unstable for too long.
fn wait_for_stable_file(path: &Path, quiet_for: Duration, max_wait: Duration) -> bool {
    let start = Instant::now();
    let mut last_size: Option<u64> = None;
    let mut last_change = Instant::now();
    while start.elapsed() < max_wait {
        std::thread::sleep(Duration::from_millis(200));
        match std::fs::metadata(path) {
            Ok(m) => {
                let size = m.len();
                if Some(size) != last_size {
                    last_size = Some(size);
                    last_change = Instant::now();
                } else if last_change.elapsed() >= quiet_for {
                    return true;
                }
            }
            Err(_) => return false,
        }
    }
    false
}

/// One-shot scan of the inbox dir. Used on (re)connect to catch any files
/// that arrived while the watcher was offline.
fn catch_up_scan(app: &AppHandle, inbox: &Path) {
    let Ok(entries) = std::fs::read_dir(inbox) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !is_image(&path) {
            continue;
        }
        // Check if already known to DB before processing
        let state = app.state::<AppState>();
        let already_imported: bool = state
            .db
            .lock()
            .ok()
            .and_then(|c| {
                c.query_row(
                    "SELECT 1 FROM images WHERE file_path = ?1 LIMIT 1",
                    rusqlite::params![path.to_string_lossy().to_string()],
                    |_| Ok(()),
                )
                .ok()
            })
            .is_some();
        if already_imported {
            continue;
        }
        if wait_for_stable_file(&path, Duration::from_secs(1), Duration::from_secs(10)) {
            import_inbox::process_inbox_file(app, &path);
        }
    }
}

fn drive_connected(state: &tauri::State<AppState>) -> bool {
    state.drive_state.lock().map(|g| g.connected).unwrap_or(false)
}

pub fn spawn_inbox_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        // 1. Resolve inbox path (depends on drive state)
        let state = app.state::<AppState>();
        let inbox_path = match import_inbox::derive_inbox_path(&state) {
            Some(p) if p.exists() => p,
            Some(p) => {
                // Auto-create _inbox if drive is mounted but folder missing
                if drive_connected(&state) {
                    let _ = std::fs::create_dir_all(&p);
                    if !p.exists() {
                        std::thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                    p
                } else {
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
            }
            None => {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        // 2. Catch up on files added while we were offline
        catch_up_scan(&app, &inbox_path);

        // 3. Start notify watcher
        let (tx, rx) = mpsc::channel();
        let mut watcher = match recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("failed to create watcher: {}", e);
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        if let Err(e) = watcher.watch(&inbox_path, RecursiveMode::NonRecursive) {
            eprintln!("watch failed for {}: {}", inbox_path.display(), e);
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }

        // 4. Process events until drive unmounts
        loop {
            // Periodic drive-state check
            if !drive_connected(&app.state::<AppState>()) {
                break;
            }
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        for path in event.paths {
                            if !path.is_file() || !is_image(&path) {
                                continue;
                            }
                            if wait_for_stable_file(&path, Duration::from_secs(1), Duration::from_secs(30)) {
                                import_inbox::process_inbox_file(&app, &path);
                            }
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("watch error: {}", e),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        // watcher dropped here — fs handle released
    });
}
```

**D2. Wire watcher into `lib.rs`**

In `setup()`, after `drive::spawn_drive_poller`:
```rust
watcher::spawn_inbox_watcher(app.handle().clone());
```
Add `pub mod import_inbox;` and `pub mod watcher;` to the module list.

Register the new commands in `invoke_handler`:
```rust
// Import inbox (Plan 7)
import_inbox::get_inbox_batches,
import_inbox::get_inbox_batch_images,
```

**Verification (Phases C–D):** `cargo check` clean. Manual test: drop a JPEG into `<drive>/_inbox/`, observe new row in `images` with `import_batch_id` set + matching row in `import_batches`. (DB inspection via `sqlite3 archive_manager.db` works since WAL.)

---

### Phase E — Editor side-effect

**E1. Modify `src-tauri/src/editor.rs::update_image_metadata`**

Find the UPDATE statement that writes the field; immediately after a successful update, add:
```rust
// Plan 7: clear awaiting_metadata when the user touches any field.
// This is what graduates an inbox-imported image from "needs review"
// to "reviewed" status.
db.execute(
    "UPDATE images SET awaiting_metadata = 0 WHERE id = ?1",
    rusqlite::params![image_id],
)?;
```
(Idempotent: setting already-0 to 0 is a no-op; no need to gate with a SELECT.)

**Verification:** Manual — edit a field on an inbox-imported image, confirm `awaiting_metadata = 0` in DB.

---

### Phase F — Frontend foundation

**F1. Create `src/lib/commands/inbox.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';

export interface BatchSummary {
  id: number;
  label: string | null;
  sourcePath: string | null;
  createdAt: string;
  closedAt: string | null;
  lastFileAt: string | null;
  totalFiles: number;
  awaitingCount: number;
}

export interface BatchImageSummary {
  id: number;
  catalogNumber: string;
  filePath: string;
  thumbnailPath: string | null;
  awaitingMetadata: boolean;
  title: string | null;
  city: string | null;
  dateDisplay: string | null;
}

export async function getInboxBatches(): Promise<BatchSummary[]> {
  return invoke('get_inbox_batches');
}

export async function getInboxBatchImages(batchId: number): Promise<BatchImageSummary[]> {
  return invoke('get_inbox_batch_images', { batchId });
}
```

**F2. Create `src/lib/stores/inboxBatches.ts`**

```ts
import { writable, derived, type Readable } from 'svelte/store';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getInboxBatches, type BatchSummary } from '$lib/commands/inbox';

export const inboxBatches = writable<BatchSummary[]>([]);
export const inboxBatchesReady = writable<boolean>(false);

/** Total awaiting-metadata count across all batches. Used by Sidebar badge. */
export const inboxAwaitingCount: Readable<number> = derived(
  inboxBatches,
  ($batches) => $batches.reduce((sum, b) => sum + b.awaitingCount, 0),
);

async function refresh() {
  try {
    const batches = await getInboxBatches();
    inboxBatches.set(batches);
    inboxBatchesReady.set(true);
  } catch (e) {
    console.error('inboxBatches refresh failed', e);
    inboxBatchesReady.set(true);
  }
}

/** Initialize. Call once from +page.svelte onMount. Returns unsubscribe fn. */
export async function initInboxBatchesListener(): Promise<() => void> {
  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen('inbox:updated', () => {
      refresh();
    });
  } catch (e) {
    console.error('inbox:updated listen failed', e);
  }
  await refresh();
  return () => { if (unlisten) unlisten(); };
}
```

---

### Phase G — UI components

**G1. `src/lib/components/inbox/ImportInboxView.svelte`**

Two-pane layout matching design artboard 6:
- PageHeader with title "Import inbox" + subtitle "Files dropped into _inbox auto-import here"
- Body: `flex flex-1 min-h-0` with:
  - 320px-wide BatchList on left
  - flex-1 BatchDetail on right (rendering the currently selected batch, or an empty state)
- StatusBar at bottom with DriveIndicator (matches other views)

Selected batch state can live as `let selectedBatchId = $state<number | null>(null)` initialized to `$inboxBatches[0]?.id ?? null` in an `$effect` block.

**G2. `src/lib/components/inbox/BatchList.svelte`**

Props: `batches: BatchSummary[]`, `selectedBatchId: number | null`, `onSelect: (id: number) => void`

Layout: vertical scrolling list. Each row shows:
- Top line: batch label
- Bottom line: file count + awaiting-count chip (amber, only if > 0)
- Footer-line: relative time ("2h ago", "Yesterday", "Apr 27")

Sticky date-group headers ("Today", "Yesterday", "Apr 27") between rows (use `position: sticky; top: 0`).

Selected row: bg-secondary + left border accent.

**G3. `src/lib/components/inbox/BatchDetail.svelte`**

Props: `batchId: number | null`

Loads images via `getInboxBatchImages(batchId)` in `$effect`. Layout:
- Header bar: batch label, "X photos · Y awaiting metadata", "Open in OpenSFHistory ↗" stub button (disabled, with tooltip "Coming in Plan 9")
- Body: justified-rows grid (reuse existing pattern from LibraryView Grid.svelte, or simple grid for v1)
- Empty state if `batchId === null`: "Select a batch to view its images"

**G4. `src/lib/components/inbox/BatchImageCard.svelte`**

Props: `image: BatchImageSummary`, `onClick: () => void`

Renders a thumbnail (`convertFileSrc(thumbnailPath)`) with:
- Catalog number overlay (bottom-left)
- Amber chip (top-right) when `awaitingMetadata === true`: "Awaiting"

Click navigates to Detail view: set `currentImageId.set(image.id); currentView.set('detail')`.

---

### Phase H — Wire-up

**H1. `src/routes/+page.svelte`**

Replace `import ImportInboxStub from '$lib/components/stubs/ImportInboxStub.svelte';` with the real view import. Replace the `{:else if $currentView === 'inbox'}` branch.

Add to onMount alongside the drive listener:
```ts
let uninstallInboxListener: (() => void) | null = null;
onMount(async () => {
  uninstallInboxListener = await initInboxBatchesListener();
});
onDestroy(() => {
  uninstallInboxListener?.();
});
```

**H2. `src/lib/components/layout/Sidebar.svelte`**

Find the "Import inbox" sidebar item. Add a small badge to the right when `$inboxAwaitingCount > 0`:
```svelte
{#if $inboxAwaitingCount > 0}
  <span class="ml-auto text-[10px] bg-warning/20 text-warning rounded-full px-1.5 py-0.5 tabular-nums">
    {$inboxAwaitingCount}
  </span>
{/if}
```

---

### Phase I — Verification + commit

1. `cargo check` clean
2. `bun run check` clean (0 errors / 0 warnings)
3. **Manual smoke test (requires drive connected):**
   - With app open, drop a JPEG into `<drive>/_inbox/`. Within ~3s, navigate to Import inbox via sidebar — see one batch with the file
   - Drop 3 more files quickly — all in same batch, batch label updates
   - Wait 5+ min, drop another — new batch
   - Click an inbox image, edit Title in Detail view — return to inbox, "Awaiting" chip is gone
   - Eject drive — observe watcher stops cleanly (no spam logs)
   - Reconnect drive — drop another file before opening app, then open: catch-up scan picks it up
4. **Sidebar badge:** total awaiting count visible next to "Import inbox" item
5. Commit message: `Plan 7: Import inbox watcher (notify-based + per-batch grouping)`

---

## Risks + open considerations deferred

1. **`notify` v6 cross-platform behavior on macOS APFS.** notify uses FSEvents on macOS; it can miss events under load and has known quirks with renamed files. Mitigation: catch-up scan on each (re)start of the watcher loop catches anything that slipped through.

2. **Race: file dropped during initial bulk import.** If user is mid-setup and a file lands in `_inbox/`, scanner.rs's recursive walk will index it but with no `import_batch_id`. The watcher then might also import it via catch-up scan, hitting the `INSERT OR IGNORE` (deduped on `file_path`), so no double-row. But the scanner-imported row won't have `awaiting_metadata=1`. Acceptable for v1 — bulk import is a one-time action.

3. **Batch labels are bland.** `"Batch #5 — 12 photos"` is informational but not friendly. Improvement: format with Rust's `chrono` (would add dep) or do label generation client-side. Defer.

4. **`extract_metadata_single` may not exist on all systems.** ExifTool is required; if missing, EXIF auto-fill silently skips (the existing `let Ok(meta) = ...` handles this). User can still manually edit. No regression vs current behavior.

5. **Watcher CPU/memory cost during long idle periods.** notify uses kqueue/FSEvents (kernel-level), nearly free. The 1s sleep loop in the watcher thread costs ~0% CPU. Acceptable.

6. **No way to "force a fresh scan" from UI.** Could add a button later; out of scope for v1 since catch-up runs on every reconnect.

7. **Watcher running while user is in Library view editing.** The DB lock contention is bounded — process_inbox_file holds the lock for ~1ms (insert) + ~1ms (metadata update). Not user-perceptible.

8. **What if `_inbox/` is on a different drive than `source_directory`?** Currently we derive `_inbox` from `source_directory`'s drive. If user wants to put inbox elsewhere, we'd need a separate setting. Not in scope.

9. **Open-question deferral: when user-sourced** *(this exists in roadmap.md but defers to actual workflow)* — should the watcher also generate full-quality thumbnails or just EXIF thumbnails? Currently we generate full Lanczos3 (via `generate_thumbnail_for_image`). For one-at-a-time arrivals this is fine; if a 100-photo batch lands at once it'll take ~30s. Could move to a queue + EXIF-thumb-first strategy if perf becomes an issue.

---

## Verification (manual test recipe)

| Step | Expected |
| --- | --- |
| Boot app with drive connected. Navigate to Import inbox. | Empty state ("No batches yet"). |
| Drop `test.jpg` into `<drive>/_inbox/`. | Within ~3s, batch list shows "Batch #1 — 1 photos" with one card. Card shows amber "Awaiting" chip. |
| Drop 5 more JPEGs in quick succession. | Same batch grows to "Batch #1 — 6 photos". |
| Wait 6 minutes. Drop one more. | New batch "Batch #2 — 1 photos" appears above. Batch #1 has `closed_at` set. |
| Click image → edit Title in Detail view → save (blur). | Return to Inbox. That image's "Awaiting" chip is gone. |
| Total "Awaiting" count in sidebar matches sum of remaining awaiting chips across all batches. | ✓ |
| Eject drive. | Indicator goes red (Plan 6). DriveDisconnectedScreen overlays main content. Watcher logs nothing alarming. |
| Reconnect drive. Before opening Inbox view, drop 2 more files. | When you open Inbox, catch-up scan has imported them into a new batch. |
| `bun run tauri dev` → no Rust runtime errors in console; SvelteKit no errors. | ✓ |

## Estimated size

**L.** Roughly 12–15 hours of focused work. Backend (~6h: schema + import logic + watcher), Frontend (~4h: store + 4 components), wire-up + smoke (~2h).
