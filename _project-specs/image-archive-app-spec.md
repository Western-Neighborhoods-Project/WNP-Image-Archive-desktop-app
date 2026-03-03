# Image Archive Manager — Project Spec

## Overview

A macOS desktop application for managing a history/archive organization's image archive of 50,000+ images stored on an external hard drive. Replaces Adobe Lightroom, which is overkill — the organization does not edit images, only manages metadata, searches, and shares them. The app runs locally on 1–3 machines with the external drive always connected.

---

## Tech Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| App shell | **Tauri** | Compiles to native `.app` bundle. No terminal or server needed for end users. Distributed as `.app` or `.dmg`. |
| Frontend | **Svelte** (SvelteKit in static mode) | Simpler than React, small bundles, intuitive reactivity. Maintainers have web dev skills. |
| UI components | **shadcn-svelte** + **Tailwind CSS** | Copy-into-project component library built on Bits UI. Provides accessible, polished primitives (inputs, dialogs, dropdowns, command palette, etc.) with minimal custom CSS needed. |
| Backend logic | **Rust** (Tauri commands) | Handles file I/O, database queries, process spawning, image resizing. ~10–15 functions, ~500–800 lines for v1. |
| Database | **SQLite** | Local catalog database. Handles 50K rows trivially. Full-text search via FTS5. |
| Metadata I/O | **ExifTool** (bundled binary) | Industry-standard tool for reading/writing EXIF, IPTC, XMP. Called as subprocess from Rust. |
| Image processing | **Rust `image` crate** | Thumbnail generation and export resizing. |
| File watching | **`notify` crate** (Rust) | Wraps macOS FSEvents for live file system monitoring. |

---

## Architecture

### Mental Model

The Rust backend functions as a local API layer. The Svelte frontend calls Rust functions via Tauri's `invoke()` command system — conceptually identical to calling HTTP endpoints, just local RPC.

### Metadata Strategy: Hybrid

- **SQLite** is the working catalog for speed (search, filter, browse).
- **ExifTool** writes metadata back to image files on explicit save.
- This gives fast database-driven UX with portable embedded metadata.

### Image Performance Strategy

- **Thumbnail cache**: Two-tier strategy. During import, EXIF thumbnails are extracted from images (fast byte-copy, ~5-10KB each). Full-quality thumbnails (300px, ~15-30KB) are generated on demand as images scroll into view in the grid. The `thumbnail_generated` flag tracks which tier each image has. This keeps import fast (minutes, not hours) and disk usage low (only images actually browsed get full thumbnails).
- **Virtual scrolling**: Only visible grid rows are rendered. Use `tanstack-virtual` (Svelte adapter) or similar.
- **Initial import**: One-time full scan + metadata extraction + thumbnail generation. Can take 30min–hours. Does not need to be resumable for v1.
- **Subsequent launches**: Directory diff — walk file tree, compare paths/modification dates against SQLite. Fast (10–30 seconds for 50K files).
- **Live watching**: FSEvents via `notify` crate detects new/modified/deleted files while app is running.

---

## Core Features

### 1. Image Browsing

- Grid view with virtual scrolling (50K+ images)
- Thumbnails loaded via Tauri asset protocol (`asset://localhost/...`)
- List view option
- Sort by: date, recency, catalog number
- Click to open detail view
- **Recently viewed**: Persistent list of last 20–30 images viewed, stored in SQLite. Accessible from sidebar.

### 2. Metadata Viewing & Editing

- Detail view shows larger image + metadata form
- **Image preview zoom**: Click or scroll to zoom into full-resolution image in detail view. Full-res loaded on demand only when zooming.
- **Before/after diffing on save**: Show what changed ("City: *empty* → San Francisco") as a confirmation step before writing. Feeds into audit trail.
- **Configurable field visibility**: Admin can set which fields show by default vs. collapsed under "Advanced." Keeps daily workflow clean as field count grows.
- Two-way Svelte bindings for form editing
- Save writes to SQLite immediately; optional "write to file" action pushes to image via ExifTool

