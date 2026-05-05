// Source directories (Plan 12).
//
// Each row in `source_directories` is a top-level archive root the user
// has registered. The library can mix images from any number of sources;
// the sidebar tree groups them by source + their relative directory path
// inside the source.
//
// Lifecycle:
//   - add_source_directory: validates path, inserts a row, returns the id.
//     Caller (frontend setup or settings UI) follows up with scan_directory
//     using that path to populate images.
//   - remove_source_directory: deletes the row; ON DELETE CASCADE removes
//     all images that belonged to that source.
//   - rename_source_directory: label-only update.
//   - get_source_directory_tree: builds a hierarchical view by reading the
//     distinct `relative_dir` values for each source's images and folding
//     them into a tree.

use crate::auth;
use crate::db::AppState;
use crate::models::{SourceDirectory, SourceTreeNode, SourceTreeRoot};
use rusqlite::params;
use std::collections::BTreeMap;
use tauri::State;

// ── Path normalization ────────────────────────────────────────────────────

/// Canonicalise a user-supplied path so the same directory entered with
/// or without a trailing slash, or in different casings (on macOS
/// case-insensitive file systems), maps to the same row.
fn normalize_path(p: &str) -> String {
    p.trim().trim_end_matches('/').to_string()
}

fn default_label_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Archive")
        .to_string()
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_source_directories(state: State<AppState>) -> Result<Vec<SourceDirectory>, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .prepare(
            "SELECT s.id, s.path, s.label, s.created_at,
                    COALESCE((SELECT COUNT(*) FROM images i WHERE i.source_directory_id = s.id), 0)
             FROM source_directories s
             ORDER BY s.created_at ASC, s.id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SourceDirectory {
                id: r.get(0)?,
                path: r.get(1)?,
                label: r.get(2)?,
                created_at: r.get(3)?,
                image_count: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn add_source_directory(
    path: String,
    label: Option<String>,
    state: State<AppState>,
) -> Result<SourceDirectory, String> {
    auth::require_admin(&state)?;
    let normalized = normalize_path(&path);
    if normalized.is_empty() {
        return Err("Source directory path is required".to_string());
    }
    if !std::path::Path::new(&normalized).is_dir() {
        return Err(format!("Path is not a directory: {}", normalized));
    }

    let resolved_label = label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_label_from_path(&normalized));

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO source_directories (path, label) VALUES (?1, ?2)",
        params![normalized, resolved_label],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "A source directory with that path is already registered".to_string()
        } else {
            e.to_string()
        }
    })?;

    let id = db.last_insert_rowid();
    let (created_at,): (String,) = db
        .query_row(
            "SELECT created_at FROM source_directories WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?,)),
        )
        .map_err(|e| e.to_string())?;
    drop(db);

    // Wire the new path into the watcher (best-effort; logging only on
    // failure so an OS-level watch problem doesn't break the add).
    if let Ok(g) = state.watcher.lock() {
        if let Some(handle) = g.as_ref() {
            if let Err(e) = handle.add_path(&normalized) {
                log::warn!("watcher: add_path failed for {}: {}", normalized, e);
            }
        }
    }

    Ok(SourceDirectory {
        id,
        path: normalized,
        label: resolved_label,
        created_at,
        image_count: 0,
    })
}

#[tauri::command]
pub fn remove_source_directory(id: i64, state: State<AppState>) -> Result<(), String> {
    auth::require_admin(&state)?;
    // Look up the path before deleting so we can stop watching it.
    let path: Option<String> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT path FROM source_directories WHERE id = ?1",
            params![id],
            |r| r.get::<_, String>(0),
        )
        .ok()
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // ON DELETE CASCADE on images.source_directory_id removes every image
    // that belonged to this source. Audit log + collection_images rows on
    // those images cascade away too via their own FKs.
    let rows = db
        .execute(
            "DELETE FROM source_directories WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Source directory not found".to_string());
    }
    drop(db);

    if let Some(path) = path {
        if let Ok(g) = state.watcher.lock() {
            if let Some(handle) = g.as_ref() {
                if let Err(e) = handle.remove_path(&path) {
                    log::warn!("watcher: remove_path failed for {}: {}", path, e);
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn rename_source_directory(
    id: i64,
    label: String,
    state: State<AppState>,
) -> Result<(), String> {
    auth::require_admin(&state)?;
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Label is required".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .execute(
            "UPDATE source_directories SET label = ?1 WHERE id = ?2",
            params![trimmed, id],
        )
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("Source directory not found".to_string());
    }
    Ok(())
}

/// Find or create a source-directory row for the given path. Used by the
/// scanner so legacy `scan_directory(path)` calls keep working — the row
/// is created on first scan, then re-used.
pub fn find_or_create(
    conn: &rusqlite::Connection,
    path: &str,
) -> Result<(i64, String), String> {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return Err("Source directory path is required".to_string());
    }
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM source_directories WHERE path = ?1",
            params![normalized],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok((id, normalized));
    }
    let label = default_label_from_path(&normalized);
    conn.execute(
        "INSERT INTO source_directories (path, label) VALUES (?1, ?2)",
        params![normalized, label],
    )
    .map_err(|e| e.to_string())?;
    Ok((conn.last_insert_rowid(), normalized))
}

