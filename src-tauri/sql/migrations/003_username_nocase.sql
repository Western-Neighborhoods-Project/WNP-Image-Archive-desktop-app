-- Migration 004 (Plan 11): case-insensitive username lookups.
-- The original schema declared `username TEXT UNIQUE NOT NULL` without
-- COLLATE NOCASE, allowing "Alice" and "alice" to coexist as separate
-- users. Add a unique index on LOWER(username) to enforce case-
-- insensitivity going forward without rewriting the table.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_nocase
  ON users(LOWER(username));
