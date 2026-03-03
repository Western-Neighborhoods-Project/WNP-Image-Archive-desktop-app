-- Image Archive Manager — SQLite Schema
-- All tables are created with IF NOT EXISTS to make this idempotent.
-- This file is embedded into the binary at compile time via include_str!().

-- ============================================================
-- Core: Images
-- ============================================================

CREATE TABLE IF NOT EXISTS images (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path           TEXT    NOT NULL UNIQUE,
    catalog_number      TEXT    NOT NULL,  -- archive's unique identifier (filename without extension)
    file_size           INTEGER,
    file_modified       TEXT,              -- ISO 8601 string
    title               TEXT,
    description         TEXT,
    city                TEXT,
    state               TEXT,
    country             TEXT,
    keywords            TEXT,              -- JSON array: ["keyword1", "keyword2"]
    date_display        TEXT,              -- human-readable: "ca. 1920", "Spring 1968"
    date_start          TEXT,              -- ISO 8601 date for filtering
    date_end            TEXT,              -- ISO 8601 date for range queries
    photographer        TEXT,
    donor               TEXT,
    acquisition_date    TEXT,
    archival_collection TEXT,              -- name of parent subdirectory at scan time
    usage_rights        TEXT,
    internal_notes      TEXT,
    thumbnail_path      TEXT,
    thumbnail_generated INTEGER DEFAULT 0, -- 0 = EXIF thumbnail only, 1 = full quality generated
    metadata_synced     INTEGER DEFAULT 0, -- 0 = local changes not written to file, 1 = synced
    created_at          TEXT    DEFAULT (datetime('now')),
    updated_at          TEXT    DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_images_catalog_number      ON images(catalog_number);
CREATE INDEX IF NOT EXISTS idx_images_city                ON images(city);
CREATE INDEX IF NOT EXISTS idx_images_date_start          ON images(date_start);
CREATE INDEX IF NOT EXISTS idx_images_archival_collection ON images(archival_collection);
CREATE INDEX IF NOT EXISTS idx_images_photographer        ON images(photographer);
CREATE INDEX IF NOT EXISTS idx_images_thumbnail_generated ON images(thumbnail_generated);
CREATE INDEX IF NOT EXISTS idx_images_thumbnail_path      ON images(thumbnail_path);

-- ============================================================
-- Full-Text Search (FTS5)
-- ============================================================

CREATE VIRTUAL TABLE IF NOT EXISTS images_fts USING fts5(
    catalog_number,
    title,
    description,
    city,
    keywords,
    photographer,
    internal_notes,
    content='images',
    content_rowid='id',
    tokenize='trigram'
);

-- Keep FTS index in sync with the images table
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

-- ============================================================
-- Collections
-- ============================================================

CREATE TABLE IF NOT EXISTS collections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'user', -- 'user' | 'archive'
    description TEXT,
    created_at  TEXT    DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_collections_source ON collections(source);

CREATE TABLE IF NOT EXISTS collection_images (
    collection_id INTEGER NOT NULL,
    image_id      INTEGER NOT NULL,
    added_at      TEXT    DEFAULT (datetime('now')),
    PRIMARY KEY (collection_id, image_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (image_id)      REFERENCES images(id)      ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_collection_images_image_id ON collection_images(image_id);

-- ============================================================
-- Smart Collections
-- ============================================================

CREATE TABLE IF NOT EXISTS smart_collections (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    filters    TEXT    NOT NULL, -- JSON filter definition
    created_at TEXT    DEFAULT (datetime('now'))
);

-- ============================================================
-- Image Requests (mirrored from Laravel API)
-- ============================================================

CREATE TABLE IF NOT EXISTS image_requests (
    id                   INTEGER PRIMARY KEY, -- matches Laravel record ID, NOT autoincrement
    image_catalog_number TEXT,
    requester_email      TEXT,
    requester_name       TEXT,
    requested_resolution TEXT,
    purpose              TEXT,
    status               TEXT DEFAULT 'pending',
    fetched_at           TEXT DEFAULT (datetime('now'))
);

-- ============================================================
-- Audit Log
-- ============================================================

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
CREATE INDEX IF NOT EXISTS idx_audit_log_image_id ON audit_log(image_id);

-- ============================================================
-- Usage Log
-- ============================================================

CREATE TABLE IF NOT EXISTS usage_log (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id         INTEGER NOT NULL,
    recipient_email  TEXT,
    recipient_name   TEXT,
    purpose          TEXT,
    resolution_sent  TEXT,
    request_id       INTEGER,
    shared_at        TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (image_id)   REFERENCES images(id)         ON DELETE CASCADE,
    FOREIGN KEY (request_id) REFERENCES image_requests(id)
);
CREATE INDEX IF NOT EXISTS idx_usage_log_image_id ON usage_log(image_id);

-- ============================================================
-- Recently Viewed
-- ============================================================

CREATE TABLE IF NOT EXISTS recently_viewed (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id  INTEGER NOT NULL UNIQUE,
    viewed_at TEXT    DEFAULT (datetime('now')),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);

-- ============================================================
-- App Settings (key-value store)
-- ============================================================

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT
);