**Standard metadata fields:** title, description, city, state, country, keywords, date, photographer, catalog number, and other IPTC/XMP fields (TBD based on organization's metadata export)

**Archival metadata fields:**
- `date_display` — human-readable date text (e.g., "ca. 1920", "Spring 1968", "1940s"). What users see.
- `date_start` / `date_end` — normalized date range for filtering and sorting. Supports approximate and partial dates.
- `donor` — who donated or provided the image
- `acquisition_date` — when it was accessioned into the archive
- `archival_collection` — the archival collection/series it belongs to (distinct from app collections)
- `usage_rights` — licensing/usage restrictions (e.g., editorial only, public domain, no commercial use)
- `internal_notes` — free-text working notes, never written to file or shared externally ("Need to verify date," "Donor says more prints available")

### 3. Search

- Search bar with debounced input (~200ms)
- SQLite FTS5 full-text search across metadata fields (including internal notes)
- Primary lookup by catalog number

### 4. Filtering

- Filter bar: City, Year/date range, photographer, keywords, archival collection
- "Missing metadata" filter (e.g., `WHERE city IS NULL`)
- Filters trigger Rust backend queries, grid reactively updates

### 5. Collections

**Archive collections** (from directory structure):
- Auto-created during directory scan from subdirectory names (e.g., "wnp27", "wnp83")
- `collections` table with `source = 'archive'`, linked via `collection_images` junction table
- Browsable in sidebar under "Archive Collections" section
- Non-editable (managed by file system structure)

**User collections** (photo albums):
- `collections` table with `source = 'user'`, linked via `collection_images` junction table
- Add images via right-click or drag-and-drop
- CRUD management in sidebar under "Collections" section

**Smart collections** (saved filters):
- `smart_collections` table with JSON filter definition column
- Example: `{"city": "San Francisco", "year_range": [2015, 2020]}`
- Query executed on open — always reflects current data
- Creation UI: rows of Field / Operator / Value with "Add rule" button

### 6. Export & Sharing

**Resize tiers:**
- Full resolution (original file)
- High-res (e.g., 2048px on long edge)
- Low-res (e.g., 800px, web-optimized)

**Image Request Flow (primary sharing mechanism):**

The organization's public Laravel site allows external users to request images. The Tauri app integrates with this:

1. Public user submits an image use request on the Laravel site
2. Laravel stores the request with status `pending` and exposes it via API endpoint (`/api/image-requests`)
3. Tauri app polls this endpoint on an interval (every 30–60 seconds) and displays pending requests in an admin queue
4. Admin reviews and approves/denies requests in Tauri
5. On approval: Tauri resizes the image locally to the requested resolution → uploads to Laravel server
6. Laravel generates a hashed/expiring download URL → sends email to the requester with the link
7. Request status updated to `fulfilled`

**Request statuses:** `pending` → `approved` → `fulfilled` (or `denied` at step 4, `expired` after time limit)

A manual "Check for requests" button supplements polling for immediate checks.

**Ad-hoc sharing** (non-request-based) also supported: admin can share any image directly from the detail view by entering an email and picking a resolution, triggering the same resize → upload → email flow.

Implementation: Rust `reqwest` crate for HTTP communication with the Laravel API. Laravel handles email sending, URL generation, and citation generation.

### 7. Keyboard Navigation

- Arrow keys to move between images in grid
- Enter to open detail view, Escape to go back
- Tab through metadata fields in detail view
- Keyboard shortcuts for common actions (save, search focus, etc.)
- Built on shadcn-svelte's accessibility foundations

### 8. Activity Log / Audit Trail

- Append-only `audit_log` table in SQLite
- Records: who changed what field, old value, new value, timestamp
- Viewable per-image ("history of changes to this image") and globally ("all recent changes")
- Critical for multi-user environment where metadata overwrites need to be traceable and reversible

### 9. Copyright & Usage Tracking

- `usage_rights` field on each image (editorial only, public domain, no commercial use, etc.)
- `usage_log` table records every time an image is shared: image ID, recipient, date, purpose, resolution sent
- Provides institutional memory: "This image has been shared 14 times for these purposes"
- Feeds from the image request approval flow automatically

### 10. Catalog Backup

- **Automatic nightly backup**: SQLite database file copied to a local backup location AND uploaded to a designated location on the organization's remote server
- **Manual backup**: "Export catalog backup" button for on-demand backups
- SQLite is a single file, so backup is essentially a file copy
- Backup on app launch as additional safety net

---

## Frontend Architecture

### Views

- **Sidebar** (persistent): Library, Recently Viewed, Archive Collections (auto-populated from directory structure), User Collections, Smart Collections, "Needs Metadata" shortcut, Image Requests (with pending count badge)
- **Main content area**: Grid view, Detail view, Collection view, Image Request Queue
- **Top bar**: Search input, filter controls, sort, view toggle

### State Management

Svelte writable stores — no external state management library needed:

- `currentView` — navigation state (`library`, `collection`, `detail`)
- `currentFilters` — active filter/search criteria
- `currentResults` — derived from filter state, updated reactively
- `selectedImages` — for bulk operations
- `collections` — loaded on app start

### Routing

No URL-based router. Simple reactive store tracking current view, current image ID, current collection ID. Sidebar and main area subscribe to the same stores.

### Component Structure

```
src/
  lib/
    components/
      Sidebar.svelte
      Grid.svelte
      GridItem.svelte
      DetailView.svelte
      MetadataForm.svelte
      FilterBar.svelte
      SearchBar.svelte
      ShareModal.svelte
      CollectionList.svelte
      ImageRequestQueue.svelte
    stores/
      navigation.ts
      filters.ts
      images.ts
      collections.ts
      requests.ts
    commands/
      images.ts          # Tauri invoke wrappers
      metadata.ts
      collections.ts
      sharing.ts
      requests.ts
    ui/                  # shadcn-svelte components (copied in)
      button/
      dialog/
      input/
      dropdown-menu/
      command/
      ...
  App.svelte
  main.ts
```

### Styling

- **shadcn-svelte** for all UI primitives (buttons, inputs, dialogs, dropdowns, popovers, command palette, data tables)
- **Tailwind CSS** for layout and custom styling — minimal hand-written CSS needed
- macOS-native aesthetic via shadcn's clean defaults + Tailwind customization
- System font stack (`-apple-system, BlinkMacSystemFont`)
- Dark mode via Tailwind's `dark:` variant and `prefers-color-scheme`
- Translucent sidebar via `backdrop-filter: blur()`
- Components are copied into the project (not an npm dependency), so fully customizable

---

## Rust Backend Commands

### File System
- `scan_directory(path)` — recursive walk, returns file entries (path, size, mod date). Extracts catalog number from filename and archival collection from parent subdirectory. Auto-creates archive collections. Uses `walkdir` crate.
- `start_file_watcher(path)` — FSEvents watcher via `notify` crate. Emits events on file add/modify/delete.

### Metadata
- `read_metadata(path)` — shells out to `exiftool -json`, parses response.
- `write_metadata(path, fields)` — shells out to `exiftool` with field arguments.
- `batch_import(directory)` — `exiftool -json -r`, streams output, inserts into SQLite.

### Database
- `query_images(filters, sort, offset, limit)` — parameterized SQL query.
- `search_images(query)` — FTS5 full-text search.
- `get_image(id)` — single image with all metadata.
- `update_image_metadata(id, fields)` — update SQLite row.
- `create_collection(name)` / `delete_collection(id)` — CRUD.
- `add_to_collection(collection_id, image_ids)` / `remove_from_collection(...)`.
- `create_smart_collection(name, filters_json)` / `query_smart_collection(id)`.

### Image Processing
- `extract_exif_thumbnails_batch()` — extract embedded EXIF thumbnails during import (fast).
- `generate_full_thumbnails(image_ids)` — generate full-quality 300px thumbnails on demand for a batch of images. Overwrites EXIF version. Sets `thumbnail_generated = 1`.
- `generate_thumbnail_single(image_id)` — generate for a single image (used by file watcher for new files).
- `export_image(source_path, resolution_tier)` — resize to target dimensions, return temp path.

### Sharing
- `upload_and_share(image_path, email, resolution)` — resize, upload via `reqwest` to org server, trigger email.

### Image Requests
- `fetch_pending_requests()` — polls Laravel API for pending image requests.
- `approve_request(request_id, resolution)` — triggers resize → upload → status update flow.
- `deny_request(request_id, reason)` — updates request status to denied via API.

### Audit & Usage
- `get_audit_log(image_id)` — returns change history for a specific image.
- `get_usage_log(image_id)` — returns sharing history for a specific image.
- `log_view(image_id)` — records image view in `recently_viewed`, prunes oldest entries.
- `get_recently_viewed()` — returns last 20–30 viewed images.

### Backup
- `backup_catalog(local_path)` — copies SQLite file to local backup location.
- `backup_catalog_remote()` — uploads SQLite file to org server via `reqwest`.
- `schedule_nightly_backup()` — sets up recurring backup (local + remote).

---

## Data Model (SQLite)

### `images`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | Auto-increment (internal database ID) |
| file_path | TEXT | Full path on external drive |
| catalog_number | TEXT | The archive's unique identifier for this image (derived from filename without extension) |
| file_size | INTEGER | Bytes |
| file_modified | DATETIME | File system mod date |
| title | TEXT | |
| description | TEXT | |
| city | TEXT | |
| state | TEXT | |
| country | TEXT | |
| keywords | TEXT | Comma-separated or JSON array |
| date_display | TEXT | Human-readable date ("ca. 1920", "Spring 1968") |
| date_start | DATE | Normalized start date for filtering/sorting |
| date_end | DATE | Normalized end date (for ranges, approximate dates) |
| photographer | TEXT | |
| donor | TEXT | Who donated/provided the image |
| acquisition_date | DATE | When accessioned into the archive |
| archival_collection | TEXT | Archival collection/series name (auto-populated from parent subdirectory during scan) |
| usage_rights | TEXT | Licensing restrictions (editorial only, public domain, etc.) |
| internal_notes | TEXT | Working notes, never shared externally |
| thumbnail_path | TEXT | Path to cached thumbnail |
| thumbnail_generated | INTEGER | 0 = EXIF thumbnail only, 1 = full quality generated |
| metadata_synced | BOOLEAN | Whether SQLite matches file |
| created_at | DATETIME | Row creation |
| updated_at | DATETIME | Last metadata edit |

*Additional fields TBD based on the organization's metadata database export.*

### `collections`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | |
| name | TEXT | Collection name (for archive: subdirectory name like "wnp27") |
| source | TEXT | `'user'` = created in app, `'archive'` = auto-created from directory structure |
| description | TEXT | Optional description |
| created_at | DATETIME | |

### `collection_images`
| Column | Type |
|--------|------|
| collection_id | INTEGER FK |
| image_id | INTEGER FK |

### `smart_collections`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | |
| name | TEXT | |
| filters | TEXT | JSON filter definition |
| created_at | DATETIME | |

### `image_requests` (mirrored from Laravel API)
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | Matches Laravel record ID |
| image_catalog_number | TEXT | Catalog number of the requested image |
| requester_email | TEXT | |
| requester_name | TEXT | |
| requested_resolution | TEXT | full, high, low |
| purpose | TEXT | Usage description from requester |
| status | TEXT | pending, approved, denied, fulfilled, expired |
| fetched_at | DATETIME | When Tauri last synced this record |

*Note: This is a local cache of the Laravel data for display in the admin queue. Laravel remains the source of truth.*

### `audit_log`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | |
| image_id | INTEGER FK | |
| field_name | TEXT | Which field was changed |
| old_value | TEXT | Previous value |
| new_value | TEXT | New value |
| changed_by | TEXT | User/machine identifier |
| changed_at | DATETIME | |

### `usage_log`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | |
| image_id | INTEGER FK | |
| recipient_email | TEXT | |
| recipient_name | TEXT | |
| purpose | TEXT | |
| resolution_sent | TEXT | full, high, low |
| request_id | INTEGER FK | Links to image_requests if applicable, NULL for ad-hoc shares |
| shared_at | DATETIME | |

### `recently_viewed`
| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PK | |
| image_id | INTEGER FK | |
| viewed_at | DATETIME | |

*Capped at ~30 rows, oldest pruned on insert.*

### Indexes
- `images.catalog_number` — primary lookup path
- `collections.source` — filtering by collection type
- `audit_log.image_id` — per-image history lookups
- `usage_log.image_id` — per-image usage history
- FTS5 virtual table on catalog_number, title, description, city, keywords, photographer, internal_notes

---

## Build Phases

### Phase 1 — Core Browsing & Import
Tauri app shell, directory scanning (with automatic archive collection detection from subdirectories), ExifTool metadata extraction, SQLite population (including archival fields), two-tier thumbnail system (EXIF extraction on import, full-quality on demand), virtual-scrolled thumbnail grid, archive collections browsable in sidebar, recently viewed. **Proves the architecture at scale and provides immediate browsable access to the archive.**

### Phase 2 — Metadata Editing & Search
Detail view with image preview zoom, metadata form with before/after diffing on save (to SQLite + optional file write), audit trail logging, search bar, filtering (including archival collection), "missing metadata" filter, configurable field visibility.

### Phase 3 — Collections
User-created static collections (CRUD, add/remove images), smart collections (define filters, save, execute). Archive collections from directory structure are already browsable from Phase 1; this phase adds user collection management alongside them.

### Phase 4 — Export & Sharing
Resize generation, image request queue (polling Laravel API, approve/deny flow), ad-hoc sharing, usage log tracking, citation generation (Laravel side).

### Phase 5 — Keyboard Navigation & Polish
Full keyboard navigation, macOS styling refinements, drag-and-drop, FSEvents live watching, dark mode, edge cases.

### Phase 6 — Backup & Operations
Automatic nightly catalog backup (local + remote server), manual backup, backup on launch.

---

## Future Roadmap

Features validated as valuable but deferred beyond v1. Roughly prioritized.

### High Priority

**OCR for text in images.** Tesseract (open-source OCR engine) called as subprocess during import, like ExifTool. Extracted text stored in a column and included in FTS5 index. Enables searching for text visible in photographs of documents, signs, newspapers, maps, letters. Value increases with archive size. *Significant for a history archive.*

**Batch metadata editing.** Select multiple images → edit shared fields (city, photographer, date, keywords) in one operation. Essential for tagging images from the same event or location.

**Map view.** Browse images geographically using Mapbox (already used on the public site). Images with GPS coordinates or city/location metadata plotted on a map. Click a pin to see images from that location.

**Keyword hierarchy / taxonomy.** Hierarchical keywords instead of flat tags — e.g., "Location > California > San Francisco > Mission District." Requires schema consideration (nested set model or materialized path). Worth designing before the flat keyword list gets too large to migrate.

### Medium Priority

**Lightroom ongoing sync.** Beyond one-time import, periodic sync if the organization continues using Lightroom in parallel during transition.

**Offline request queue.** If internet is down when admin approves an image request, queue the action and retry when connectivity returns. Makes the app more robust for daily use.

**Batch import with convention-based metadata.** Parse folder structure or naming conventions during import to pre-populate metadata fields (e.g., folder level 1 = City, folder level 2 = Decade). Saves manual entry on bulk imports.

**Tauri auto-updater.** Built-in update mechanism — app checks a URL (e.g., GitHub Releases) for new versions, downloads and installs updates automatically. Useful as update frequency increases.

### Nice to Have

**Related images / grouping.** Explicitly link related images (same subject from different angles, same location across decades). Lightweight junction table. May be largely addressed by location-based browsing and smart collections.

**Before/after metadata diffing in audit log viewer.** Visual diff view showing full change history for an image with old/new values side by side.

---

## Distribution

- Tauri compiles to native macOS `.app` bundle
- Distribute as `.app` file or `.dmg` installer
- Optional: Tauri built-in updater for future versions
- Installed on 1–3 machines by a developer (no self-service install needed)

---

## Open Questions

- **Exact metadata fields**: TBD — will be determined from the organization's metadata database export.
- **Thumbnail size/quality**: Starting small (~300px for generated). May add a configurable slider later.
- **Smart collection operators**: TBD — will determine which filter combinations (equals, contains, date ranges, is empty) matter most.

## Resolved Decisions

- **Bulk metadata editing**: Not needed for v1 (roadmap item).
- **UI component library**: shadcn-svelte with Tailwind CSS — modern aesthetic with minimal custom styling effort.
- **Sharing flow**: Image request queue integrated with existing Laravel site API. Polling-based (30–60s interval) with manual check button. Request statuses tracked across both systems.
- **Server infrastructure**: Existing Laravel app handles upload storage, hashed URL generation, email delivery, and citation generation. No new server needed.
- **Watermark generation**: Handled on the public Laravel site. Tauri app shares non-watermarked versions.
- **Duplicate detection**: Not prioritized — the archive organization maintains good file hygiene.
- **Date handling**: Dual-field approach — `date_display` for human-readable text, `date_start`/`date_end` for normalized filtering. Supports archival date formats (circa, decades, ranges, seasons).
- **Lightroom import**: Not applicable — org uses a separate database for metadata. Import system built with pluggable adapter pattern; field mapping TBD once data export is provided.
- **Directory change**: Full catalog reset approach — changing source directory clears all indexed data, thumbnails, collections, and logs. Original image files are never affected. App returns to setup screen for re-import.
- **Thumbnail strategy**: Two-tier — EXIF thumbnails extracted during import (fast), full-quality generated on demand as images are browsed. Tracked via `thumbnail_generated` flag on each image.
- **Image identifiers**: `id` (auto-increment) is the internal database key for joins and foreign keys. `catalog_number` (derived from filename without extension) is the archive's stable external identifier. Both are kept because they serve different purposes and can diverge (e.g., on re-import).
- **Archive collections from directories**: Subdirectories on the external drive represent named archive collections (e.g., "wnp27"). These are auto-detected during directory scan and stored as collections with `source = 'archive'`. Browsable in the sidebar, non-editable by users. User-created collections use `source = 'user'` in the same table.
