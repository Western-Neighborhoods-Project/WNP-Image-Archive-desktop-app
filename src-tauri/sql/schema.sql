-- Image Archive Manager — SQLite Schema
-- All tables are created with IF NOT EXISTS to make this idempotent.
-- This file is embedded into the binary at compile time via include_str!().

-- ============================================================
-- Core: Images
-- ============================================================

-- ============================================================
-- Core: Images
-- ============================================================
--
-- NOTE on Plan 12 additions (source_directories + the
-- source_directory_id / relative_dir columns + their indexes): those
-- live in db::apply_migration_004 rather than here. SQLite
-- `CREATE TABLE IF NOT EXISTS` doesn't add new columns to a pre-
-- existing table, so referencing those columns from this file's index
-- declarations would crash on existing installs that haven't run
-- migration 004 yet. The migration is idempotent (column-exists check
-- + IF NOT EXISTS indexes) and runs from apply_pending_migrations.

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
    archival_collection TEXT,              -- legacy: name of immediate parent subdirectory; superseded by relative_dir
    usage_rights        TEXT,
    internal_notes      TEXT,
    thumbnail_path      TEXT,
    thumbnail_generated INTEGER DEFAULT 0, -- 0 = EXIF thumbnail only, 1 = full quality generated
    metadata_synced     INTEGER DEFAULT 0, -- 0 = local changes not written to file, 1 = synced
    -- Plan 9: OpenSFHistory mirror columns. Source of truth lives on the
    -- OpenSFHistory site; these are populated by `opensf_sync` on detail
    -- view open and treated as read-only in the UI for now.
    caption             TEXT,
    dimensions          TEXT,
    format              TEXT,
    publisher           TEXT,
    citation            TEXT,
    download_permitted  INTEGER,
    neighborhoods       TEXT,             -- JSON array of slugs
    photosets           TEXT,             -- JSON object {id: title}
    osf_collections     TEXT,             -- JSON array of names (distinct from local `collections` table)
    osf_page_url        TEXT,
    last_synced_at      TEXT,             -- ISO timestamp; null until first sync
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
CREATE INDEX IF NOT EXISTS idx_images_created_at          ON images(created_at);
CREATE INDEX IF NOT EXISTS idx_images_updated_at          ON images(updated_at);

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
-- Users (Plan 10 — local user management)
-- ============================================================
-- Local username/password auth. Passwords are argon2id hashes.
-- Roles: 'admin' (full access) or 'editor' (no Settings).

CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    username        TEXT    NOT NULL UNIQUE,
    password_hash   TEXT    NOT NULL,
    role            TEXT    NOT NULL CHECK (role IN ('admin','editor')),
    created_at      TEXT    DEFAULT (datetime('now')),
    last_login_at   TEXT
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ============================================================
-- App Settings (key-value store)
-- ============================================================

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT
);