#[tauri::command]
pub fn get_source_directory_tree(state: State<AppState>) -> Result<Vec<SourceTreeRoot>, String> {
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // 1. List all sources with their image counts.
    let sources: Vec<SourceDirectory> = {
        let mut stmt = db
            .prepare(
                "SELECT s.id, s.path, s.label, s.created_at,
                        COALESCE((SELECT COUNT(*) FROM images i WHERE i.source_directory_id = s.id), 0)
                 FROM source_directories s
                 ORDER BY s.created_at ASC, s.id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SourceDirectory {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    label: r.get(2)?,
                    created_at: r.get(3)?,
                    image_count: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // 2. For each source, fetch all distinct (relative_dir, count) pairs.
    let mut roots: Vec<SourceTreeRoot> = Vec::with_capacity(sources.len());
    for source in sources {
        let mut stmt = db
            .prepare(
                "SELECT relative_dir, COUNT(*)
                 FROM images
                 WHERE source_directory_id = ?1
                 GROUP BY relative_dir",
            )
            .map_err(|e| e.to_string())?;
        let pairs: Vec<(String, i64)> = stmt
            .query_map(params![source.id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let children = build_tree(source.id, &pairs);
        roots.push(SourceTreeRoot { source, children });
    }

    Ok(roots)
}

/// Intermediate builder used by build_tree. Public-in-module so the
/// recursive finalize() helper can name it.
struct NodeBuilder {
    own_count: i64,        // images whose relative_dir is exactly this node's path
    descendant_count: i64, // images strictly beneath this node
    children: BTreeMap<String, NodeBuilder>,
}

impl NodeBuilder {
    fn new() -> Self {
        Self {
            own_count: 0,
            descendant_count: 0,
            children: BTreeMap::new(),
        }
    }
}

/// Fold a `[(relative_dir, image_count), ...]` flat list into a nested
/// tree. Counts at internal nodes are the sum of all descendants (so
/// clicking "Forest Hill" shows all images under any subfolder of it).
fn build_tree(source_id: i64, pairs: &[(String, i64)]) -> Vec<SourceTreeNode> {
    let mut root: BTreeMap<String, NodeBuilder> = BTreeMap::new();

    for (relative_dir, count) in pairs {
        if relative_dir.is_empty() {
            // Files directly under the source — they're shown by clicking
            // the source root, not as a tree node here.
            continue;
        }
        let parts: Vec<&str> = relative_dir.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        // Walk parts; at each level, ensure a node exists. The leaf node
        // (last part) gets own_count; intermediates get descendant_count.
        let mut cursor: &mut BTreeMap<String, NodeBuilder> = &mut root;
        for (depth, part) in parts.iter().enumerate() {
            let entry = cursor
                .entry((*part).to_string())
                .or_insert_with(NodeBuilder::new);
            if depth == parts.len() - 1 {
                entry.own_count += count;
            } else {
                entry.descendant_count += count;
            }
            cursor = &mut entry.children;
        }
    }

    finalize(source_id, "", root)
}

fn finalize(
    source_id: i64,
    prefix: &str,
    nodes: BTreeMap<String, NodeBuilder>,
) -> Vec<SourceTreeNode> {
    nodes
        .into_iter()
        .map(|(name, builder)| {
            let relative_dir = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };
            let children = finalize(source_id, &relative_dir, builder.children);
            SourceTreeNode {
                source_directory_id: source_id,
                label: name,
                relative_dir,
                image_count: builder.own_count + builder.descendant_count,
                children,
            }
        })
        .collect()
}
