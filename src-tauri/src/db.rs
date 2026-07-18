use crate::auth::UserSession;
use crate::drive::DriveStatus;
use crate::watcher::WatcherHandle;
use rusqlite::{Connection, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Global app state holding the SQLite connection plus shared caches.
/// Wrapped in Mutexes for thread-safe access from Tauri commands and the
/// drive-monitor background poller.
pub struct AppState {
    pub db: Mutex<Connection>,
    /// Snapshot of the archive drive's current state. Updated continuously
    /// by `drive::spawn_drive_poller`; read by the drive commands.
    pub drive_state: Mutex<DriveStatus>,
    /// Active user session (Plan 10). None when no one is logged in.
    /// Lives in RAM; closing the app drops it.
    pub current_user: Mutex<Option<UserSession>>,
    /// Failed-login tracker keyed by lowercased username (Plan 11).
    /// Tuple is (failed_count, first_failure_at). Reset on success or
    /// after the lockout window expires.
    pub login_attempts: Mutex<HashMap<String, (u32, Instant)>>,
    /// Plan 12: file-system watcher handle. Populated in lib.rs setup
    /// once the Tauri app handle is available; None during the brief
    /// init window.
    pub watcher: Mutex<Option<WatcherHandle>>,
}

/// Initialize (or open) the SQLite database, running all migrations.
/// Called once at startup from lib.rs.
pub fn init_db() -> Result<Connection> {
    // We get the path from the environment at runtime — but at init time we
    // don't have the Tauri app handle yet. We store the DB in a well-known
    // location: the user's app support directory. For development we fall back
    // to the current directory.
    let db_path = get_db_path();
    let conn = Connection::open(&db_path)?;

    // The DB holds S3/API credentials in plaintext (see SECURITY.md). Restrict
    // it to the owner so another local user on a shared machine can't read it.
    restrict_permissions(&db_path, 0o600);

    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    run_migrations(&conn)?;

    Ok(conn)
}

/// Resolve the database file path. In production, uses the OS app data dir.
/// In development / testing, uses a local `archive_manager.db` file.
pub fn get_db_path() -> std::path::PathBuf {
    // Try the standard macOS app support directory
    if let Some(home) = dirs_next::data_dir() {
        let dir = home.join("org.wnp.imagearchive");
        let _ = std::fs::create_dir_all(&dir);
        // Owner-only so the DB and its WAL sidecars (which hold plaintext
        // credentials) aren't readable by other local users.
        restrict_permissions(&dir, 0o700);
        return dir.join("archive_manager.db");
    }
    // Fallback for development
    std::path::PathBuf::from("archive_manager.db")
}

/// Best-effort restrict a path to owner-only access. No-op on non-unix and on
/// filesystems that don't support unix permissions; failures are ignored
/// because this is defense in depth, not the primary protection.
#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}
#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path, _mode: u32) {}

/// Run all schema migrations. Creates the live schema, the migration
/// bookkeeping table, back-fills versions for any migrations that
/// existing DBs already had, then applies any pending migrations from
/// `sql/migrations/`.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../sql/schema.sql"))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    backfill_applied_migrations(conn)?;
    apply_pending_migrations(conn)?;
    Ok(())
}

/// Detect migrations whose effects are already present in the DB and
/// record them in `schema_migrations` so the runner doesn't try to apply
/// them again. Only relevant on first launch after the Plan 11 upgrade —
/// fresh installs hit `apply_pending_migrations` immediately.
fn backfill_applied_migrations(conn: &Connection) -> Result<()> {
    // 001 fts_trigram — applied if the FTS table's DDL mentions trigram.
    let fts_ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name = 'images_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    if fts_ddl.contains("trigram") {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (1)",
            [],
        )?;
    }

    // 002 osf_mirror_columns — applied if `format` column exists on images.
    let has_format: bool = conn
        .query_row(
            "SELECT 1 FROM pragma_table_info('images') WHERE name = 'format' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if has_format {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (2)",
            [],
        )?;
    }

    // 003 username_nocase — applied if the unique index exists.
    let has_idx: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE name = 'idx_users_username_nocase' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if has_idx {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (3)",
            [],
        )?;
    }

    // Migration 004 (source_directories) is idempotent at the Rust level
    // (apply_migration_004 uses add_column_if_missing + IF NOT EXISTS), so
    // we don't need to detect-and-mark it here. apply_pending_migrations
    // calls it the first time and records v4 then.
    Ok(())
}

