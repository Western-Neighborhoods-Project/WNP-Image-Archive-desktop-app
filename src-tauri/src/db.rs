use rusqlite::{Connection, Result};
use std::sync::Mutex;

/// Global app state holding the SQLite connection.
/// Wrapped in a Mutex for thread-safe access from multiple Tauri commands.
pub struct AppState {
    pub db: Mutex<Connection>,
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
