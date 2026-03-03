# Architecture

## High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  macOS App (.app bundle)                                        │
│                                                                 │
│  ┌────────────────────────────┐   ┌────────────────────────┐   │
│  │   SvelteKit Frontend       │   │   Rust Backend (Tauri)  │   │
│  │   (WebView / WKWebView)    │◄──►   src-tauri/src/        │   │
│  │                            │   │                        │   │
│  │   Svelte 5 + Tailwind CSS  │   │  ┌──────────────────┐  │   │
│  │   @tanstack/svelte-virtual │   │  │  SQLite (rusqlite)│  │   │
│  │   @tauri-apps/api          │   │  └──────────────────┘  │   │
│  └────────────────────────────┘   │  ┌──────────────────┐  │   │
│                                   │  │  ExifTool (CLI)   │  │   │
│                                   │  └──────────────────┘  │   │
│                                   │  ┌──────────────────┐  │   │
│                                   │  │  image-rs crate   │  │   │
│                                   │  └──────────────────┘  │   │
│                                   └────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
         │                                   │
         │                                   ▼
         │                        ┌──────────────────────┐
         │                        │  External Hard Drive  │
         │                        │  /Volumes/Archive/    │
         │                        │  50,000+ images       │
         └───────────────────────►│  (read-only access)   │
                                  └──────────────────────┘
```

## Frontend ↔ Backend Communication

The frontend calls Rust functions via the **Tauri invoke pattern**:

```typescript
// Frontend (TypeScript)
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('command_name', { param1: value1 });

// Backend (Rust)
#[tauri::command]
fn command_name(param1: String, state: tauri::State<AppState>) -> Result<ReturnType, String> { ... }
```

All commands are wrapped in typed TypeScript functions under `src/lib/commands/`. Components never call `invoke()` directly.

## Database

- SQLite database at `~/Library/Application Support/org.wnp.imagearchive/archive_manager.db`
- Schema embedded at compile time from `src-tauri/sql/schema.sql` via `include_str!()`
- Migrations run on every startup (all `CREATE TABLE IF NOT EXISTS`)
- Thread-safe access via `Mutex<Connection>` in `AppState`
- WAL mode enabled for better read concurrency
- FTS5 virtual table for full-text search, kept in sync via triggers

## Thumbnail Caching Strategy

Two-tier system to balance import speed vs. browse quality:

### Tier 1: EXIF Thumbnail Extraction (during import)
- Run `exiftool -b -ThumbnailImage <file>` on each image
- Most JPEGs have an embedded ~160px thumbnail in their EXIF data
- This is a byte-copy — no image decoding needed → extremely fast (milliseconds/image)
- Output saved to `<app_data_dir>/thumbnails/<id>.jpg`
- `thumbnail_generated = 0` in database

### Tier 2: Full Quality Generation (on demand)
- When an image scrolls into view in the grid, the frontend queues it for regeneration
- `thumbnailQueue.ts` debounces and batches these requests (every 300ms, batches of 20)
- Rust resizes to 300×300px (Lanczos3) and overwrites the EXIF thumbnail at the same path
- `thumbnail_generated = 1` in database
- Grid item detects the update and appends a `?t=<timestamp>` cache-buster to reload

For images without embedded EXIF thumbnails (TIFFs, PNGs), a full-quality thumbnail is generated immediately during import as a fallback.

## Key Data Flows

### Import Flow
```
User selects directory
  → scan_directory (walkdir, batch INSERT, create archive collections)
  → extract_metadata_batch (exiftool -json -r, UPDATE images)
  → extract_exif_thumbnails_batch (exiftool -b -ThumbnailImage, save JPEGs)
  → Navigate to library grid
```

### Browse Flow
```
Grid mounts
  → query_images (paginated, filters from store)
  → Display EXIF thumbnails immediately
  → As rows scroll into view → thumbnailQueue.add(id)
  → generate_full_thumbnails (batch, Lanczos3 resize)
  → Grid items reload with updated thumbnails
```

### Filter/Search Flow
```
User changes filter/search
  → filters store updates
  → Grid subscribes → calls query_images with new filters
  → For text search: FTS5 MATCH query on images_fts virtual table
  → For filters: dynamic WHERE clause (parameterized, whitelist-validated)
```
