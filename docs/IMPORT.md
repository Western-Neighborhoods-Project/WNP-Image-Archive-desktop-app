# Import System

## Overview

The import system runs in three sequential stages after the user selects an archive directory:

1. **Directory scan** (`scanner.rs`) — finds all image files, inserts into DB, creates archive collections
2. **Metadata extraction** (`metadata.rs`) — runs ExifTool, maps fields to DB schema
3. **Thumbnail extraction** (`thumbnails.rs`) — extracts embedded EXIF thumbnails; full-quality generation is lazy

---

## Stage 1: Directory Scan

`walkdir` recursively finds all files with supported extensions:
`.jpg`, `.jpeg`, `.tif`, `.tiff`, `.png`, `.gif`, `.bmp`, `.webp` (case-insensitive)

For each file, collects:
- `file_path` — absolute path
- `catalog_number` — filename without extension
- `file_size` — bytes
- `file_modified` — last modified timestamp
- `archival_collection` — parent directory name

All inserts use `INSERT OR IGNORE` inside a single transaction (dramatically faster than individual inserts for 50K+ files).

After scanning, archive collections are auto-created: for each unique parent folder name, a `collections` row is created (`source = 'archive'`) and all images in that folder are linked via `collection_images`.

---

## Stage 2: ExifTool Metadata Extraction

### ExifTool Adapter

The metadata system is pluggable. The current adapter (`exiftool_adapter`) is implemented in `metadata.rs`. Future adapters (CSV, JSON from external database export) would follow the same pattern, returning `ExtractedMetadata` structs.

### ExifTool Command

```bash
exiftool -json -r -fast2 -q /path/to/directory
```

- `-json` — output JSON array (one object per file)
- `-r` — recursive
- `-fast2` — skip MakerNotes (faster, adequate for catalog metadata)
- `-q` — quiet (suppress progress)

### Field Mapping

| ExifTool Field | Our Schema Field | Notes |
|---|---|---|
| `SourceFile` | `file_path` | Used to match the database row |
| `Title` / `ObjectName` | `title` | First non-empty wins |
| `Description` / `Caption-Abstract` / `ImageDescription` | `description` | First non-empty wins |
| `City` | `city` | IPTC |
| `Province-State` / `State` | `state` | IPTC |
| `Country-PrimaryLocationName` / `Country` | `country` | IPTC |
| `Keywords` | `keywords` | JSON array string; handles both array and comma-string |
| `DateTimeOriginal` / `CreateDate` / `DateCreated` | `date_start` | Normalized to `YYYY-MM-DD` |
| `Creator` / `Artist` / `By-line` / `Author` | `photographer` | First non-empty wins |
| `CopyrightNotice` / `Rights` / `Copyright` | `usage_rights` | First non-empty wins |

### Adding a New Adapter

To add a CSV or JSON import adapter for the organization's metadata export:

1. Create a function that parses the format and returns `Vec<ExtractedMetadata>`
2. `ExtractedMetadata` is defined in `models.rs`:
   ```rust
   pub struct ExtractedMetadata {
       pub file_path: String,
       pub title: Option<String>,
       pub description: Option<String>,
       // ... all fields
   }
   ```
3. Match records to the database via `file_path` or `catalog_number`
4. Use the same `UPDATE images SET ... WHERE file_path = ?` pattern

---

## Stage 3: Thumbnail Extraction

See [ARCHITECTURE.md — Thumbnail Caching Strategy](ARCHITECTURE.md) for the two-tier system details.

### ExifTool EXIF Thumbnail Command

```bash
exiftool -b -ThumbnailImage /path/to/image.jpg
```

Outputs raw JPEG bytes for the embedded thumbnail. Empty output means no embedded thumbnail exists.

### Thumbnail File Naming

Thumbnails are named by database ID: `<app_data_dir>/thumbnails/<id>.jpg`

This provides:
- Stable paths (no path encoding issues)
- Easy lookup without querying the DB
- Overwrite-in-place for tier-2 upgrades

### ExifTool Path Configuration

The ExifTool binary path defaults to `exiftool` (assumes it's on PATH). Override via:
```sql
INSERT INTO app_settings (key, value) VALUES ('exiftool_path', '/usr/local/bin/exiftool');
```

---

## Supported Image Formats

| Format | EXIF Thumbnail | Notes |
|---|---|---|
| JPEG (.jpg, .jpeg) | ✅ Usually present | Main format |
| TIFF (.tif, .tiff) | Rarely | Falls back to full-quality gen; can be large |
| PNG (.png) | ❌ Never | Always uses full-quality fallback |
| GIF (.gif) | ❌ Never | Always uses full-quality fallback |
| BMP (.bmp) | ❌ Never | Always uses full-quality fallback |
| WebP (.webp) | Varies | Depends on image |
