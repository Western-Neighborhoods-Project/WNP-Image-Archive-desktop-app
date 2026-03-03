# Database Schema

SQLite database at `~/Library/Application Support/org.wnp.imagearchive/archive_manager.db`.

Schema is embedded into the binary from `src-tauri/sql/schema.sql` at compile time and executed on every startup (all `CREATE TABLE IF NOT EXISTS`).

WAL mode and `PRAGMA foreign_keys=ON` are set at startup.

---

## Tables

### `images`

Core table. One row per image file.

```sql
CREATE TABLE IF NOT EXISTS images (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path           TEXT    NOT NULL UNIQUE,    -- absolute path on disk
    catalog_number      TEXT    NOT NULL,           -- filename without extension
    file_size           INTEGER,                    -- bytes
    file_modified       TEXT,                       -- ISO 8601
    title               TEXT,
    description         TEXT,
    city                TEXT,
    state               TEXT,
    country             TEXT,
    keywords            TEXT,                       -- JSON array: ["kw1","kw2"]
    date_display        TEXT,                       -- human-readable: "ca. 1920"
    date_start          TEXT,                       -- ISO 8601 for filtering
    date_end            TEXT,                       -- ISO 8601 for range queries
    photographer        TEXT,
    donor               TEXT,
    acquisition_date    TEXT,
    archival_collection TEXT,                       -- parent folder name at scan time
    usage_rights        TEXT,
    internal_notes      TEXT,
    thumbnail_path      TEXT,                       -- absolute path to cached thumbnail
    thumbnail_generated INTEGER DEFAULT 0,          -- 0=EXIF thumb, 1=full quality
    metadata_synced     INTEGER DEFAULT 0,          -- 0=local changes, 1=synced
    created_at          TEXT    DEFAULT (datetime('now')),
    updated_at          TEXT    DEFAULT (datetime('now'))
);
```

**Indexes:**
- `idx_images_catalog_number` — primary sort/lookup key
- `idx_images_city` — filter by city
- `idx_images_date_start` — sort/filter by date
- `idx_images_archival_collection` — filter by archive folder
- `idx_images_photographer` — filter by photographer
- `idx_images_thumbnail_generated` — find images needing thumbnail upgrade
- `idx_images_thumbnail_path` — find images without thumbnails

---

### `images_fts`

FTS5 virtual table for full-text search across key metadata fields.

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS images_fts USING fts5(
    catalog_number, title, description, city, keywords, photographer, internal_notes,
    content='images', content_rowid='id'
);
```

Kept in sync via `images_ai`, `images_ad`, `images_au` triggers on the `images` table.

**Usage:**
```sql
SELECT images.* FROM images_fts
JOIN images ON images.id = images_fts.rowid
WHERE images_fts MATCH 'san francisco'
ORDER BY rank;
```

---

### `collections`

User-created and archive-sourced image collections.

```sql
CREATE TABLE IF NOT EXISTS collections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'user',  -- 'user' | 'archive'
    description TEXT,
    created_at  TEXT    DEFAULT (datetime('now'))
);
```

`source = 'archive'` collections are auto-created during directory scan from parent folder names. `source = 'user'` collections are created in-app.

---

### `collection_images`

Junction table linking images to collections.

```sql
CREATE TABLE IF NOT EXISTS collection_images (
    collection_id INTEGER NOT NULL,
    image_id      INTEGER NOT NULL,
    added_at      TEXT    DEFAULT (datetime('now')),
    PRIMARY KEY (collection_id, image_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (image_id)      REFERENCES images(id)      ON DELETE CASCADE
);
```

---

### `smart_collections`

Saved filter definitions that dynamically query images.

```sql
CREATE TABLE IF NOT EXISTS smart_collections (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    filters    TEXT    NOT NULL,  -- JSON filter definition
    created_at TEXT    DEFAULT (datetime('now'))
);
```

**Filter JSON format:**
```json
{
  "rules": [
    { "field": "city", "operator": "equals", "value": "Denver" }
  ],
  "match": "all"
}
```

---

### `image_requests`

Pending image requests mirrored from the Laravel API.

```sql
CREATE TABLE IF NOT EXISTS image_requests (
    id                   INTEGER PRIMARY KEY,  -- matches Laravel ID
    image_catalog_number TEXT,
    requester_email      TEXT,
    requester_name       TEXT,
    requested_resolution TEXT,
    purpose              TEXT,
    status               TEXT DEFAULT 'pending',
    fetched_at           TEXT DEFAULT (datetime('now'))
);
```

---

### `audit_log`

Records all metadata field changes made in the app.

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id   INTEGER NOT NULL,
    field_name TEXT    NOT NULL,
    old_value  TEXT,
    new_value  TEXT,
    changed_by TEXT    DEFAULT 'local',
    changed_at TEXT    DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);
```

---

### `usage_log`

Records every time an image was shared or exported.

```sql
CREATE TABLE IF NOT EXISTS usage_log (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id         INTEGER NOT NULL,
    recipient_email  TEXT,
    recipient_name   TEXT,
    purpose          TEXT,
    resolution_sent  TEXT,
    request_id       INTEGER,
    shared_at        TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (image_id)   REFERENCES images(id) ON DELETE CASCADE,
    FOREIGN KEY (request_id) REFERENCES image_requests(id)
);
```

---

### `recently_viewed`

Tracks the last 30 images viewed in the detail view.

```sql
CREATE TABLE IF NOT EXISTS recently_viewed (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id  INTEGER NOT NULL UNIQUE,
    viewed_at TEXT    DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);
```

---

### `app_settings`

Key-value store for application preferences and configuration.

```sql
CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT
);
```

**Known keys:**
| Key | Description |
|---|---|
| `source_directory` | Absolute path to the image archive directory |
| `last_scan_time` | ISO 8601 timestamp of last full scan |
| `exiftool_path` | Path to exiftool binary (default: `exiftool`) |
| `laravel_api_url` | Base URL of the Laravel API (Phase 4) |
| `backup_local_path` | Local directory for catalog backups (Phase 6) |
| `backup_remote_enabled` | `"true"` or `"false"` (Phase 6) |
| `last_backup_time` | ISO 8601 timestamp of last backup (Phase 6) |

---

## Migration History

| Version | Changes |
|---|---|
| Phase 1 | Initial schema: all tables created |
