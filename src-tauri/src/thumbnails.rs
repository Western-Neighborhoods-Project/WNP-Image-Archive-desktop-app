use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use image::imageops::FilterType;

use crate::db::{get_thumbnail_cache_dir, AppState};
use crate::models::{ThumbnailRequest, ThumbnailResult};

const THUMBNAIL_SIZE: u32 = 300;

/// Return the thumbnail path for a given image ID.
pub fn thumbnail_path_for_id(id: i64) -> PathBuf {
    get_thumbnail_cache_dir().join(format!("{}.jpg", id))
}

// ============================================================
// Tier 1: EXIF Thumbnail Extraction (fast, runs during import)
// ============================================================

/// Extract embedded EXIF thumbnails from all images that don't yet have a
/// thumbnail. For images without an embedded EXIF thumbnail (e.g. TIFFs, PNGs),
/// falls back to generating a full-quality thumbnail.
#[tauri::command]
pub async fn extract_exif_thumbnails_batch(
    state: tauri::State<'_, AppState>,
) -> Result<ThumbnailResult, String> {
    let start = Instant::now();

    // Fetch all images without a thumbnail.
    // Collect into Vec inside the block to release the Mutex before doing I/O.
    let images: Vec<(i64, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare("SELECT id, file_path FROM images WHERE thumbnail_path IS NULL")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        let result: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
        result
    };

    let mut extracted: u64 = 0;
    let mut fallback_generated: u64 = 0;
    let mut failed: u64 = 0;

    for (id, file_path) in &images {
        let thumb_path = thumbnail_path_for_id(*id);

        let result = try_extract_exif_thumbnail(*id, file_path, &thumb_path);
        match result {
            ExifResult::Extracted => {
                update_thumbnail_db(&state, *id, &thumb_path, false)?;
                extracted += 1;
            }
            ExifResult::NoEmbedded => {
                // No EXIF thumbnail — generate one immediately
                match generate_thumbnail_for_file(file_path, &thumb_path) {
                    Ok(()) => {
                        update_thumbnail_db(&state, *id, &thumb_path, true)?;
                        fallback_generated += 1;
                    }
                    Err(e) => {
                        eprintln!("thumbnail fallback failed for {}: {}", file_path, e);
                        failed += 1;
                    }
                }
            }
            ExifResult::Error(e) => {
                eprintln!("exif extraction failed for {}: {}", file_path, e);
                // Try fallback
                match generate_thumbnail_for_file(file_path, &thumb_path) {
                    Ok(()) => {
                        update_thumbnail_db(&state, *id, &thumb_path, true)?;
                        fallback_generated += 1;
                    }
                    Err(_) => {
                        failed += 1;
                    }
                }
            }
        }
    }

    Ok(ThumbnailResult {
        extracted,
        fallback_generated,
        failed,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

enum ExifResult {
    Extracted,
    NoEmbedded,
    Error(String),
}

/// Try to extract the embedded EXIF thumbnail from a file using exiftool.
/// Returns whether the extraction succeeded, found no thumbnail, or failed.
fn try_extract_exif_thumbnail(_id: i64, file_path: &str, thumb_path: &Path) -> ExifResult {
    // exiftool -b -ThumbnailImage <file_path>
    let output = match Command::new("exiftool")
        .args(["-b", "-ThumbnailImage", file_path])
        .output()
    {
        Ok(o) => o,
        Err(e) => return ExifResult::Error(e.to_string()),
    };

    if !output.status.success() || output.stdout.is_empty() {
        // No embedded thumbnail — not an error, just no EXIF thumb present
        return ExifResult::NoEmbedded;
    }

    // Save the raw bytes to the thumbnail file
    match std::fs::File::create(thumb_path).and_then(|mut f| f.write_all(&output.stdout)) {
        Ok(()) => ExifResult::Extracted,
        Err(e) => ExifResult::Error(e.to_string()),
    }
}

// ============================================================
// Tier 2: Full Quality Thumbnail Generation (on-demand)
// ============================================================

/// Generate full-quality (300px, Lanczos3) thumbnails for a batch of image IDs.
/// Called by the frontend as images scroll into view.
#[tauri::command]
pub async fn generate_full_thumbnails(
    request: ThumbnailRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ThumbnailResult, String> {
    let start = Instant::now();

    if request.image_ids.is_empty() {
        return Ok(ThumbnailResult {
            extracted: 0,
            fallback_generated: 0,
            failed: 0,
            duration_ms: 0,
        });
    }

    // Fetch file paths for the requested IDs.
    // Build an IN clause with literal IDs (safe: IDs are i64, not user strings).
    let id_list = request
        .image_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let images: Vec<(i64, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "SELECT id, file_path FROM images WHERE id IN ({}) AND thumbnail_generated = 0",
            id_list
        );
        let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        let result: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
        result
    };

    let mut generated: u64 = 0;
    let mut failed: u64 = 0;

    for (id, file_path) in &images {
        let thumb_path = thumbnail_path_for_id(*id);

        match generate_thumbnail_for_file(file_path, &thumb_path) {
            Ok(()) => {
                update_thumbnail_db(&state, *id, &thumb_path, true)?;
                generated += 1;
            }
            Err(e) => {
                eprintln!("thumbnail generation failed for {}: {}", file_path, e);
                failed += 1;
            }
        }
    }

    Ok(ThumbnailResult {
        extracted: 0,
        fallback_generated: generated,
        failed,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Generate a full-quality thumbnail for a single image by database ID.
/// Returns the thumbnail path on success.
#[tauri::command]
pub fn generate_thumbnail_single(
    image_id: i64,
    state: tauri::State<AppState>,
) -> Result<String, String> {
    let file_path: String = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT file_path FROM images WHERE id = ?1",
            rusqlite::params![image_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Image not found (id={}): {}", image_id, e))?
    };

    let thumb_path = thumbnail_path_for_id(image_id);
    generate_thumbnail_for_file(&file_path, &thumb_path)?;
    update_thumbnail_db(&state, image_id, &thumb_path, true)?;

    Ok(thumb_path.to_string_lossy().to_string())
}

// ============================================================
// Shared Helpers
// ============================================================

/// Resize an image file to fit within THUMBNAIL_SIZE×THUMBNAIL_SIZE,
/// maintaining aspect ratio, and save as JPEG to thumb_path.
fn generate_thumbnail_for_file(file_path: &str, thumb_path: &Path) -> Result<(), String> {
    let img = image::open(file_path)
        .map_err(|e| format!("Failed to open image {}: {}", file_path, e))?;

    let thumb = img.resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, FilterType::Lanczos3);

    thumb
        .save_with_format(thumb_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to save thumbnail: {}", e))?;

    Ok(())
}

/// Update the database thumbnail_path and thumbnail_generated flag for an image.
fn update_thumbnail_db(
    state: &tauri::State<AppState>,
    image_id: i64,
    thumb_path: &Path,
    generated: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE images SET thumbnail_path = ?1, thumbnail_generated = ?2 WHERE id = ?3",
        rusqlite::params![
            thumb_path.to_string_lossy().to_string(),
            generated as i32,
            image_id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
