# Rust Commands Reference

All Tauri commands exposed to the frontend. Each command is wrapped in a typed TypeScript function in `src/lib/commands/`.

---

## Settings (`settings.rs`)

### `get_setting`
```rust
fn get_setting(key: String, state: State<AppState>) -> Result<Option<String>, String>
```
Get a value from the app key-value store.

**Example:**
```typescript
import { getSetting } from '$lib/commands/settings';
const dir = await getSetting('source_directory'); // string | null
```

---

### `set_setting`
```rust
fn set_setting(key: String, value: String, state: State<AppState>) -> Result<(), String>
```
Set a value in the app key-value store.

**Known keys:**
- `source_directory` — path to the image archive directory
- `last_scan_time` — ISO 8601 timestamp of last scan
- `exiftool_path` — path to exiftool binary (default: `"exiftool"`)
- `laravel_api_url` — URL of the Laravel API (Phase 4)

---

### `reset_catalog`
```rust
fn reset_catalog(state: State<AppState>) -> Result<(), String>
```
Delete all catalog data (images, collections, thumbnails). Does not delete original image files. After reset, app returns to setup screen.

---

## Scanner (`scanner.rs`)

### `scan_directory`
```rust
fn scan_directory(path: String, state: State<AppState>) -> Result<ScanResult, String>
```
Recursively scan a directory for image files, insert new ones into the database, and auto-create archive collections from subdirectory names.

**Parameters:**
- `path` — absolute path to scan (e.g. `/Volumes/Archive/Images`)

**Returns `ScanResult`:**
```typescript
{ total_files: number; new_files: number; archive_collections_found: number; scan_duration_ms: number }
```

---

### `get_scan_stats`
```rust
fn get_scan_stats(state: State<AppState>) -> Result<ScanStats, String>
```
Return current catalog statistics.

**Returns `ScanStats`:**
```typescript
{ total_images: number; images_with_thumbnails: number; images_without_metadata: number }
```

---

## Metadata (`metadata.rs`)

### `extract_metadata_batch`
```rust
async fn extract_metadata_batch(directory: String, state: State<AppState>) -> Result<MetadataImportResult, String>
```
Run `exiftool -json -r -fast2` on a directory and update the database with extracted IPTC/XMP/EXIF metadata.

**Returns `MetadataImportResult`:**
```typescript
{ processed: number; updated: number; errors: number; duration_ms: number }
```

---

### `extract_metadata_single`
```rust
fn extract_metadata_single(file_path: String) -> Result<ExtractedMetadata, String>
```
Run ExifTool on a single file and return parsed metadata. Used for refreshing a single image.

---

## Thumbnails (`thumbnails.rs`)

### `extract_exif_thumbnails_batch`
```rust
async fn extract_exif_thumbnails_batch(state: State<AppState>) -> Result<ThumbnailResult, String>
```
Extract embedded EXIF thumbnails from all images that don't yet have a thumbnail. Falls back to full-quality generation for images without embedded thumbnails (TIFFs, PNGs).

**Returns `ThumbnailResult`:**
```typescript
{ extracted: number; fallback_generated: number; failed: number; duration_ms: number }
```

---

### `generate_full_thumbnails`
```rust
async fn generate_full_thumbnails(request: ThumbnailRequest, state: State<AppState>) -> Result<ThumbnailResult, String>
```
Generate 300×300px Lanczos3 thumbnails for a batch of image IDs. Called by the frontend as images scroll into view.

**Parameters:**
```typescript
{ request: { image_ids: number[] } }  // max ~20 IDs per batch
```

---

### `generate_thumbnail_single`
```rust
fn generate_thumbnail_single(image_id: i64, state: State<AppState>) -> Result<String, String>
```
Generate a full-quality thumbnail for a single image. Returns the thumbnail file path.

---

## Queries (`queries.rs`)

### `query_images`
```rust
fn query_images(query: ImageQuery, state: State<AppState>) -> Result<ImageQueryResult, String>
```
Paginated image query with optional sorting and filtering.

**Parameters (`ImageQuery`):**
```typescript
{
  offset: number;
  limit: number;
  sort_by?: 'catalog_number' | 'date_start' | 'created_at' | 'updated_at' | 'title' | 'city' | 'photographer' | 'file_size';
  sort_order?: 'asc' | 'desc';
  // Filters (all optional, null = no filter)
  city?: string | null;
  photographer?: string | null;
  collection_id?: number | null;
  year_start?: number | null;
  year_end?: number | null;
  missing_metadata?: boolean | null;
  search_query?: string | null;  // FTS5 MATCH query
}
```

**Returns `ImageQueryResult`:**
```typescript
{ images: ImageRecord[]; total_count: number }
```

---

### `get_image`
```rust
fn get_image(id: i64, state: State<AppState>) -> Result<ImageRecord, String>
```
Fetch a single image record by database ID.

---

## Collections (`collections.rs`)

### `get_collections`
```rust
fn get_collections(state: State<AppState>) -> Result<Vec<Collection>, String>
```
Return all collections (both `archive` and `user` source) with image counts.

**Returns `Collection[]`:**
```typescript
{ id: number; name: string; source: 'archive' | 'user'; description: string | null; image_count: number; created_at: string }
```

---

## Data Types

### `ImageRecord`
Full image record matching the `images` table. See `src/lib/commands/images.ts` for the TypeScript definition.

### `ImageQuery`
Paginated query parameters. See above.
