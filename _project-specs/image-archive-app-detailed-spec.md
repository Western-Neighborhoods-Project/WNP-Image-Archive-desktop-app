# Image Archive Manager — Detailed Implementation Spec

> **Purpose**: This document provides implementation-level instructions for building the Image Archive Manager, a macOS desktop app for a history/archive organization. It is written to be consumed by an LLM coding agent (Claude Code) and should be followed sequentially, phase by phase. Each phase builds on the previous one.
>
> **For the high-level project overview, design decisions, and rationale**, see the companion document: `image-archive-app-spec.md`.

---

## Table of Contents

1. [Project Context](#project-context)
2. [Development Principles](#development-principles)
3. [Documentation Standards](#documentation-standards)
4. [Phase 1 — Beta: Project Setup, Import & Grid](#phase-1--beta-project-setup-import--grid)
5. [Phase 2 — Metadata Editing & Search](#phase-2--metadata-editing--search)
6. [Phase 3 — Collections](#phase-3--collections)
7. [Phase 4 — Export & Sharing](#phase-4--export--sharing)
8. [Phase 5 — Keyboard Navigation & Polish](#phase-5--keyboard-navigation--polish)
9. [Phase 6 — Backup & Operations](#phase-6--backup--operations)
10. [Future Roadmap](#future-roadmap)

---

## Project Context

**What this app does**: Manages 50,000+ images on an external hard drive for a history/archive organization. Users browse, search, filter, tag metadata, create collections, and share images. No image editing.

**Who uses it**: 1–3 office machines, always connected to the external drive. End users are non-technical. The developer who maintains the app has web development skills (Laravel, JS/TS, HTML/CSS).

**Key constraint**: The external hard drive is always mounted. We do not need to handle the "drive disconnected" scenario for v1.

**Metadata source**: The organization does NOT use Lightroom for metadata. They have a separate database. A metadata import function will be built but field mapping will be finalized later once a data export is provided. For now, build with a sensible default set of fields and make the import system pluggable.

---

## Development Principles

Follow these throughout all phases:

1. **Document as you build.** Every phase must produce or update README files. See [Documentation Standards](#documentation-standards).
2. **Commit frequently.** Each logical unit of work (a new command, a new component, a schema change) should be its own commit with a clear message.
3. **Type everything.** Use TypeScript on the frontend. Use Rust's type system fully on the backend. Define shared types/interfaces for data passed between frontend and backend.
4. **Error handling first.** Every Rust command should return `Result<T, String>` at minimum. The frontend should handle errors gracefully — show the user a message, never silently fail.
5. **Keep the Rust layer thin.** Rust handles I/O, database, process spawning, and image processing. Business logic that doesn't require system access can live in the frontend.
6. **Test with real scale.** Throughout development, test with a directory containing 50,000+ image files (even dummy JPEGs). Performance issues must be caught early.
7. **Future-proof the schema.** Include all columns from the data model even if the UI doesn't expose them yet. Adding columns later is fine; restructuring is painful.

---

## Documentation Standards

This project must maintain excellent documentation. Create and maintain the following README files:

### `README.md` (project root)
- Project name, one-paragraph description
- Screenshot(s) of the app (add as UI is built)
- Tech stack summary (Tauri, SvelteKit, Rust, SQLite, ExifTool)
- Prerequisites for development (Rust, Node.js, ExifTool installation)
- Getting started: clone, install deps, run dev, build for production
- Project structure overview (what lives where)
- Link to other docs

### `docs/ARCHITECTURE.md`
- High-level architecture diagram (text-based is fine, e.g., ASCII or Mermaid)
- How the frontend communicates with the Rust backend (Tauri invoke pattern)
- Database schema overview with relationships
- Thumbnail caching strategy
- File watching strategy
- Data flow for key operations (import, search, metadata save, share)

### `docs/RUST-COMMANDS.md`
- Every Tauri command: name, parameters (with types), return type, description, example usage from frontend
- Organized by category (file system, metadata, database, image processing, sharing, etc.)
- Updated every time a command is added or changed

### `docs/DATABASE.md`
- Full schema with CREATE TABLE statements (copy from the migration files)
- Column descriptions and constraints
- Index definitions and why each exists
- FTS5 configuration
- Migration history / changelog

### `docs/COMPONENTS.md`
- Every Svelte component: name, purpose, props, events, key behaviors
- Organized by category (layout, browsing, metadata, collections, sharing)
- Updated every time a component is added or changed

### `docs/DEVELOPMENT.md`
- Detailed development setup instructions
- How to run in dev mode
- How to build a production `.app` bundle
- How to run with a test image directory
- Debugging tips (Tauri devtools, Rust logging, SQLite inspection)
- Common issues and solutions

### `docs/IMPORT.md`
- How the metadata import system works
- How to add a new import adapter (for when the org's metadata export format is provided)
- Current field mappings
- ExifTool field name reference

### Per-phase update rule
At the end of each phase, review ALL documentation files and update them to reflect what was built. Documentation that doesn't match the code is worse than no documentation.

---

## Phase 1 — Beta: Project Setup, Import & Grid

> **Goal**: A working Tauri app that scans a directory of 50K+ images on an external drive, extracts basic metadata via ExifTool, stores it in SQLite, generates thumbnails, and displays them in a performant virtual-scrolling grid. This phase proves the core architecture works at scale.

### Step 1.1 — Scaffold the Tauri + SvelteKit Project

**Create the project:**

```bash
# Use the Tauri CLI to scaffold a new project with SvelteKit
npm create tauri-app@latest image-archive-manager -- --template sveltekit-ts
cd image-archive-manager
```

**Verify the scaffold runs:**
- `npm install`
- `npm run tauri dev` should open a window with the SvelteKit default page
- Confirm hot-reload works on the frontend
- Confirm the Rust backend compiles

**Configure SvelteKit for static mode:**

In `svelte.config.js`, set the adapter to `adapter-static`:
```bash
npm install -D @sveltejs/adapter-static
```
```js
import adapter from '@sveltejs/adapter-static';
export default {
  kit: {
    adapter: adapter({
      fallback: 'index.html'
    })
  }
};
```

**Install frontend dependencies for Phase 1:**
```bash
npm install -D tailwindcss @tailwindcss/vite
```

Configure Tailwind:
- Add the Tailwind Vite plugin to `vite.config.ts`
- Add `@import "tailwindcss"` to a global CSS file imported in the root layout
- Set up the system font stack in Tailwind config:
```js
// In your CSS or Tailwind config
// Use Tailwind's default font-family customization to set:
// font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
```

**Note on shadcn-svelte:** Do NOT install shadcn-svelte yet. Phase 1 uses minimal UI — plain HTML elements styled with Tailwind are sufficient. shadcn-svelte will be added in Phase 2 when form components become necessary. This keeps Phase 1 focused on proving the backend architecture.

**Add Rust dependencies to `src-tauri/Cargo.toml`:**
```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
walkdir = "2"
image = "0.25"
```

Do NOT add `notify`, `reqwest`, or other crates not needed for Phase 1. They'll be added in their respective phases.

**Verify everything compiles:** `npm run tauri dev` should still work after adding dependencies.

**Deliverables:**
- Working Tauri + SvelteKit + Tailwind project
- Rust compiles with SQLite, walkdir, image crates
- `README.md` with project description, tech stack, and getting started instructions
- `docs/DEVELOPMENT.md` with setup instructions

---

### Step 1.2 — SQLite Database Setup

**Create a database initialization module in Rust.**

File: `src-tauri/src/db.rs`

This module should:
1. Create or open a SQLite database file at a known location. Use the Tauri app data directory: `tauri::api::path::app_data_dir()` / `archive_manager.db`
2. Run migrations on startup to create all tables
3. Export a function to get a database connection that other modules can use

**Create ALL tables from the schema, even those not used until later phases.** This future-proofs the database and avoids migration headaches.

```sql
-- Images table (core)
CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    catalog_number TEXT NOT NULL,  -- The archive's unique identifier for this image (derived from filename without extension)
    file_size INTEGER,
    file_modified TEXT,  -- ISO 8601 string
    title TEXT,
    description TEXT,
    city TEXT,
    state TEXT,
    country TEXT,
    keywords TEXT,  -- JSON array: ["keyword1", "keyword2"]
    date_display TEXT,  -- Human-readable: "ca. 1920", "Spring 1968"
    date_start TEXT,  -- ISO 8601 date for filtering
    date_end TEXT,  -- ISO 8601 date for range queries
    photographer TEXT,
    donor TEXT,
    acquisition_date TEXT,
    archival_collection TEXT,
    usage_rights TEXT,
    internal_notes TEXT,
    thumbnail_path TEXT,
    thumbnail_generated INTEGER DEFAULT 0,  -- 0 = EXIF thumbnail only, 1 = full quality generated
    metadata_synced INTEGER DEFAULT 0,  -- 0 = not synced, 1 = synced with file
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Indexes on images
CREATE INDEX IF NOT EXISTS idx_images_catalog_number ON images(catalog_number);
CREATE INDEX IF NOT EXISTS idx_images_city ON images(city);
CREATE INDEX IF NOT EXISTS idx_images_date_start ON images(date_start);
CREATE INDEX IF NOT EXISTS idx_images_archival_collection ON images(archival_collection);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS images_fts USING fts5(
    catalog_number,
    title,
    description,
    city,
    keywords,
    photographer,
    internal_notes,
    content='images',
    content_rowid='id'
);

-- FTS triggers to keep the index in sync
CREATE TRIGGER IF NOT EXISTS images_ai AFTER INSERT ON images BEGIN
    INSERT INTO images_fts(rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES (new.id, new.catalog_number, new.title, new.description, new.city, new.keywords, new.photographer, new.internal_notes);
END;

CREATE TRIGGER IF NOT EXISTS images_ad AFTER DELETE ON images BEGIN
    INSERT INTO images_fts(images_fts, rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES ('delete', old.id, old.catalog_number, old.title, old.description, old.city, old.keywords, old.photographer, old.internal_notes);
END;

CREATE TRIGGER IF NOT EXISTS images_au AFTER UPDATE ON images BEGIN
    INSERT INTO images_fts(images_fts, rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES ('delete', old.id, old.catalog_number, old.title, old.description, old.city, old.keywords, old.photographer, old.internal_notes);
    INSERT INTO images_fts(rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES (new.id, new.catalog_number, new.title, new.description, new.city, new.keywords, new.photographer, new.internal_notes);
END;

-- Collections (used for both user-created and archive-sourced collections)
CREATE TABLE IF NOT EXISTS collections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',  -- 'user' = created in app, 'archive' = auto-created from directory structure
    description TEXT,  -- Optional description, auto-populated for archive collections with folder path
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_collections_source ON collections(source);

CREATE TABLE IF NOT EXISTS collection_images (
    collection_id INTEGER NOT NULL,
    image_id INTEGER NOT NULL,
    added_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (collection_id, image_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);

-- Smart collections
CREATE TABLE IF NOT EXISTS smart_collections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    filters TEXT NOT NULL,  -- JSON filter definition
    created_at TEXT DEFAULT (datetime('now'))
);

-- Image requests (mirrored from Laravel API)
CREATE TABLE IF NOT EXISTS image_requests (
    id INTEGER PRIMARY KEY,  -- Matches Laravel record ID, NOT autoincrement
    image_catalog_number TEXT,  -- Catalog number of the requested image
    requester_email TEXT,
    requester_name TEXT,
    requested_resolution TEXT,
    purpose TEXT,
    status TEXT DEFAULT 'pending',
    fetched_at TEXT DEFAULT (datetime('now'))
);

-- Audit log
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id INTEGER NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by TEXT DEFAULT 'local',
    changed_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_audit_log_image_id ON audit_log(image_id);

-- Usage log
CREATE TABLE IF NOT EXISTS usage_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id INTEGER NOT NULL,
    recipient_email TEXT,
    recipient_name TEXT,
    purpose TEXT,
    resolution_sent TEXT,
    request_id INTEGER,
    shared_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE,
    FOREIGN KEY (request_id) REFERENCES image_requests(id)
);
CREATE INDEX IF NOT EXISTS idx_usage_log_image_id ON usage_log(image_id);

-- Recently viewed
CREATE TABLE IF NOT EXISTS recently_viewed (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id INTEGER NOT NULL UNIQUE,
    viewed_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);

-- App settings (key-value store for preferences)
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

**Note on the `app_settings` table:** This is an addition to the original spec. Use it to store:
- `source_directory` — the path to the external drive's image directory
- `thumbnail_cache_path` — where thumbnails are stored
- `last_scan_time` — when the last full directory scan completed
- Any other app configuration

**Use `rusqlite::Connection` wrapped in a `Mutex<Connection>`** managed by Tauri's state system. This ensures thread-safe access from multiple Tauri commands.

```rust
// In main.rs or a state module:
use std::sync::Mutex;
use rusqlite::Connection;

pub struct AppState {
    pub db: Mutex<Connection>,
}
```

Register it with Tauri's `manage()` in the builder.

**Deliverables:**
- `src-tauri/src/db.rs` — database initialization and migration
- Database created automatically on first launch
- All tables created (even those for later phases)
- `docs/DATABASE.md` — full schema documentation with CREATE statements and column descriptions

---

### Step 1.3 — Directory Scanning

**Create a Rust module for file system operations.**

File: `src-tauri/src/scanner.rs`

**Tauri command: `scan_directory`**

```rust
#[tauri::command]
fn scan_directory(path: String, state: tauri::State<AppState>) -> Result<ScanResult, String>
```

Behavior:
1. Takes a directory path (the external drive root, e.g., `/Volumes/ArchiveDrive/Images`)
2. Uses `walkdir` to recursively find all image files
3. Filter for supported extensions: `.jpg`, `.jpeg`, `.tif`, `.tiff`, `.png`, `.gif`, `.bmp`, `.webp` (case-insensitive)
4. For each file, collect: full path, catalog number (filename without extension), file size (bytes), last modified time, **parent subdirectory name** (the immediate folder containing the file)
5. Insert new files into the `images` table (skip files that already exist by `file_path`). Set `catalog_number` from the filename without extension. Set `archival_collection` from the parent subdirectory name.
6. **Auto-create archive collections:** After scanning, query `SELECT DISTINCT archival_collection FROM images WHERE archival_collection IS NOT NULL`. For each unique subdirectory name, create a `collections` row with `source = 'archive'` and `name` set to the subdirectory name (if one doesn't already exist). Then populate the `collection_images` junction table by joining images to their corresponding archive collection.
7. Return a `ScanResult` struct with counts: `{ total_files: u64, new_files: u64, archive_collections_found: u64, scan_duration_ms: u64 }`

**Important performance considerations:**
- This command will be called with 50K+ files. It must not block the UI.
- Use `walkdir` with default settings — it's already efficient.
- Batch SQLite inserts using a transaction: `BEGIN TRANSACTION` → all inserts → `COMMIT`. This is dramatically faster than individual inserts (100x+ improvement).
- Do NOT extract metadata in this step. That's a separate pass. This step only collects file system information.

**Tauri command: `get_scan_stats`**

```rust
#[tauri::command]
fn get_scan_stats(state: tauri::State<AppState>) -> Result<ScanStats, String>
```

Returns: `{ total_images: u64, images_with_thumbnails: u64, images_without_metadata: u64 }`

This lets the frontend show the state of the catalog at any time.

**Data types (define in a shared `models.rs`):**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ScanResult {
    pub total_files: u64,
    pub new_files: u64,
    pub archive_collections_found: u64,
    pub scan_duration_ms: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ImageRecord {
    pub id: i64,
    pub file_path: String,
    pub catalog_number: String,
    pub file_size: Option<i64>,
    pub file_modified: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub keywords: Option<String>,
    pub date_display: Option<String>,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub photographer: Option<String>,
    pub donor: Option<String>,
    pub acquisition_date: Option<String>,
    pub archival_collection: Option<String>,
    pub usage_rights: Option<String>,
    pub internal_notes: Option<String>,
    pub thumbnail_path: Option<String>,
    pub thumbnail_generated: bool,
    pub metadata_synced: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

**Deliverables:**
- `src-tauri/src/scanner.rs` — directory scanning logic
- `src-tauri/src/models.rs` — shared data types
- `scan_directory` and `get_scan_stats` commands registered with Tauri
- Tested with a large directory (ideally 50K+ files, or extrapolated from smaller test)
- Update `docs/RUST-COMMANDS.md`

---

### Step 1.4 — ExifTool Metadata Extraction

**Create a Rust module for ExifTool integration.**

File: `src-tauri/src/metadata.rs`

**ExifTool setup:**
- ExifTool must be installed on the development machine (`brew install exiftool` on macOS)
- For production distribution, ExifTool will be bundled inside the `.app` — but for Phase 1 development, assume it's available at `/usr/local/bin/exiftool` or on PATH
- Add an `app_settings` entry for `exiftool_path` so this is configurable

**Tauri command: `extract_metadata_batch`**

```rust
#[tauri::command]
async fn extract_metadata_batch(directory: String, state: tauri::State<'_, AppState>) -> Result<MetadataImportResult, String>
```

Behavior:
1. Run `exiftool -json -r -fast2 <directory>` as a subprocess
   - `-json` outputs JSON (one object per file)
   - `-r` recurses into subdirectories
   - `-fast2` skips MakerNotes for faster processing (they're rarely needed for catalog purposes)
2. Parse the JSON output (it's an array of objects)
3. For each object, map ExifTool field names to our schema fields:
   - `FileName` → `catalog_number` (strip extension) — Note: this is also set during directory scan (Step 1.3). ExifTool extraction can verify/confirm it but the scan is the primary source.
   - `Title` or `ObjectName` (IPTC) → `title`
   - `Description` or `Caption-Abstract` (IPTC) → `description`
   - `City` (IPTC) → `city`
   - `Province-State` (IPTC) → `state`
   - `Country-PrimaryLocationName` (IPTC) → `country`
   - `Keywords` (IPTC) → `keywords` (convert to JSON array)
   - `DateTimeOriginal` or `CreateDate` → parse into `date_start` (ISO 8601)
   - `Creator` or `Artist` or `By-line` → `photographer`
   - `CopyrightNotice` or `Rights` → `usage_rights`
4. Update the corresponding `images` row (match on `file_path`)
5. Set `metadata_synced = 1` for updated rows
6. Return: `{ processed: u64, updated: u64, errors: u64, duration_ms: u64 }`

**IMPORTANT: The field mapping above is a starting point.** The organization's metadata may come from an external database, not embedded in the images. Build this extraction as one "adapter" in a pluggable system:

```rust
// metadata.rs should define a trait or pattern like:
pub struct ExtractedMetadata {
    pub file_path: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub city: Option<String>,
    // ... all fields
}

// exiftool_adapter extracts from image files
// future: csv_adapter, json_adapter, etc. will import from external data exports
```

**Tauri command: `extract_metadata_single`**

```rust
#[tauri::command]
fn extract_metadata_single(file_path: String) -> Result<ExtractedMetadata, String>
```

Runs ExifTool on a single file and returns the parsed metadata. Used when a new file is detected or when refreshing a single image's metadata.

**Deliverables:**
- `src-tauri/src/metadata.rs` — ExifTool integration with pluggable adapter pattern
- `extract_metadata_batch` and `extract_metadata_single` commands
- Field mapping documented in `docs/IMPORT.md`
- Update `docs/RUST-COMMANDS.md`

---

### Step 1.5 — Thumbnail Generation (Two-Tier Strategy)

**Create a Rust module for image processing.**

File: `src-tauri/src/thumbnails.rs`

**Strategy overview:**

Thumbnails use a two-tier approach to balance import speed with browsing quality:

1. **During import (fast):** Extract the embedded EXIF thumbnail from each image using ExifTool. Most JPEGs contain a small thumbnail (~160px, 5-10KB) in their EXIF data. This is a byte-copy operation — no image processing needed. Takes seconds per image, not minutes.
2. **On demand (lazy):** When an image scrolls into view in the grid and only has an EXIF thumbnail (`thumbnail_generated = 0`), queue it for full-quality thumbnail generation (300px, Lanczos3 resize). The generated thumbnail overwrites the EXIF version at the same path, and `thumbnail_generated` is set to `1`.

This means the initial import is dramatically faster (minutes instead of hours for 50K images), the app is immediately browsable after import, and full-quality thumbnails are progressively generated as the user browses.

**Thumbnail cache directory:**
- Store thumbnails in the Tauri app data directory: `<app_data_dir>/thumbnails/`
- Name each thumbnail by the image's database ID: `<id>.jpg`
- Always output as JPEG regardless of source format (saves space, consistent loading)
- The same path is used for both EXIF and generated thumbnails — generation overwrites the EXIF version in place

**Tauri command: `extract_exif_thumbnails_batch`**

```rust
#[tauri::command]
async fn extract_exif_thumbnails_batch(state: tauri::State<'_, AppState>) -> Result<ThumbnailResult, String>
```

Behavior:
1. Query the database for all images where `thumbnail_path IS NULL`
2. For each image, run: `exiftool -b -ThumbnailImage <file_path>`
   - If the image has an embedded thumbnail, save the output to `<cache_dir>/<id>.jpg`
   - If no embedded thumbnail exists (common for TIFFs, PNGs), fall back to generating a full thumbnail immediately using the `image` crate (since there's no EXIF shortcut available)
3. Update `thumbnail_path` in the database for all processed images
4. Set `thumbnail_generated = 0` for EXIF-extracted thumbnails, `thumbnail_generated = 1` for fallback-generated ones
5. Return: `{ extracted: u64, fallback_generated: u64, failed: u64, duration_ms: u64 }`

**Performance note:** ExifTool can extract thumbnails from multiple files in a single invocation. Rather than calling ExifTool once per image, batch them:
```bash
exiftool -b -ThumbnailImage -w <cache_dir>/%f.jpg <directory>
```
This is significantly faster than individual calls. However, the output naming needs to be mapped back to database IDs, so you may need to process in chunks and rename. An alternative is to use ExifTool's `-json` output with `-b -ThumbnailImage` to get base64-encoded thumbnail data that you decode and save with the correct ID-based filename.

**Tauri command: `generate_full_thumbnails`**

```rust
#[derive(serde::Deserialize)]
pub struct ThumbnailRequest {
    pub image_ids: Vec<i64>,
}

#[tauri::command]
async fn generate_full_thumbnails(request: ThumbnailRequest, state: tauri::State<'_, AppState>) -> Result<ThumbnailResult, String>
```

Behavior:
1. Accept a batch of image IDs (sent by the frontend when images scroll into view)
2. For each image:
   a. Open the source file using the `image` crate
   b. Resize to fit within 300x300 pixels, maintaining aspect ratio. Use `image::imageops::FilterType::Lanczos3` for quality.
   c. Save as JPEG (quality 80) to the thumbnail cache directory, overwriting the existing EXIF thumbnail at the same path
   d. Set `thumbnail_generated = 1` in the database
3. Process images sequentially (disk I/O on external drive is the bottleneck)
4. Limit batch size to prevent long-running operations — the frontend should send batches of ~10-20 at a time
5. Return: `{ generated: u64, failed: u64, duration_ms: u64 }`

**Tauri command: `generate_thumbnail_single`**

```rust
#[tauri::command]
fn generate_thumbnail_single(image_id: i64, state: tauri::State<AppState>) -> Result<String, String>
```

Generates a full-quality thumbnail for a single image. Returns the thumbnail path. Used for newly added files detected by the file watcher.

**Error handling:**
- Some image files may be corrupted or in unsupported formats. Log the error, skip the file, and continue. Do not stop the batch.
- TIFF files can be very large (100MB+). The `image` crate handles them but they'll be slow. This is expected.
- If EXIF thumbnail extraction fails AND full generation fails, set `thumbnail_path` to a sentinel value (e.g., the path to a built-in placeholder image) so the grid can still display something.

**Deliverables:**
- `src-tauri/src/thumbnails.rs` — two-tier thumbnail system (EXIF extraction + on-demand generation)
- Thumbnail cache directory created on first run
- `extract_exif_thumbnails_batch`, `generate_full_thumbnails`, and `generate_thumbnail_single` commands
- Update `docs/RUST-COMMANDS.md`

---

### Step 1.6 — Image Query Commands

**These Rust commands provide the data the frontend grid needs.**

Add to `src-tauri/src/db.rs` or a new `src-tauri/src/queries.rs`:

**Tauri command: `query_images`**

```rust
#[derive(serde::Deserialize)]
pub struct ImageQuery {
    pub offset: u64,
    pub limit: u64,
    pub sort_by: Option<String>,      // "catalog_number", "date_start", "created_at", "updated_at"
    pub sort_order: Option<String>,   // "asc", "desc"
}

#[derive(serde::Serialize)]
pub struct ImageQueryResult {
    pub images: Vec<ImageRecord>,
    pub total_count: u64,
}

#[tauri::command]
fn query_images(query: ImageQuery, state: tauri::State<AppState>) -> Result<ImageQueryResult, String>
```

Behavior:
- Returns a page of images with offset/limit pagination
- Always returns `total_count` so the frontend can calculate total scroll height
- Default sort: `catalog_number ASC`
- Validate `sort_by` against a whitelist of allowed column names (prevent SQL injection)

**Tauri command: `get_image`**

```rust
#[tauri::command]
fn get_image(id: i64, state: tauri::State<AppState>) -> Result<ImageRecord, String>
```

Returns a single image with all fields. Used for the detail view (Phase 2) and other lookups.

**Deliverables:**
- `query_images` and `get_image` commands
- Pagination works correctly at 50K+ scale
- Update `docs/RUST-COMMANDS.md`

---

### Step 1.7 — Frontend: Initial Setup Screen

**Build a simple first-run experience.**

When the app launches and no `source_directory` is set in `app_settings`, show a setup screen:

**File: `src/routes/+page.svelte`** (or a dedicated setup component)

UI:
- App title/logo placeholder
- Text: "Welcome to Image Archive Manager. Select the directory containing your image archive."
- A "Select Directory" button
- Clicking it opens a native macOS directory picker (use Tauri's `dialog.open` API with `directory: true`)
- After selection, store the path in `app_settings` table via a Tauri command
- Show a "Start Import" button
- Clicking it triggers the import sequence (Step 1.8)

**Tauri command: `set_setting` / `get_setting`**

```rust
#[tauri::command]
fn set_setting(key: String, value: String, state: tauri::State<AppState>) -> Result<(), String>

#[tauri::command]
fn get_setting(key: String, state: tauri::State<AppState>) -> Result<Option<String>, String>
```

Simple key-value operations on the `app_settings` table.

**Frontend Tauri invoke wrapper:**

Create `src/lib/commands/settings.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core';

export async function getSetting(key: string): Promise<string | null> {
  return invoke('get_setting', { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke('set_setting', { key, value });
}
```

Follow this pattern for ALL Tauri command wrappers: a TypeScript file in `src/lib/commands/` that provides typed functions. Never call `invoke()` directly from components.

**Deliverables:**
- Setup screen component
- `set_setting` / `get_setting` Tauri commands
- `reset_catalog` Tauri command
- `src/lib/commands/settings.ts` — typed invoke wrappers
- Native directory picker integration
- After setup, app remembers the source directory on relaunch
- Settings view allows changing source directory with full reset

---

### Step 1.7b — Change Source Directory / Reset Catalog

**This functionality allows pointing the app at a different directory, which triggers a full catalog reset.**

Accessible from: a settings view (add a gear icon in the sidebar or top bar that navigates to a simple settings page).

**UI:**
- Current source directory displayed (read-only)
- "Change Source Directory" button
- Clicking it shows a confirmation dialog:
  ```
  ⚠️ Changing the source directory will reset the entire catalog.
  
  This will:
  • Remove all indexed images from the database
  • Delete all cached thumbnails
  • Clear all user collections, smart collections, and archive collections
  • Clear the audit log and usage log
  
  The original image files on the drive are NOT affected.
  Archive collections will be re-detected from the new directory's folder structure.
  
  [Cancel] [Reset and Choose New Directory]
  ```
- On confirm: open directory picker → reset → re-run import

**Tauri command: `reset_catalog`**

```rust
#[tauri::command]
fn reset_catalog(state: tauri::State<AppState>) -> Result<(), String>
```

Behavior:
1. Delete all rows from: `images`, `collections`, `collection_images`, `smart_collections`, `image_requests`, `audit_log`, `usage_log`, `recently_viewed`
2. Reset the FTS5 index: `DELETE FROM images_fts`
3. Delete all files in the thumbnail cache directory
4. Remove the `source_directory` and `last_scan_time` entries from `app_settings`
5. Do NOT delete other app_settings (like `laravel_api_url`, backup paths, etc.)

After reset, the app returns to the setup screen (Step 1.7) where the user selects a new directory and re-imports.

**Frontend flow:**
1. User clicks "Change Source Directory" in settings
2. Confirmation dialog shown
3. On confirm → call `reset_catalog` → navigate to setup screen
4. User selects new directory → import runs as normal

---

### Step 1.8 — Frontend: Import Progress Screen

**After the user selects a directory, run the import sequence and show progress.**

The import has three stages that run sequentially:
1. **Directory scan** — find all image files (fast, seconds)
2. **Metadata extraction** — run ExifTool on all files (slow, minutes to hours)
3. **EXIF thumbnail extraction** — extract embedded thumbnails from all images (fast, minutes — NOT full thumbnail generation)

Full-quality thumbnail generation happens on demand as users browse. See Step 1.5 for details.

**UI for the import screen:**

- Show which stage is running: "Scanning files..." → "Extracting metadata..." → "Extracting thumbnails..."
- Show progress counts where possible (e.g., "Extracted 1,234 / 52,000 thumbnails")
- A simple progress bar (can be approximate)
- When complete, show summary stats and a "Browse Library" button
- Note: the import should complete much faster than a full thumbnail generation pass — minutes, not hours

**Implementation approach for progress reporting:**

For the batch operations (metadata extraction, thumbnail generation), the simplest approach for Phase 1 is:
1. Call the batch Tauri command (it runs to completion)
2. Poll `get_scan_stats` on a timer (every 2 seconds) to update the progress display
3. When the batch command resolves, stop polling and show completion

This avoids the complexity of Tauri event streaming for now. Event-based progress can be added in Phase 5 (Polish) if needed.

**Deliverables:**
- Import progress screen component
- Three-stage import flow works end-to-end
- Progress display updates during import
- Summary stats on completion
- Navigation to the grid view after import

---

### Step 1.9 — Frontend: Virtual-Scrolling Image Grid

**This is the most critical frontend component. It must handle 50K+ images smoothly.**

**File: `src/lib/components/Grid.svelte`**

**Install the virtual scrolling library:**
```bash
npm install @tanstack/svelte-virtual
```

**Grid architecture:**

1. On mount, call `query_images` with `{ offset: 0, limit: 100, sort_by: 'catalog_number', sort_order: 'asc' }` to get the first page AND `total_count`
2. Calculate grid layout:
   - Thumbnail display size: 200x200px (CSS, with object-fit: cover)
   - Gap between items: 8px
   - Columns: `Math.floor(containerWidth / (200 + 8))`
   - Total rows: `Math.ceil(totalCount / columns)`
   - Row height: 200 + 8px (thumbnail + gap) + 24px (catalog number label below) = ~232px
3. Use `@tanstack/svelte-virtual` to virtualize the rows
4. As the user scrolls, calculate which image indices are visible
5. Fetch pages of images as needed (prefetch 1-2 pages ahead of scroll position)

**Thumbnail loading:**

Each grid item shows:
- The thumbnail image, loaded from the local thumbnail cache path
- The catalog number below the image
- Use Tauri's `convertFileSrc()` to convert local file paths to asset URLs the webview can load:
  ```typescript
  import { convertFileSrc } from '@tauri-apps/api/core';
  // convertFileSrc('/path/to/thumb.jpg') → 'asset://localhost/path/to/thumb.jpg'
  ```

**GridItem component:**

**File: `src/lib/components/GridItem.svelte`**

```svelte
<!-- Minimal structure: -->
<div class="grid-item">
  <img
    src={thumbnailSrc}
    alt={catalogNumber}
    loading="lazy"
    class="w-[200px] h-[200px] object-cover rounded"
  />
  <span class="text-xs text-gray-600 truncate w-[200px]">{catalogNumber}</span>
</div>
```

Use the browser's native `loading="lazy"` as an additional optimization layer on top of virtual scrolling.

**On-demand thumbnail generation queue:**

When grid items scroll into view, the grid needs to check whether visible images need full-quality thumbnails generated (`thumbnail_generated === false`). Implement a simple queue:

**File: `src/lib/utils/thumbnailQueue.ts`**

```typescript
// Manages on-demand full thumbnail generation for visible grid items
// - Collects image IDs that need generation as they scroll into view
// - Debounces to batch requests (waits 300ms after last addition)
// - Sends batches of 10-20 IDs to the Rust backend
// - On completion, notifies the grid to refresh the affected thumbnails
// - Deduplicates: never queues the same ID twice
```

The Grid component calls `thumbnailQueue.add(imageId)` for each visible image where `thumbnail_generated === false`. The queue debounces, batches, and calls `generate_full_thumbnails`. When generation completes, the grid item's `<img>` src is refreshed (append a cache-buster query param like `?t=timestamp` since the file path hasn't changed but the file content has).

This should be invisible to the user — EXIF thumbnails display immediately, and if you look closely you might notice them sharpen slightly as the full-quality versions replace them. In practice, most users won't notice.

**Page caching:**

Maintain a simple in-memory cache of fetched pages:
```typescript
// In a store or the Grid component
const pageCache = new Map<number, ImageRecord[]>();
const PAGE_SIZE = 100;

async function getPage(pageIndex: number): Promise<ImageRecord[]> {
  if (pageCache.has(pageIndex)) return pageCache.get(pageIndex)!;
  const result = await queryImages({
    offset: pageIndex * PAGE_SIZE,
    limit: PAGE_SIZE,
    sort_by: 'catalog_number',
    sort_order: 'asc'
  });
  pageCache.set(pageIndex, result.images);
  return result.images;
}
```

**Performance targets:**
- Initial grid render: < 500ms after data is loaded
- Scroll performance: 60fps (no visible jank)
- Memory: Should not grow unboundedly. Consider evicting distant pages from the cache if memory becomes an issue (unlikely at 50K but good practice).

**File: `src/lib/commands/images.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface ImageRecord {
  id: number;
  file_path: string;
  catalog_number: string;
  file_size: number | null;
  file_modified: string | null;
  title: string | null;
  description: string | null;
  city: string | null;
  state: string | null;
  country: string | null;
  keywords: string | null;
  date_display: string | null;
  date_start: string | null;
  date_end: string | null;
  photographer: string | null;
  donor: string | null;
  acquisition_date: string | null;
  archival_collection: string | null;
  usage_rights: string | null;
  internal_notes: string | null;
  thumbnail_path: string | null;
  thumbnail_generated: boolean;
  metadata_synced: boolean;
  created_at: string;
  updated_at: string;
}

export interface ImageQuery {
  offset: number;
  limit: number;
  sort_by?: string;
  sort_order?: 'asc' | 'desc';
}

export interface ImageQueryResult {
  images: ImageRecord[];
  total_count: number;
}

export async function queryImages(query: ImageQuery): Promise<ImageQueryResult> {
  return invoke('query_images', { query });
}

export async function getImage(id: number): Promise<ImageRecord> {
  return invoke('get_image', { id });
}
```

**Deliverables:**
- `Grid.svelte` with virtual scrolling using `@tanstack/svelte-virtual`
- `GridItem.svelte` displaying thumbnail + catalog number
- Page-based data fetching with caching
- `src/lib/commands/images.ts` — typed invoke wrappers
- Tested and smooth at 50K+ images
- Update `docs/COMPONENTS.md`

---

### Step 1.10 — Frontend: Basic Layout Shell

**Create the app layout that will persist across all phases.**

**File: `src/routes/+layout.svelte`**

Layout structure:
```
┌─────────────────────────────────────────────┐
│ Top Bar (search + filters — placeholder)    │
├──────────┬──────────────────────────────────┤
│ Sidebar  │ Main Content Area               │
│          │                                  │
│ Library  │ (Grid / Detail / etc.)           │
│ Recent   │                                  │
│ ...      │                                  │
│          │                                  │
└──────────┴──────────────────────────────────┘
```

**Sidebar for Phase 1:**
- "Library" link (active) — shows the full grid
- "Recently Viewed" link — placeholder for now, just the label
- Divider
- "Archive Collections" section — list archive collections auto-created from directory scan. Each shows name + image count. Clicking one filters the grid to that collection. This is functional in Phase 1 since archive collections are created during import.
- Divider
- Image count: "52,341 images" (from `get_scan_stats`)

Style the sidebar with:
- Fixed width: 220px
- Background: slightly translucent (`bg-gray-50/80 backdrop-blur-md`)
- Border-right: subtle (`border-r border-gray-200`)
- Dark mode compatible (add `dark:` variants)

**Top bar for Phase 1:**
- App name on the left
- Placeholder text "Search and filters coming in Phase 2" (or just leave it as reserved space)
- Sort dropdown (catalog number, date, recent) — functional, wired to the grid's query

**Navigation store:**

**File: `src/lib/stores/navigation.ts`**

```typescript
import { writable } from 'svelte/store';

export type ViewType = 'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests';

export const currentView = writable<ViewType>('library');
export const currentImageId = writable<number | null>(null);
export const currentCollectionId = writable<number | null>(null);
```

The main content area in the layout switches based on `currentView`.

**Deliverables:**
- Layout component with sidebar, top bar, main content area
- Navigation store
- Sort dropdown wired to the grid
- Sidebar shows image count
- Conditional rendering: setup screen → import screen → library grid

---

### Step 1.11 — End-to-End Testing & Phase 1 Docs

**Before moving to Phase 2, verify the complete flow works:**

1. Fresh launch → setup screen → select a test directory with images
2. Click import → directory scan completes → metadata extraction runs → EXIF thumbnail extraction completes
3. Grid displays all images with EXIF thumbnails, scrolls smoothly
4. As images scroll into view, full-quality thumbnails generate in the background and replace EXIF versions
5. `thumbnail_generated` flag updates correctly (check via SQLite inspection)
6. Sort by catalog number, date work correctly
7. Relaunch the app → skips setup, goes directly to library grid
8. Settings → "Change Source Directory" → confirmation dialog → reset clears database and thumbnails → setup screen appears → select a different directory → re-import works cleanly
9. Confirm performance is acceptable with a large number of images

**Testing approach:** Use a real directory of images for testing (even a few hundred is sufficient for functional testing). Point the app at different directories using the reset functionality to verify the full cycle. Performance testing with 50K+ images should be done when the real external drive is available.

**Update all documentation:**
- `README.md` — add screenshots, update getting started
- `docs/ARCHITECTURE.md` — create with architecture diagram, data flow, thumbnail strategy explanation
- `docs/RUST-COMMANDS.md` — all Phase 1 commands documented
- `docs/DATABASE.md` — full schema documentation (including `thumbnail_generated` column)
- `docs/COMPONENTS.md` — all Phase 1 components documented
- `docs/DEVELOPMENT.md` — update with any issues/tips discovered
- `docs/IMPORT.md` — create with ExifTool field mapping, adapter pattern docs, and thumbnail extraction approach

---

## Phase 2 — Metadata Editing & Search

> **Goal**: Users can view a single image at larger size, see and edit all metadata fields, search across the catalog, and filter by various criteria. Introduces shadcn-svelte for form components.

### Step 2.1 — Install shadcn-svelte

```bash
npx shadcn-svelte@latest init
```

Follow the prompts:
- Style: Default
- Base color: Slate (closest to macOS aesthetic)
- Global CSS file: point to your existing Tailwind CSS file

**Add the components needed for Phase 2:**
```bash
npx shadcn-svelte@latest add button input label select textarea dialog scroll-area separator badge tabs tooltip
```

These will be copied into `src/lib/components/ui/`. They are now project-owned code, fully customizable.

**Verify the app still builds and runs after adding shadcn-svelte.**

---

### Step 2.2 — Detail View

**File: `src/lib/components/DetailView.svelte`**

When a user clicks an image in the grid, navigate to the detail view.

**Layout:**
```
┌─────────────────────────────────────────────┐
│ ← Back to Library          Image Title      │
├─────────────────────┬───────────────────────┤
│                     │ Metadata Form         │
│   Image Preview     │                       │
│   (large)           │ Title: [___________]  │
│                     │ City: [___________]   │
│                     │ State: [___________]  │
│   click to zoom     │ ...                   │
│                     │ [Save] [Write to File]│
│                     │                       │
│                     │ ▸ Advanced Fields     │
│                     │ ▸ Internal Notes      │
└─────────────────────┴───────────────────────┘
```

**Image preview panel (left):**
- Display the image at medium resolution. Load from the original file path via `convertFileSrc()` — the browser will handle resizing for display. For very large TIFFs, consider generating a mid-size "preview" JPG (2048px) during import and loading that instead. This is an optimization that can be deferred if performance is acceptable.
- Clicking or scroll-to-zoom loads the full resolution image in a modal/overlay with pan capability. Use CSS `transform: scale()` and `translate()` for zoom/pan. This can be a simple implementation — not a full image viewer.

**Metadata form (right):**

Use shadcn-svelte `Input`, `Textarea`, `Select`, `Label` components.

**Field groupings:**

**Primary fields (always visible):**
- Title (input)
- Description (textarea)
- City (input)
- State (input)
- Country (input)
- Keywords (input — comma-separated for now, tag input in future)
- Date display (input — free text: "ca. 1920")
- Date start (date input)
- Date end (date input)
- Photographer (input)

**Archival fields (collapsible section, label: "Archival Details"):**
- Donor (input)
- Acquisition date (date input)
- Archive collection (read-only display — shows which archive collection(s) this image belongs to, derived from `collection_images` where collection `source = 'archive'`. Not directly editable since it's determined by directory structure.)
- Usage rights (select — predefined options: "Public Domain", "Editorial Only", "No Commercial Use", "Contact for Permission", "Unknown")

**Internal fields (collapsible section, label: "Internal Notes"):**
- Internal notes (textarea — larger, with placeholder: "Working notes — not shared externally")

**File info (read-only, collapsible section, label: "File Information"):**
- Catalog number
- File path
- File size (human-readable: "14.2 MB")
- File modified date
- Metadata synced status

**Recently viewed tracking:**

When the detail view opens, call a Tauri command to log the view:

```rust
#[tauri::command]
fn log_image_view(image_id: i64, state: tauri::State<AppState>) -> Result<(), String>
```

This inserts/updates `recently_viewed` and prunes to 30 entries.

---

### Step 2.3 — Metadata Save with Diff & Audit Trail

**When the user clicks "Save":**

1. Frontend compares the form values against the original values loaded from the database
2. Build a list of changed fields: `[{ field: "city", old: null, new: "San Francisco" }, ...]`
3. Show a confirmation dialog listing the changes (using shadcn `Dialog`):
   ```
   Changes to save:
   • City: (empty) → San Francisco
   • Photographer: (empty) → John Doe
   
   [Cancel] [Save]
   ```
4. On confirm, call the Tauri command to save

**Tauri command: `update_image_metadata`**

```rust
#[derive(serde::Deserialize)]
pub struct MetadataUpdate {
    pub image_id: i64,
    pub changes: Vec<FieldChange>,
}

#[derive(serde::Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[tauri::command]
fn update_image_metadata(update: MetadataUpdate, state: tauri::State<AppState>) -> Result<(), String>
```

Behavior:
1. Validate that `field` names are in a whitelist of allowed columns
2. Begin a transaction
3. Update the `images` row with the new values
4. Set `updated_at = datetime('now')`
5. Set `metadata_synced = 0` (indicates local changes not yet written to file)
6. Insert a row into `audit_log` for each changed field
7. Commit the transaction

**"Write to File" button:**

Separate from Save. This writes the current SQLite metadata back to the image file using ExifTool.

**Tauri command: `write_metadata_to_file`**

```rust
#[tauri::command]
fn write_metadata_to_file(image_id: i64, state: tauri::State<AppState>) -> Result<(), String>
```

Behavior:
1. Read the image record from SQLite
2. Build ExifTool arguments mapping our fields back to IPTC/XMP tags
3. Run `exiftool -overwrite_original <args> <file_path>`
4. Set `metadata_synced = 1`

---

### Step 2.4 — Search

**Add search components to shadcn:**
```bash
npx shadcn-svelte@latest add command
```

The shadcn `Command` component provides a command-palette-style search input that's perfect for this.

**File: `src/lib/components/SearchBar.svelte`**

Behavior:
- Text input in the top bar
- Debounced: waits 200ms after the user stops typing before searching
- Calls `search_images` Tauri command

**Tauri command: `search_images`**

```rust
#[tauri::command]
fn search_images(query: String, limit: u64, state: tauri::State<AppState>) -> Result<Vec<ImageRecord>, String>
```

Uses SQLite FTS5:
```sql
SELECT images.* FROM images_fts
JOIN images ON images.id = images_fts.rowid
WHERE images_fts MATCH ?
ORDER BY rank
LIMIT ?
```

FTS5 `MATCH` syntax supports prefix queries (`san franc*`), phrase queries (`"san francisco"`), and boolean operators.

When search is active, the grid shows search results instead of the full library. Clearing the search input returns to the full library view.

---

### Step 2.5 — Filtering

**File: `src/lib/components/FilterBar.svelte`**

Position: Below the top bar / search, above the grid.

**Filter controls:**
- **City** — select dropdown, populated from `SELECT DISTINCT city FROM images WHERE city IS NOT NULL ORDER BY city`
- **Year range** — two number inputs (start year, end year), filters on `date_start`/`date_end`
- **Photographer** — select dropdown, populated from `SELECT DISTINCT photographer FROM images WHERE photographer IS NOT NULL`
- **Archive collection** — select dropdown, populated from `SELECT name FROM collections WHERE source = 'archive' ORDER BY name`. Filters to images in that collection via the `collection_images` junction table.
- **Missing metadata** — checkbox: "Show only images missing metadata." When checked, filters to images where key fields (title, city, date_display) are NULL.
- **Clear filters** button

**Tauri command: `query_images_filtered`**

Extend the existing `query_images` command to accept optional filter parameters:

```rust
#[derive(serde::Deserialize)]
pub struct ImageQuery {
    pub offset: u64,
    pub limit: u64,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    // Filters:
    pub city: Option<String>,
    pub photographer: Option<String>,
    pub collection_id: Option<i64>,       // Filter by collection (archive or user)
    pub year_start: Option<i32>,
    pub year_end: Option<i32>,
    pub missing_metadata: Option<bool>,
    pub search_query: Option<String>,  // FTS5 query
}
```

Build the SQL query dynamically based on which filters are provided. Use parameterized queries for all values.

**Tauri command: `get_filter_options`**

```rust
#[tauri::command]
fn get_filter_options(state: tauri::State<AppState>) -> Result<FilterOptions, String>
```

Returns:
```rust
pub struct FilterOptions {
    pub cities: Vec<String>,
    pub photographers: Vec<String>,
    pub archive_collections: Vec<Collection>,  // Collections where source = 'archive'
    pub year_range: (Option<i32>, Option<i32>),  // min and max years in the catalog
}
```

Called on app load and cached in a Svelte store. Used to populate filter dropdowns.

**Filter store:**

**File: `src/lib/stores/filters.ts`**

```typescript
import { writable, derived } from 'svelte/store';

export interface FilterState {
  city: string | null;
  photographer: string | null;
  collectionId: number | null;  // Archive collection or user collection
  yearStart: number | null;
  yearEnd: number | null;
  missingMetadata: boolean;
  searchQuery: string | null;
}

export const filters = writable<FilterState>({
  city: null,
  photographer: null,
  collectionId: null,
  yearStart: null,
  yearEnd: null,
  missingMetadata: false,
  searchQuery: null,
});
```

The grid subscribes to this store and re-fetches when filters change.

---

### Step 2.6 — Recently Viewed

**Update the sidebar** to show the "Recently Viewed" section.

**Tauri command: `get_recently_viewed`**

```rust
#[tauri::command]
fn get_recently_viewed(state: tauri::State<AppState>) -> Result<Vec<ImageRecord>, String>
```

Returns the last 30 viewed images, ordered by `viewed_at DESC`.

**Sidebar display:**
- Show as a small list under "Recently Viewed" in the sidebar
- Show thumbnail (tiny, ~40px) + catalog number for each
- Clicking navigates to the detail view

---

### Step 2.7 — Phase 2 Docs & Testing

**Test:**
- Edit metadata on an image → save → verify changes in database
- Edit metadata → "Write to File" → verify changes in file (run `exiftool` manually to confirm)
- Search by catalog number, by city name, by keyword → verify results
- Apply filters → verify grid updates
- Chain search + filters → verify they work together
- Audit log records all changes correctly
- Recently viewed updates and displays correctly

**Update all docs** to reflect Phase 2 additions.

---

## Phase 3 — Collections

> **Goal**: Users can create static collections (albums) and smart collections (saved filters). Archive collections (auto-created from directory structure in Phase 1) are already browsable. This phase adds user-created collections on top.

### Step 3.1 — Static Collections CRUD

**Add shadcn components:**
```bash
npx shadcn-svelte@latest add dropdown-menu context-menu alert-dialog
```

**Note:** Archive collections (`source = 'archive'`) were auto-created during the directory scan in Phase 1. They already exist in the `collections` table and have their images linked via `collection_images`. Phase 3 adds the ability to create, edit, and delete **user** collections (`source = 'user'`), and provides the sidebar UI for browsing both types.

**Tauri commands:**

```rust
#[tauri::command]
fn create_collection(name: String, state: tauri::State<AppState>) -> Result<i64, String>
// Always creates with source = 'user'

#[tauri::command]
fn rename_collection(id: i64, name: String, state: tauri::State<AppState>) -> Result<(), String>
// Only allowed for user collections. Return error if source = 'archive'.

#[tauri::command]
fn delete_collection(id: i64, state: tauri::State<AppState>) -> Result<(), String>
// Only allowed for user collections. Return error if source = 'archive'.

#[tauri::command]
fn get_collections(state: tauri::State<AppState>) -> Result<Vec<Collection>, String>
// Returns all collections. The frontend uses the `source` field to render them in separate sidebar sections.

#[tauri::command]
fn add_to_collection(collection_id: i64, image_ids: Vec<i64>, state: tauri::State<AppState>) -> Result<(), String>
// Allowed for both user and archive collections (an image can belong to its archive collection AND user-created albums)

#[tauri::command]
fn remove_from_collection(collection_id: i64, image_ids: Vec<i64>, state: tauri::State<AppState>) -> Result<(), String>
// Only allowed for user collections. Images in archive collections are managed by the file system.

#[tauri::command]
fn get_collection_images(collection_id: i64, query: ImageQuery, state: tauri::State<AppState>) -> Result<ImageQueryResult, String>
// Works for both types
```

**Collection data type:**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub source: String,         // "user" or "archive"
    pub description: Option<String>,
    pub image_count: u64,       // Computed via COUNT on collection_images
    pub created_at: String,
}
```

**Sidebar:**

The sidebar displays collections in two distinct sections:

```
Archive Collections
  wnp27 (1,234)
  wnp83 (567)
  wnp105 (2,891)
  ...

Collections
  + New Collection
  My Favorites (42)
  Exhibition 2025 (18)
  ...
```

- **Archive Collections** section: lists all collections where `source = 'archive'`, sorted alphabetically. Non-editable — no rename/delete. Clicking shows that collection's images in the grid.
- **Collections** section: lists all collections where `source = 'user'`. "+" button to create. Right-click → rename, delete (with confirmation). Clicking shows collection images in the grid.
- Both sections show image count badges.

**Adding images to collections:**
- In the grid, right-click an image → "Add to Collection" → submenu lists user collections only (archive collections are managed by folder structure)
- In the detail view, a "Collections" section shows which collections this image belongs to (both archive and user), with an "Add to Collection" dropdown listing user collections

---

### Step 3.2 — Smart Collections

**Tauri commands:**

```rust
#[tauri::command]
fn create_smart_collection(name: String, filters: String, state: tauri::State<AppState>) -> Result<i64, String>
// `filters` is a JSON string representing the filter definition

#[tauri::command]
fn update_smart_collection(id: i64, name: Option<String>, filters: Option<String>, state: tauri::State<AppState>) -> Result<(), String>

#[tauri::command]
fn delete_smart_collection(id: i64, state: tauri::State<AppState>) -> Result<(), String>

#[tauri::command]
fn get_smart_collections(state: tauri::State<AppState>) -> Result<Vec<SmartCollection>, String>

#[tauri::command]
fn query_smart_collection(id: i64, query: ImageQuery, state: tauri::State<AppState>) -> Result<ImageQueryResult, String>
// Reads the filter JSON, builds the SQL query, returns results with pagination
```

**Smart collection filter format (JSON):**
```json
{
  "rules": [
    { "field": "city", "operator": "equals", "value": "San Francisco" },
    { "field": "date_start", "operator": "after", "value": "2015-01-01" },
    { "field": "photographer", "operator": "equals", "value": "John Doe" }
  ],
  "match": "all"  // "all" = AND, "any" = OR
}
```

**Supported operators by field type:**
- Text fields (city, state, photographer, etc.): `equals`, `not_equals`, `contains`, `is_empty`, `is_not_empty`
- Date fields (date_start, date_end): `before`, `after`, `between`
- Boolean (metadata_synced): `is_true`, `is_false`

**Smart collection creation UI:**

A dialog/modal with:
- Name input
- "Match: [All ▾] of the following rules"
- Rules list, each row: `[Field ▾] [Operator ▾] [Value input]` + delete button
- "+ Add Rule" button
- Preview count: "Matches 342 images" (run the query live as rules change)
- [Cancel] [Save]

**Sidebar:**
- Smart collections listed under "Smart Collections" heading with a different icon/styling than static collections
- Click to view, right-click to edit/delete

---

### Step 3.3 — Phase 3 Docs & Testing

**Test:**
- Create, rename, delete static collections
- Add/remove images from collections
- View collection contents in grid
- Create smart collections with various rule combinations
- Smart collection results update when metadata changes
- Edge cases: empty collections, collections with 10K+ images

**Update all docs.**

---

## Phase 4 — Export & Sharing

> **Goal**: Users can resize and share images. The app integrates with the organization's Laravel site for image requests.

### Step 4.1 — Image Resizing

**Add Rust dependency:**
```toml
# Already have `image` crate from Phase 1, no new dependency needed
```

**Tauri command: `export_image`**

```rust
#[derive(serde::Deserialize)]
pub struct ExportRequest {
    pub image_id: i64,
    pub resolution: String,  // "full", "high", "low"
}

#[tauri::command]
fn export_image(request: ExportRequest, state: tauri::State<AppState>) -> Result<String, String>
// Returns path to the exported file in a temp directory
```

Resolution tiers:
- `full` — copy the original file as-is
- `high` — resize to max 2048px on the long edge, JPEG quality 90
- `low` — resize to max 800px on the long edge, JPEG quality 80

Save exported files to a temp directory: `<app_data_dir>/exports/`

---

### Step 4.2 — Laravel API Integration

**Add Rust dependency:**
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
```

**Tauri commands:**

```rust
#[tauri::command]
async fn fetch_pending_requests(api_url: String, state: tauri::State<'_, AppState>) -> Result<Vec<ImageRequest>, String>

#[tauri::command]
async fn approve_request(request_id: i64, resolution: String, api_url: String, state: tauri::State<'_, AppState>) -> Result<(), String>

#[tauri::command]
async fn deny_request(request_id: i64, reason: String, api_url: String, state: tauri::State<'_, AppState>) -> Result<(), String>

#[tauri::command]
async fn upload_image_to_server(file_path: String, api_url: String, state: tauri::State<'_, AppState>) -> Result<String, String>
// Returns the server URL/path for the uploaded file
```

**`approve_request` flow:**
1. Find the requested image in the local database by `image_catalog_number`
2. Resize to the requested resolution using `export_image`
3. Upload the resized file to the Laravel server via multipart POST
4. Call the Laravel API to update request status to `fulfilled`
5. Laravel sends the email with download link (server-side concern)
6. Log in `usage_log`

**Store the Laravel API URL in `app_settings`:**
- Key: `laravel_api_url`
- Value: e.g., `https://archive.example.org/api`
- Configurable in a settings screen (add a simple settings view accessible from the sidebar)

---

### Step 4.3 — Image Request Queue UI

**File: `src/lib/components/ImageRequestQueue.svelte`**

**Layout:**
- Table/list of pending requests
- Columns: Requester Name, Email, Image (catalog #), Resolution, Purpose, Status, Actions
- Actions: Approve button, Deny button (with reason dialog)
- Polling: fetch new requests every 60 seconds + manual "Refresh" button
- Badge on sidebar: shows count of pending requests

**Ad-hoc sharing:**

In the detail view, add a "Share" button that opens a dialog:
- Email input
- Resolution selector (full, high, low)
- Optional purpose/note
- "Send" button → same resize + upload + email flow

---

### Step 4.4 — Phase 4 Docs & Testing

**Test:**
- Export images at all three resolution tiers
- Image request polling works (mock the Laravel API endpoint if the real one isn't ready)
- Approve flow: resize → upload → status update
- Deny flow: reason recorded, status updated
- Ad-hoc sharing works
- Usage log records all shares
- Error handling: network failures, missing images, invalid requests

**Update all docs.** Especially `docs/RUST-COMMANDS.md` with all new async commands and `docs/IMPORT.md` with notes on the Laravel API contract.

---

## Phase 5 — Keyboard Navigation & Polish

> **Goal**: The app feels like a polished native macOS tool. Full keyboard navigation, visual refinements, FSEvents file watching, dark mode.

### Step 5.1 — Keyboard Navigation

**Global keyboard shortcuts (register at the app level):**

| Key | Action |
|-----|--------|
| `Cmd+F` | Focus search bar |
| `Cmd+,` | Open settings |
| `Escape` | Back to previous view / close modal / clear search |

**Grid keyboard navigation:**
- Arrow keys move selection between images
- Enter opens the selected image's detail view
- Space bar could toggle image selection (for future multi-select)

**Detail view keyboard navigation:**
- `Escape` returns to grid
- `Left/Right` arrows navigate to previous/next image
- `Tab` moves through form fields
- `Cmd+S` saves metadata changes

**Implementation:** Use Svelte's `on:keydown` event on the window or specific containers. Consider a small keyboard shortcut manager utility that maps keys to actions.

---

### Step 5.2 — FSEvents File Watching

**Add Rust dependency:**
```toml
[dependencies]
notify = "6"
```

**Tauri command: `start_file_watcher`**

```rust
#[tauri::command]
fn start_file_watcher(path: String, app_handle: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String>
```

Behavior:
1. Create a `notify::RecommendedWatcher` watching the source directory
2. On file events (create, modify, delete):
   - **Create**: Insert new file into `images` table, generate thumbnail, extract metadata
   - **Delete**: Remove from `images` table (or mark as removed)
   - **Modify**: Re-extract metadata, regenerate thumbnail if needed
3. Emit Tauri events to the frontend so the grid can reactively update:
   ```rust
   app_handle.emit("file-changed", payload)?;
   ```
4. Frontend listens for these events and refreshes affected data

Start the watcher on app launch after the initial scan completes.

---

### Step 5.3 — Dark Mode

- Tailwind's `dark:` variants should already be in place from earlier phases
- Add a toggle in settings (or auto-detect via `prefers-color-scheme`)
- Store preference in `app_settings`
- shadcn-svelte supports dark mode natively via CSS class strategy (`class="dark"` on root)
- Test all components in both modes

---

### Step 5.4 — Visual Polish

- Review all components for consistent spacing, typography, and alignment
- Add loading states (skeleton placeholders) for grid items while thumbnails load
- Add empty states (no images, no results, no collections)
- Add error states (network failure, file not found)
- Add hover states on grid items (subtle border/shadow)
- Ensure the translucent sidebar effect looks good
- Add subtle transitions/animations (page transitions, modal open/close)
- Test with macOS system dark/light mode switching

---

### Step 5.5 — Phase 5 Docs & Testing

**Full app review:**
- Every keyboard shortcut works as documented
- File watcher detects new files, removes deleted files
- Dark mode works across all views
- No visual regressions
- Performance still smooth at 50K+ images

**Update all docs.** Add a keyboard shortcuts reference to `README.md` or a dedicated `docs/SHORTCUTS.md`.

---

## Phase 6 — Backup & Operations

> **Goal**: Automated catalog backups to protect against data loss.

### Step 6.1 — Backup Commands

**Tauri commands:**

```rust
#[tauri::command]
fn backup_catalog_local(destination: String, state: tauri::State<AppState>) -> Result<String, String>
// Copies the SQLite file to the destination path
// Returns the backup file path

#[tauri::command]
async fn backup_catalog_remote(api_url: String, state: tauri::State<'_, AppState>) -> Result<(), String>
// Uploads the SQLite file to the Laravel server
```

**SQLite backup approach:** Use SQLite's online backup API (`rusqlite` supports this via `backup()`) rather than raw file copy. This ensures a consistent backup even if the database is being written to.

---

### Step 6.2 — Automated Nightly Backup

**Implementation options:**
1. If the app is always running: use a Rust timer (e.g., `tokio::time::interval`) to trigger backup at a configured time (e.g., 2:00 AM)
2. If the app may not be running: trigger backup on app launch if the last backup was more than 24 hours ago

Store backup configuration in `app_settings`:
- `backup_local_path` — local directory for backups
- `backup_remote_enabled` — whether to also upload to server
- `last_backup_time` — ISO 8601 timestamp

**Backup on launch**: Always run a local backup when the app starts. This is fast (copying a SQLite file) and provides an automatic safety net.

**Backup rotation:** Keep the last 7 local backups, delete older ones. Name them with timestamps: `archive_manager_backup_2026-02-06T14-00-00.db`

---

### Step 6.3 — Backup UI

**Settings view addition:**
- "Backup" section
- Show last backup time
- "Backup Now" button (local + remote)
- Configure local backup path
- Toggle remote backup on/off
- Configure remote backup URL/endpoint

---

### Step 6.4 — Phase 6 Docs & Testing

**Test:**
- Manual backup creates a valid SQLite file that can be opened independently
- Backup-on-launch works
- Remote backup uploads successfully
- Backup rotation keeps only 7 files
- Restore: verify a backup file can replace the main database and the app works

**Update all docs.** Add backup/restore instructions to `docs/DEVELOPMENT.md`.

---

## Future Roadmap

> These features are validated as valuable but not included in the v1 build. They are documented here for future implementation reference.

### OCR for Text in Images (High Priority)

**Approach:** Bundle Tesseract OCR (or use a Rust OCR crate like `leptess`). During import or as a separate batch operation, run OCR on each image. Store extracted text in a new `ocr_text` column on the `images` table. Add this column to the FTS5 index so OCR text is searchable alongside other metadata.

**Schema change:**
```sql
ALTER TABLE images ADD COLUMN ocr_text TEXT;
-- Rebuild FTS5 index to include ocr_text
```

**Considerations:** OCR is CPU-intensive. For 50K images, this could take many hours. Should be an opt-in operation, not part of the standard import flow. Progress tracking is essential.

### Batch Metadata Editing (High Priority)

**Approach:** Multi-select in the grid (checkboxes or shift-click). Open a batch edit panel that shows only fields being changed. On save, apply the same changes to all selected images, with audit trail entries for each.

### Map View (High Priority)

**Approach:** Add Mapbox GL JS to the frontend. If images have GPS EXIF data, extract lat/lng during import and store in new columns. If not, geocode based on city/state/country using a geocoding API. Display images as clusters on the map. Click a cluster or pin to see images at that location.

**Schema change:**
```sql
ALTER TABLE images ADD COLUMN latitude REAL;
ALTER TABLE images ADD COLUMN longitude REAL;
```

### Keyword Hierarchy / Taxonomy (High Priority)

**Approach:** Replace the flat `keywords` JSON array with a normalized keyword system:
- `keywords` table: `id`, `name`, `parent_id` (self-referential for hierarchy)
- `image_keywords` junction table: `image_id`, `keyword_id`
- UI: tree-view keyword browser, autocomplete with hierarchy display

This is a significant schema change. Best done before the organization has invested heavily in the flat keyword system.

### Offline Request Queue (Medium Priority)

**Approach:** When an approve/share action fails due to network issues, store the pending action in a local `pending_actions` queue table. On app launch and periodically, retry pending actions. Show a badge/notification for queued actions.

### Tauri Auto-Updater (Medium Priority)

**Approach:** Configure Tauri's built-in updater plugin. Host release artifacts on GitHub Releases. The app checks for updates on launch and prompts the user to install.

### Batch Import with Convention-Based Metadata (Medium Priority)

**Approach:** A configuration UI where the admin defines rules mapping folder structure to metadata fields. Example: "Folder depth 1 = City, Folder depth 2 = Decade." Applied during the directory scan phase.

### Related Images / Grouping (Nice to Have)

**Approach:** `related_images` junction table with `image_id_a`, `image_id_b`, `relationship_type`. UI in the detail view showing related images. May be largely superseded by smart collections and map view.

---

## Appendix: Rust Module Structure

```
src-tauri/src/
  main.rs          — Tauri app entry point, registers all commands and state
  db.rs            — Database initialization, migrations, connection management
  models.rs        — Shared data types (ImageRecord, Collection, etc.)
  scanner.rs       — Directory scanning (walkdir)
  metadata.rs      — ExifTool integration, metadata extraction adapters
  thumbnails.rs    — Thumbnail generation
  queries.rs       — Image query/search/filter logic
  collections.rs   — Collection CRUD operations
  sharing.rs       — Export, upload, image request handling
  backup.rs        — Catalog backup operations
  watcher.rs       — FSEvents file watching
  settings.rs      — App settings key-value operations
```

Each module exposes Tauri commands that are registered in `main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .manage(AppState { db: Mutex::new(init_db()) })
        .invoke_handler(tauri::generate_handler![
            // Settings
            get_setting, set_setting, reset_catalog,
            // Scanner
            scan_directory, get_scan_stats,
            // Metadata
            extract_metadata_batch, extract_metadata_single,
            // Thumbnails
            extract_exif_thumbnails_batch, generate_full_thumbnails, generate_thumbnail_single,
            // Queries
            query_images, get_image, search_images, get_filter_options,
            // Collections
            create_collection, rename_collection, delete_collection,
            get_collections, add_to_collection, remove_from_collection,
            get_collection_images,
            // Smart Collections
            create_smart_collection, update_smart_collection,
            delete_smart_collection, get_smart_collections,
            query_smart_collection,
            // Metadata editing
            update_image_metadata, write_metadata_to_file,
            // Recently viewed
            log_image_view, get_recently_viewed,
            // Audit
            get_audit_log,
            // Sharing
            export_image, upload_image_to_server,
            fetch_pending_requests, approve_request, deny_request,
            // Usage
            get_usage_log,
            // Backup
            backup_catalog_local, backup_catalog_remote,
            // File watching
            start_file_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Appendix: Frontend File Structure (Complete)

```
src/
  routes/
    +layout.svelte           — App shell: sidebar, top bar, main area
    +page.svelte             — Main entry: routes to setup/import/library based on state
  lib/
    components/
      layout/
        Sidebar.svelte
        TopBar.svelte
        SettingsView.svelte
      browsing/
        Grid.svelte
        GridItem.svelte
        DetailView.svelte
        ImageZoom.svelte
        RecentlyViewed.svelte
      metadata/
        MetadataForm.svelte
        MetadataDiff.svelte   — Change confirmation dialog
        AuditLog.svelte
      search/
        SearchBar.svelte
        FilterBar.svelte
      collections/
        CollectionList.svelte
        SmartCollectionEditor.svelte
      sharing/
        ShareModal.svelte
        ImageRequestQueue.svelte
        ExportDialog.svelte
      setup/
        SetupScreen.svelte
        ImportProgress.svelte
    stores/
      navigation.ts
      filters.ts
      images.ts
      collections.ts
      requests.ts
      settings.ts
    commands/
      settings.ts
      images.ts
      metadata.ts
      collections.ts
      sharing.ts
      requests.ts
      backup.ts
    utils/
      format.ts              — File size formatting, date formatting, etc.
      keyboard.ts            — Keyboard shortcut manager
      thumbnailQueue.ts      — On-demand thumbnail generation queue
    ui/                      — shadcn-svelte components (added in Phase 2)
      button/
      input/
      dialog/
      ...
  app.css                    — Global styles, Tailwind imports
```
