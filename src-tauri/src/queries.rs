use crate::db::AppState;
use crate::models::{ImageQuery, ImageQueryResult, ImageRecord};

/// Allowed sort columns — used as a whitelist to prevent SQL injection.
const ALLOWED_SORT_COLUMNS: &[&str] = &[
    "catalog_number",
    "date_start",
    "created_at",
    "updated_at",
    "title",
    "city",
    "photographer",
    "file_size",
];

fn validate_sort_column(col: &str) -> Result<&str, String> {
    if ALLOWED_SORT_COLUMNS.contains(&col) {
        Ok(col)
    } else {
        Err(format!(
            "Invalid sort column '{}'. Allowed: {:?}",
            col, ALLOWED_SORT_COLUMNS
        ))
    }
}

/// Map an ImageRecord from a database row.
/// Public so `editor.rs` can reuse it for `get_recently_viewed`.
pub fn row_to_image_record(row: &rusqlite::Row) -> rusqlite::Result<ImageRecord> {
    Ok(ImageRecord {
        id: row.get(0)?,
        file_path: row.get(1)?,
        catalog_number: row.get(2)?,
        file_size: row.get(3)?,
        file_modified: row.get(4)?,
        title: row.get(5)?,
        description: row.get(6)?,
        city: row.get(7)?,
        state: row.get(8)?,
        country: row.get(9)?,
        keywords: row.get(10)?,
        date_display: row.get(11)?,
        date_start: row.get(12)?,
        date_end: row.get(13)?,
        photographer: row.get(14)?,
        donor: row.get(15)?,
        acquisition_date: row.get(16)?,
        archival_collection: row.get(17)?,
        usage_rights: row.get(18)?,
        internal_notes: row.get(19)?,
        thumbnail_path: row.get(20)?,
        thumbnail_generated: row.get::<_, i32>(21)? != 0,
        metadata_synced: row.get::<_, i32>(22)? != 0,
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
    })
}

const IMAGE_SELECT_COLS: &str = "
    id, file_path, catalog_number, file_size, file_modified,
    title, description, city, state, country,
    keywords, date_display, date_start, date_end, photographer,
    donor, acquisition_date, archival_collection, usage_rights, internal_notes,
    thumbnail_path, thumbnail_generated, metadata_synced, created_at, updated_at
";

/// Query images with pagination, optional sorting, and optional filters.
/// This command serves both the basic library view and the filtered view.
#[tauri::command]
pub fn query_images(
    query: ImageQuery,
    state: tauri::State<AppState>,
) -> Result<ImageQueryResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sort_col = validate_sort_column(
        query
            .sort_by
            .as_deref()
            .unwrap_or("catalog_number"),
    )?;
    let sort_order = match query.sort_order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    // Build WHERE clauses dynamically
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref city) = query.city {
        params.push(Box::new(city.clone()));
        where_clauses.push(format!("i.city = ?{}", params.len()));
    }

    if let Some(ref photographer) = query.photographer {
        params.push(Box::new(photographer.clone()));
        where_clauses.push(format!("i.photographer = ?{}", params.len()));
    }

    if let Some(collection_id) = query.collection_id {
        params.push(Box::new(collection_id));
        where_clauses.push(format!(
            "i.id IN (SELECT image_id FROM collection_images WHERE collection_id = ?{})",
            params.len()
        ));
    }

    if let Some(year_start) = query.year_start {
        params.push(Box::new(format!("{}-01-01", year_start)));
        where_clauses.push(format!("i.date_start >= ?{}", params.len()));
    }

    if let Some(year_end) = query.year_end {
        params.push(Box::new(format!("{}-12-31", year_end)));
        where_clauses.push(format!("i.date_start <= ?{}", params.len()));
    }

    if query.missing_metadata.unwrap_or(false) {
        where_clauses.push(
            "(i.title IS NULL AND i.city IS NULL AND i.date_display IS NULL)".to_string(),
        );
    }

    // Full-text search via FTS5
    if let Some(ref q) = query.search_query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            // Sanitize for FTS5: replace hyphens with spaces so catalog numbers like
            // "WNP83-0001" don't get interpreted as "WNP83 NOT 0001".
            // We only do this when the query isn't already using explicit FTS5 phrase
            // syntax (double quotes), so phrase queries like "san francisco" still work.
            let sanitized = if trimmed.contains('"') {
                trimmed.to_string()
            } else {
                trimmed.replace('-', " ")
            };
            params.push(Box::new(sanitized));
            where_clauses.push(format!(
                "i.id IN (SELECT rowid FROM images_fts WHERE images_fts MATCH ?{})",
                params.len()
            ));
        }
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // COUNT query
    let count_sql = format!("SELECT COUNT(*) FROM images i {}", where_sql);
    let count_params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();

    let total_count: u64 = db
        .query_row(&count_sql, count_params.as_slice(), |row| row.get(0))
        .map_err(|e| e.to_string())?;

    // Data query
    let data_sql = format!(
        "SELECT {} FROM images i {} ORDER BY i.{} {} LIMIT ?{} OFFSET ?{}",
        IMAGE_SELECT_COLS,
        where_sql,
        sort_col,
        sort_order,
        params.len() + 1,
        params.len() + 2,
    );

    params.push(Box::new(query.limit as i64));
    params.push(Box::new(query.offset as i64));

    let all_params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = db.prepare(&data_sql).map_err(|e| e.to_string())?;
    let images: Vec<ImageRecord> = stmt
        .query_map(all_params.as_slice(), row_to_image_record)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ImageQueryResult {
        images,
        total_count,
    })
}

/// Fetch a single image record by database ID.
#[tauri::command]
pub fn get_image(id: i64, state: tauri::State<AppState>) -> Result<ImageRecord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sql = format!("SELECT {} FROM images i WHERE i.id = ?1", IMAGE_SELECT_COLS);
    db.query_row(&sql, rusqlite::params![id], row_to_image_record)
        .map_err(|e| format!("Image not found (id={}): {}", id, e))
}
