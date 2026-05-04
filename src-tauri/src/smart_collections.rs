// Smart collections — saved filter presets the user creates from the
// library view's active filter state. Stored in the `smart_collections`
// table (id, name, filters TEXT, created_at) which has been in the
// schema since Plan 1; this module just adds the CRUD commands.
//
// The `filters` column is opaque JSON: whatever the frontend's
// `FilterState` shape is at write-time. The backend doesn't introspect
// it — list returns the JSON as-is, the frontend parses + applies.

use crate::auth;
use crate::db::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartCollection {
    pub id: i64,
    pub name: String,
    /// JSON blob of the saved filter state (frontend-defined shape).
    pub filters: String,
    pub created_at: String,
}

#[tauri::command]
pub fn list_smart_collections(
    state: State<AppState>,
) -> Result<Vec<SmartCollection>, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, filters, created_at
             FROM smart_collections
             ORDER BY name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SmartCollection {
                id: r.get(0)?,
                name: r.get(1)?,
                filters: r.get(2)?,
                created_at: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn create_smart_collection(
    name: String,
    filters: String,
    state: State<AppState>,
) -> Result<SmartCollection, String> {
    auth::require_session(&state)?;
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("Name is required".to_string());
    }
    // Light JSON validation so we don't store garbage; backend doesn't
    // care about the shape, just that it's valid JSON.
    if serde_json::from_str::<serde_json::Value>(&filters).is_err() {
        return Err("Filters must be valid JSON".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO smart_collections (name, filters) VALUES (?1, ?2)",
        params![trimmed, filters],
    )
    .map_err(|e| e.to_string())?;
    let id = db.last_insert_rowid();
    let created_at: String = db
        .query_row(
            "SELECT created_at FROM smart_collections WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(SmartCollection {
        id,
        name: trimmed,
        filters,
        created_at,
    })
}

#[tauri::command]
pub fn delete_smart_collection(
    id: i64,
    state: State<AppState>,
) -> Result<(), String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM smart_collections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