fn apply_pending_migrations(conn: &Connection) -> Result<()> {
    // SQL-only migrations. include_str! takes a literal path, so they
    // listed explicitly. Migration 004 is the odd one out — it needs
    // conditional ALTER TABLE ADD COLUMN, which SQLite can't do in
    // pure SQL — so it lives in apply_migration_004 below.
    let migrations: &[(i64, &str, &str)] = &[
        (
            1,
            "001_fts_trigram",
            include_str!("../sql/migrations/001_fts_trigram.sql"),
        ),
        (
            2,
            "002_osf_mirror_columns",
            include_str!("../sql/migrations/002_osf_mirror_columns.sql"),
        ),
        (
            3,
            "003_username_nocase",
            include_str!("../sql/migrations/003_username_nocase.sql"),
        ),
    ];

    let applied: std::collections::HashSet<i64> = {
        let mut stmt = conn.prepare("SELECT version FROM schema_migrations")?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    for (version, name, sql) in migrations {
        if applied.contains(version) {
            continue;
        }
        log::info!("Applying migration {} ({})", version, name);
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            rusqlite::params![version],
        )?;
    }

    if !applied.contains(&4) {
        log::info!("Applying migration 4 (004_source_directories)");
        apply_migration_004(conn)?;
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            rusqlite::params![4i64],
        )?;
    }

    if !applied.contains(&5) {
        log::info!("Applying migration 5 (005_background_jobs)");
        apply_migration_005(conn)?;
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            rusqlite::params![5i64],
        )?;
    }

    // After migration 004 the table + columns exist, but existing rows
    // need the source_directory_id + relative_dir backfill — that requires
    // path arithmetic which is awkward in pure SQL.
    backfill_source_directory(conn)?;

    Ok(())
}

/// Idempotent application of migration 004. Splits across three
/// operations: (1) the source_directories table — IF NOT EXISTS makes
/// this safe; (2) the two new columns on images — SQLite doesn't have
/// `ALTER TABLE ADD COLUMN IF NOT EXISTS`, so we check pragma_table_info
/// first; (3) the new indexes — IF NOT EXISTS, fine to call repeatedly.
///
/// Safe to call on fresh installs (does the work once) and on existing
/// pre-Plan-12 installs (adds the missing schema then bookkeeps).
fn apply_migration_004(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS source_directories (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            path        TEXT    NOT NULL UNIQUE,
            label       TEXT    NOT NULL,
            created_at  TEXT    DEFAULT (datetime('now'))
        );
        ",
    )?;
    add_column_if_missing(
        conn,
        "images",
        "source_directory_id",
        "INTEGER REFERENCES source_directories(id) ON DELETE CASCADE",
    )?;
    add_column_if_missing(conn, "images", "relative_dir", "TEXT DEFAULT ''")?;
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_images_source_directory_id ON images(source_directory_id);
        CREATE INDEX IF NOT EXISTS idx_images_relative_dir        ON images(relative_dir);
        ",
    )?;
    Ok(())
}

/// Idempotent application of migration 005 (Plan 13). Adds the
/// thumbnail/metadata state-tracking columns used by the background
/// jobs worker. Backfills states for existing rows so already-extracted
/// data isn't reprocessed.
fn apply_migration_005(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "images", "thumbnail_state", "TEXT DEFAULT 'pending'")?;
    add_column_if_missing(conn, "images", "thumbnail_error", "TEXT")?;
    add_column_if_missing(conn, "images", "metadata_state", "TEXT DEFAULT 'pending'")?;
    add_column_if_missing(conn, "images", "metadata_error", "TEXT")?;

    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_images_thumbnail_state ON images(thumbnail_state);
        CREATE INDEX IF NOT EXISTS idx_images_metadata_state  ON images(metadata_state);
        ",
    )?;

    // Backfill: anything that already has a thumbnail on disk is 'done'.
    // Rows added before migration 005 default to 'pending' otherwise.
    conn.execute(
        "UPDATE images
         SET thumbnail_state = 'done'
         WHERE thumbnail_path IS NOT NULL
           AND (thumbnail_state IS NULL OR thumbnail_state = 'pending')",
        [],
    )?;
    // Anything that already has extracted metadata fields is 'done'.
    // Heuristic: any of title/description/city/photographer/date_start
    // populated implies a previous exiftool pass ran on this image.
    conn.execute(
        "UPDATE images
         SET metadata_state = 'done'
         WHERE (metadata_state IS NULL OR metadata_state = 'pending')
           AND (title IS NOT NULL
                OR description IS NOT NULL
                OR city IS NOT NULL
                OR photographer IS NOT NULL
                OR date_start IS NOT NULL)",
        [],
    )?;

    // Disk verification: an existing pre-Plan-13 install may have had
    // thumbnail files disappear (manual delete, disk hiccup, OS cleanup
    // of an external volume) without any state-tracking. Stat each
    // 'done' row's path; flip back to 'pending' if missing so the
    // worker regenerates instead of the grid showing broken images.
    let to_repair: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT id, thumbnail_path FROM images
             WHERE thumbnail_state = 'done' AND thumbnail_path IS NOT NULL",
        )?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        rows.into_iter()
            .filter(|(_, path)| !std::path::Path::new(path).exists())
            .map(|(id, _)| id)
            .collect()
    };
    if !to_repair.is_empty() {
        log::info!(
            "migration 005: {} thumbnails missing on disk, marking pending",
            to_repair.len()
        );
        for id in &to_repair {
            let _ = conn.execute(
                "UPDATE images
                 SET thumbnail_state = 'pending',
                     thumbnail_path = NULL,
                     thumbnail_generated = 0
                 WHERE id = ?1",
                rusqlite::params![id],
            );
        }
    }
    Ok(())
}

