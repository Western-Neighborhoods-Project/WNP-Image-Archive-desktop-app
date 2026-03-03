pub mod collections;
pub mod db;
pub mod editor;
pub mod metadata;
pub mod models;
pub mod queries;
pub mod scanner;
pub mod settings;
pub mod thumbnails;

use std::sync::Mutex;
use db::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = db::init_db().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            // Settings
            settings::get_setting,
            settings::set_setting,
            settings::reset_catalog,
            // Scanner
            scanner::scan_directory,
            scanner::get_scan_stats,
            // Metadata
            metadata::extract_metadata_batch,
            metadata::extract_metadata_single,
            // Thumbnails
            thumbnails::extract_exif_thumbnails_batch,
            thumbnails::generate_full_thumbnails,
            thumbnails::generate_thumbnail_single,
            // Queries
            queries::query_images,
            queries::get_image,
            // Collections (Phase 1: read-only)
            collections::get_collections,
            // Editor (Phase 2)
            editor::update_image_metadata,
            editor::get_audit_log,
            editor::write_metadata_to_file,
            editor::log_image_view,
            editor::get_recently_viewed,
            editor::get_filter_options,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
