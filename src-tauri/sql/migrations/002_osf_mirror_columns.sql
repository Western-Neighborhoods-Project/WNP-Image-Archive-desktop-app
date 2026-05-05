-- Migration 002 (Plan 9): OpenSFHistory mirror columns.
-- These store the read-only synced data from the Laravel API. Source of
-- truth is OpenSFHistory; the desktop app shows them as locked inputs.
-- New installs get these via schema.sql; this migration retroactively
-- adds them to existing prod DBs.
ALTER TABLE images ADD COLUMN caption TEXT;
ALTER TABLE images ADD COLUMN dimensions TEXT;
ALTER TABLE images ADD COLUMN format TEXT;
ALTER TABLE images ADD COLUMN publisher TEXT;
ALTER TABLE images ADD COLUMN citation TEXT;
ALTER TABLE images ADD COLUMN download_permitted INTEGER;
ALTER TABLE images ADD COLUMN neighborhoods TEXT;
ALTER TABLE images ADD COLUMN photosets TEXT;
ALTER TABLE images ADD COLUMN osf_collections TEXT;
ALTER TABLE images ADD COLUMN osf_page_url TEXT;
ALTER TABLE images ADD COLUMN last_synced_at TEXT;
