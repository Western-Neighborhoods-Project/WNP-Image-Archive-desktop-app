use crate::auth;
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

/// Build a safe FTS5 MATCH query from free-text user input.
///
/// Users type into a plain search box, not FTS5 query syntax, so raw input must
/// never reach MATCH: bare metacharacters like `(` `)` `:` `*` `^` or an
/// unbalanced `"` raise `fts5: syntax error` and fail the *entire* query,
/// blanking the library view. We split the input into terms (honoring
/// `"quoted phrases"`), then re-emit each term as a double-quoted FTS5 string —
/// quoting makes every metacharacter literal, so the output can never be
/// malformed. Terms are ANDed implicitly. Hyphens are
/// treated as separators so catalog numbers like `WNP83-0001` match as the
/// parts `"WNP83"` AND `"0001"` rather than `WNP83 NOT 0001`.
///
/// Returns `None` when the input has no searchable characters (so the caller
/// omits the FTS clause entirely rather than matching on an empty string).
fn build_fts_match_query(input: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        match c {
            '"' => {
                // A quote toggles phrase mode; flush whatever term precedes it.
                if !current.is_empty() {
                    terms.push(std::mem::take(&mut current));
                }
                in_quotes = !in_quotes;
            }
            _ if in_quotes => current.push(c),
            _ if c.is_whitespace() || c == '-' => {
                if !current.is_empty() {
                    terms.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        terms.push(current);
    }

    if terms.is_empty() {
        return None;
    }

    // Terms never contain a `"` (it is always a structural delimiter above), so
    // wrapping in quotes is sufficient to make the phrase literal.
    Some(
        terms
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Map an ImageRecord from a database row, looking up each column by
/// NAME (not ordinal). This means every SELECT that returns from the
/// `images` table just needs to include all the columns ImageRecord
/// expects — order doesn't matter, and adding new columns in the
/// future doesn't silently break callers that hadn't updated their
/// SELECT lists. Public so editor.rs can reuse it.
pub fn row_to_image_record(row: &rusqlite::Row) -> rusqlite::Result<ImageRecord> {
    Ok(ImageRecord {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        catalog_number: row.get("catalog_number")?,
        file_size: row.get("file_size")?,
        file_modified: row.get("file_modified")?,
        title: row.get("title")?,
        description: row.get("description")?,
        city: row.get("city")?,
        state: row.get("state")?,
        country: row.get("country")?,
        keywords: row.get("keywords")?,
        date_display: row.get("date_display")?,
        date_start: row.get("date_start")?,
        date_end: row.get("date_end")?,
        photographer: row.get("photographer")?,
        donor: row.get("donor")?,
        acquisition_date: row.get("acquisition_date")?,
        archival_collection: row.get("archival_collection")?,
        usage_rights: row.get("usage_rights")?,
        internal_notes: row.get("internal_notes")?,
        thumbnail_path: row.get("thumbnail_path")?,
        thumbnail_generated: row.get::<_, i32>("thumbnail_generated")? != 0,
        metadata_synced: row.get::<_, i32>("metadata_synced")? != 0,
        // Plan 9: OpenSFHistory mirror columns
        caption: row.get("caption")?,
        dimensions: row.get("dimensions")?,
        format: row.get("format")?,
        publisher: row.get("publisher")?,
        citation: row.get("citation")?,
        download_permitted: row.get("download_permitted")?,
        neighborhoods: row.get("neighborhoods")?,
        photosets: row.get("photosets")?,
        osf_collections: row.get("osf_collections")?,
        osf_page_url: row.get("osf_page_url")?,
        last_synced_at: row.get("last_synced_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub const IMAGE_SELECT_COLS: &str = "
    id, file_path, catalog_number, file_size, file_modified,
    title, description, city, state, country,
    keywords, date_display, date_start, date_end, photographer,
    donor, acquisition_date, archival_collection, usage_rights, internal_notes,
    thumbnail_path, thumbnail_generated, metadata_synced,
    caption, dimensions, format, publisher, citation, download_permitted,
    neighborhoods, photosets, osf_collections, osf_page_url, last_synced_at,
    created_at, updated_at
";

/// Query images with pagination, optional sorting, and optional filters.
/// This command serves both the basic library view and the filtered view.
#[tauri::command]
pub fn query_images(
    query: ImageQuery,
    state: tauri::State<AppState>,
) -> Result<ImageQueryResult, String> {
    auth::require_session(&state)?;
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

    // Plan 12: source-directory tree filters.
    if let Some(source_id) = query.source_directory_id {
        params.push(Box::new(source_id));
        where_clauses.push(format!("i.source_directory_id = ?{}", params.len()));
    }
    if let Some(ref relative_dir) = query.relative_dir {
        let trimmed = relative_dir.trim_matches('/');
        if !trimmed.is_empty() {
            // Match the directory itself OR any descendant. Two parameter
            // slots: exact match + "<prefix>/" LIKE for descendants.
            params.push(Box::new(trimmed.to_string()));
            let exact_idx = params.len();
            params.push(Box::new(format!("{}/%", trimmed)));
            let like_idx = params.len();
            where_clauses.push(format!(
                "(i.relative_dir = ?{} OR i.relative_dir LIKE ?{})",
                exact_idx, like_idx
            ));
        }
    }

    // Full-text search via FTS5
    if let Some(ref q) = query.search_query {
        if let Some(match_query) = build_fts_match_query(q) {
            params.push(Box::new(match_query));
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
    auth::require_session(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sql = format!("SELECT {} FROM images i WHERE i.id = ?1", IMAGE_SELECT_COLS);
    db.query_row(&sql, rusqlite::params![id], row_to_image_record)
        .map_err(|e| format!("Image not found (id={}): {}", id, e))
}

#[cfg(test)]
mod tests {
    use super::build_fts_match_query;

    #[test]
    fn plain_words_are_quoted_and_anded() {
        assert_eq!(
            build_fts_match_query("sutro baths").as_deref(),
            Some("\"sutro\" \"baths\"")
        );
    }

    #[test]
    fn hyphenated_catalog_numbers_split_into_terms() {
        assert_eq!(
            build_fts_match_query("WNP83-0001").as_deref(),
            Some("\"WNP83\" \"0001\"")
        );
    }

    #[test]
    fn fts5_metacharacters_stay_inside_quotes() {
        // The exact input the old sanitizer crashed on — a filename the scanner
        // itself accepts as a catalog number. The parens must end up *inside* a
        // quoted phrase (literal), never as bare grouping operators for MATCH.
        let out = build_fts_match_query("IMG_1234 (1)").unwrap();
        assert_eq!(out, "\"IMG_1234\" \"(1)\"");
    }

    #[test]
    fn colon_and_star_are_literal_not_operators() {
        // `*` and `:` are FTS5 operators when bare; quoting keeps them literal.
        let out = build_fts_match_query("*:foo").unwrap();
        assert_eq!(out, "\"*:foo\"");
    }

    #[test]
    fn explicit_phrases_are_preserved_as_one_term() {
        assert_eq!(
            build_fts_match_query("\"san francisco\"").as_deref(),
            Some("\"san francisco\"")
        );
    }

    #[test]
    fn unbalanced_quote_does_not_produce_malformed_output() {
        // An unbalanced quote used to crash MATCH. Now it just closes at EOF.
        let out = build_fts_match_query("beach \"party").unwrap();
        assert_eq!(out, "\"beach\" \"party\"");
    }

    #[test]
    fn empty_or_whitespace_only_returns_none() {
        assert_eq!(build_fts_match_query("   "), None);
        assert_eq!(build_fts_match_query(""), None);
        assert_eq!(build_fts_match_query("\"\""), None);
    }

    #[test]
    fn generated_queries_never_error_against_real_fts5_trigram() {
        // The real safety proof: run every generated query against an actual
        // FTS5 trigram table (the same tokenizer schema.sql uses) and confirm
        // MATCH executes without a syntax error — including short/punctuation
        // inputs that tokenize to zero trigrams.
        use rusqlite::Connection;
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE fts USING fts5(body, tokenize='trigram');
             INSERT INTO fts(body) VALUES ('IMG_1234 (1) sutro baths san francisco');",
        )
        .unwrap();

        let inputs = [
            "IMG_1234 (1)",
            "*:foo",
            "beach \"party",
            "\"san francisco\"",
            "WNP83-0001",
            "sutro baths",
            "()",
            "^",
            "a:b (c) OR d",
            "NEAR(x y)",
            "AND",
        ];
        for input in inputs {
            if let Some(mq) = build_fts_match_query(input) {
                let res: Result<i64, _> = conn.query_row(
                    "SELECT count(*) FROM fts WHERE fts MATCH ?1",
                    [&mq],
                    |r| r.get(0),
                );
                assert!(
                    res.is_ok(),
                    "MATCH errored for input {input:?} -> {mq:?}: {:?}",
                    res.err()
                );
            }
        }
    }
}
