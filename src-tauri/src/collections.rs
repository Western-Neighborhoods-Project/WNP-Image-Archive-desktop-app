use crate::db::AppState;
use crate::models::Collection;

/// Return all collections, with image counts.
/// The frontend uses the `source` field to separate archive vs. user collections in the sidebar.
#[tauri::command]
pub fn get_collections(state: tauri::State<AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .prepare(
            "SELECT c.id, c.name, c.source, c.description, c.created_at,
                    COUNT(ci.image_id) as image_count
             FROM collections c
             LEFT JOIN collection_images ci ON ci.collection_id = c.id
             GROUP BY c.id
             ORDER BY c.source DESC, c.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                image_count: row.get::<_, i64>(5)? as u64,
            })
        })
        .map_err(|e| e.to_string())?;

    let result: Vec<Collection> = rows.filter_map(|r| r.ok()).collect();
    Ok(result)
}