/// Add a column to a table only if it doesn't already exist. SQLite's
/// `ALTER TABLE ADD COLUMN` has no `IF NOT EXISTS`, so this checks
/// pragma_table_info first.
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    type_decl: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!(
        "SELECT 1 FROM pragma_table_info('{}') WHERE name = ?1 LIMIT 1",
        table
    ))?;
    let exists = stmt.query_row(rusqlite::params![column], |_| Ok(())).is_ok();
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column, type_decl
        ))?;
    }
    Ok(())
}

/// One-shot data migration for Plan 12: if the legacy `source_directory`
/// app_setting points to a real path and `source_directories` is empty,
/// promote it to the first row, then walk every image and fill in the
/// new `source_directory_id` + `relative_dir` columns based on path-prefix
/// matching. Idempotent: subsequent runs are no-ops once images are
/// populated.
fn backfill_source_directory(conn: &Connection) -> Result<()> {
    let any_unmigrated: bool = conn
        .query_row(
            "SELECT 1 FROM images WHERE source_directory_id IS NULL LIMIT 1",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if !any_unmigrated {
        return Ok(());
    }

    // Promote the legacy single-source setting if it's still around.
    let legacy: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'source_directory'",
            [],
            |r| r.get(0),
        )
        .ok();
    if let Some(path) = legacy {
        let trimmed = path.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            let label = std::path::Path::new(trimmed)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Archive")
                .to_string();
            // INSERT OR IGNORE so a partially-migrated DB won't error here.
            let _ = conn.execute(
                "INSERT OR IGNORE INTO source_directories (path, label) VALUES (?1, ?2)",
                rusqlite::params![trimmed, label],
            );
        }
    }

    // Build a vector of (id, path) for all known sources.
    let sources: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, path FROM source_directories ORDER BY length(path) DESC")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    if sources.is_empty() {
        return Ok(());
    }

    // For each unmigrated image, find the longest source path that's a
    // prefix of file_path and use that as its source. Compute relative_dir
    // = parent of file_path with the source prefix stripped.
    let mut stmt =
        conn.prepare("SELECT id, file_path FROM images WHERE source_directory_id IS NULL")?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (image_id, file_path) in rows {
        let Some((source_id, source_path)) = sources.iter().find(|(_id, path)| {
            // Match either the exact path or the path followed by a separator.
            let trimmed = path.trim_end_matches('/');
            file_path == *trimmed
                || file_path.starts_with(&format!("{}/", trimmed))
        }) else {
            log::warn!(
                "backfill_source_directory: no source matches file_path {} — leaving NULL",
                file_path
            );
            continue;
        };

        let trimmed_source = source_path.trim_end_matches('/');
        let stripped = file_path
            .strip_prefix(trimmed_source)
            .unwrap_or(&file_path)
            .trim_start_matches('/');
        let relative_dir = std::path::Path::new(stripped)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        conn.execute(
            "UPDATE images SET source_directory_id = ?1, relative_dir = ?2 WHERE id = ?3",
            rusqlite::params![source_id, relative_dir, image_id],
        )?;
    }

    Ok(())
}

/// Read a required setting from the app_settings key-value store. Returns
/// an Err if the key is missing, empty, or the query fails. Centralised
/// here so sharing.rs / opensf_sync.rs / settings.rs share one definition.
pub fn read_setting(conn: &Connection, key: &str) -> std::result::Result<String, String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .map_err(|_| format!("Setting '{}' is not configured", key))
    .and_then(|v| {
        if v.is_empty() {
            Err(format!("Setting '{}' is empty", key))
        } else {
            Ok(v)
        }
    })
}

/// Optional read variant. Returns None for missing/empty values instead of
/// an error string.
pub fn read_setting_opt(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|s| !s.is_empty())
}

/// Get the thumbnail cache directory, creating it if it doesn't exist.
pub fn get_thumbnail_cache_dir() -> std::path::PathBuf {
    let base = if let Some(home) = dirs_next::data_dir() {
        home.join("org.wnp.imagearchive")
    } else {
        std::path::PathBuf::from(".")
    };
    let dir = base.join("thumbnails");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Get the exports directory, creating it if it doesn't exist.
pub fn get_exports_dir() -> std::path::PathBuf {
    let base = if let Some(home) = dirs_next::data_dir() {
        home.join("org.wnp.imagearchive")
    } else {
        std::path::PathBuf::from(".")
    };
    let dir = base.join("exports");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
