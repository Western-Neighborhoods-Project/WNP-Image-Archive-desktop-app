-- Migration 007: convert file_modified from raw Unix-epoch seconds to the
-- SQLite datetime format used by every other timestamp column. Earlier scans
-- stored file_modified as a bare epoch string (e.g. "1752781234"), which the
-- detail view rendered to the user literally instead of as a date. Convert any
-- all-digit value in place; values already in datetime form (they contain '-')
-- are left untouched.
UPDATE images
   SET file_modified = datetime(CAST(file_modified AS INTEGER), 'unixepoch')
 WHERE file_modified IS NOT NULL
   AND file_modified GLOB '[0-9]*'
   AND file_modified NOT GLOB '*[^0-9]*';
