-- Migration 001: Switch FTS5 tokenizer to trigram for substring search.
-- Enables partial catalog number searches (e.g. "5078" finds "DSCF5078",
-- "1234" finds "wnp84.1234"). Applied once; schema.sql already uses trigram
-- for new installs.

-- Drop old triggers first (they reference images_fts by name)
DROP TRIGGER IF EXISTS images_ai;
DROP TRIGGER IF EXISTS images_ad;
DROP TRIGGER IF EXISTS images_au;

-- Drop old FTS table and rebuild with trigram tokenizer
DROP TABLE IF EXISTS images_fts;

CREATE VIRTUAL TABLE images_fts USING fts5(
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

-- Recreate the sync triggers
CREATE TRIGGER images_ai AFTER INSERT ON images BEGIN
    INSERT INTO images_fts(rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES (new.id, new.catalog_number, new.title, new.description, new.city, new.keywords, new.photographer, new.internal_notes);
END;

CREATE TRIGGER images_ad AFTER DELETE ON images BEGIN
    INSERT INTO images_fts(images_fts, rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES ('delete', old.id, old.catalog_number, old.title, old.description, old.city, old.keywords, old.photographer, old.internal_notes);
END;

CREATE TRIGGER images_au AFTER UPDATE ON images BEGIN
    INSERT INTO images_fts(images_fts, rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES ('delete', old.id, old.catalog_number, old.title, old.description, old.city, old.keywords, old.photographer, old.internal_notes);
    INSERT INTO images_fts(rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
    VALUES (new.id, new.catalog_number, new.title, new.description, new.city, new.keywords, new.photographer, new.internal_notes);
END;

-- Repopulate the index from existing images
INSERT INTO images_fts(rowid, catalog_number, title, description, city, keywords, photographer, internal_notes)
SELECT id, catalog_number, title, description, city, keywords, photographer, internal_notes
FROM images;
