use crate::db::AppState;
use crate::models::Collection;

fn row_to_collection(row: &rusqlite::Row) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        source: row.get(2)?,
        description: row.get(3)?,
        created_at: row.get(4)?,
        image_count: row.get::<_, i64>(5)? as u64,
    })
}

const COLLECTION_SELECT: &str = "
    SELECT c.id, c.name, c.source, c.description, c.created_at,
           COUNT(ci.image_id) as image_count
    FROM collections c
    LEFT JOIN collection_images ci ON ci.collection_id = c.id
    GROUP BY c.id
";

/// Return all collections with image counts.
/// The frontend uses `source` to split 'archive' vs 'user' collections.
#[tauri::command]
pub fn get_collections(state: tauri::State<AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(&format!("{} ORDER BY c.source DESC, c.name ASC", COLLECTION_SELECT))
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], row_to_collection)
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Create a new user collection. Returns the new collection's database ID.
#[tauri::command]
pub fn create_collection(name: String, state: tauri::State<AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO collections (name, source) VALUES (?1, 'user')",
        rusqlite::params![name.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

/// Rename a collection. Works for both user and archive collections.
#[tauri::command]
pub fn rename_collection(
    id: i64,
    name: String,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE collections SET name = ?1 WHERE id = ?2",
        rusqlite::params![name.trim(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a collection. ON DELETE CASCADE removes collection_images rows automatically.
/// Original image files are never touched.
#[tauri::command]
pub fn delete_collection(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "DELETE FROM collections WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Add images to a collection. Uses INSERT OR IGNORE so duplicates are silently skipped.
/// Wrapped in a transaction for atomicity.
#[tauri::command]
pub fn add_to_collection(
    collection_id: i64,
    image_ids: Vec<i64>,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    for image_id in &image_ids {
        tx.execute(
            "INSERT OR IGNORE INTO collection_images (collection_id, image_id) VALUES (?1, ?2)",
            rusqlite::params![collection_id, image_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove images from a collection. Wrapped in a transaction for atomicity.
#[tauri::command]
pub fn remove_from_collection(
    collection_id: i64,
    image_ids: Vec<i64>,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    for image_id in &image_ids {
        tx.execute(
            "DELETE FROM collection_images WHERE collection_id = ?1 AND image_id = ?2",
            rusqlite::params![collection_id, image_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Return all collections that contain the given image.
#[tauri::command]
pub fn get_image_collections(
    image_id: i64,
    state: tauri::State<AppState>,
) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT c.id, c.name, c.source, c.description, c.created_at,
                    COUNT(ci2.image_id) as image_count
             FROM collections c
             JOIN collection_images ci ON ci.collection_id = c.id AND ci.image_id = ?1
             LEFT JOIN collection_images ci2 ON ci2.collection_id = c.id
             GROUP BY c.id
             ORDER BY c.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![image_id], row_to_collection)
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
