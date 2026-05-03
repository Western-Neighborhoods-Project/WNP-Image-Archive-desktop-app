use crate::auth::UserSession;
use crate::drive::DriveStatus;
use rusqlite::{Connection, Result};
use std::sync::Mutex;

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
        return dir.join("archive_manager.db");
    }
    // Fallback for development
    std::path::PathBuf::from("archive_manager.db")
}

/// Run all schema migrations. Creating tables with IF NOT EXISTS makes this
/// idempotent — safe to call on every startup.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../sql/schema.sql"))?;

    // Migration 001: switch FTS5 to trigram tokenizer for substring search.
    // Check the stored DDL — if it doesn't mention "trigram", the table was
    // created by the old schema and needs to be rebuilt.
    let fts_ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name = 'images_fts'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if !fts_ddl.is_empty() && !fts_ddl.contains("trigram") {
        conn.execute_batch(include_str!("../sql/migration_001_fts_trigram.sql"))?;
    }

    // Migration 003 (Plan 9): OpenSFHistory mirror columns. Populated by
    // `opensf_sync::sync_image_from_opensf` and read-only in the UI for
    // now. New DBs get these via schema.sql; existing prod DBs need ALTER.
    // Adding `format` here also retroactively fixes drive.rs's
    // `format_mix` query, which was failing silently because the column
    // didn't exist.
    add_column_if_missing(conn, "images", "caption", "TEXT")?;
    add_column_if_missing(conn, "images", "dimensions", "TEXT")?;
    add_column_if_missing(conn, "images", "format", "TEXT")?;
    add_column_if_missing(conn, "images", "publisher", "TEXT")?;
    add_column_if_missing(conn, "images", "citation", "TEXT")?;
    add_column_if_missing(conn, "images", "download_permitted", "INTEGER")?;
    add_column_if_missing(conn, "images", "neighborhoods", "TEXT")?;
    add_column_if_missing(conn, "images", "photosets", "TEXT")?;
    add_column_if_missing(conn, "images", "osf_collections", "TEXT")?;
    add_column_if_missing(conn, "images", "osf_page_url", "TEXT")?;
    add_column_if_missing(conn, "images", "last_synced_at", "TEXT")?;

    Ok(())
}

/// Add a column to a table only if it doesn't already exist. Idempotent.
/// Used by Plan 9's Migration 003 (and any future migration that needs
/// additive schema changes on populated production DBs).
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
