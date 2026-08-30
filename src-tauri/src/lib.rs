pub mod auth;
pub mod background_jobs;
pub mod bug_reports;
pub mod collections;
pub mod db;
pub mod drive;
pub mod editor;
pub mod export;
pub mod http;
pub mod metadata;
pub mod models;
pub mod opensf_sync;
pub mod queries;
pub mod scanner;
pub mod settings;
pub mod sharing;
pub mod smart_collections;
pub mod source_directories;
pub mod thumbnails;
pub mod user_management;
pub mod watcher;

use db::AppState;
use drive::DriveStatus;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize structured logging. Default level is `info`; override via
    // RUST_LOG=debug,image_archive_manager_lib=debug for noisy traces.
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();

    let db = db::init_db().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            db: Mutex::new(db),
            drive_state: Mutex::new(DriveStatus::default()),
            current_user: Mutex::new(None),
            login_attempts: Mutex::new(HashMap::new()),
            watcher: Mutex::new(None),
        })
        .setup(|app| {
            // Drive monitoring poller — runs for the life of the app. The
            // returned shutdown flag is dropped here; the OS reaps the
            // background thread when the process exits. Future work: store
            // the flag + a JoinHandle in AppState so a graceful in-process
            // restart can stop the loop and start a new one.
            let _shutdown = drive::spawn_drive_poller(app.handle().clone());

            // Plan 12 file watcher — fires "library:filesystem-changed"
            // events when files appear/disappear under any registered
            // source directory. The frontend re-scans + refreshes in
            // response (it has the admin session for the existing
            // scan_directory + thumbnail commands).
            match watcher::spawn_watcher(app.handle().clone()) {
                Ok(handle) => {
                    if let Ok(mut g) = app.state::<AppState>().watcher.lock() {
                        *g = Some(handle);
                    }
                }
                Err(e) => log::warn!("Failed to spawn file watcher: {}", e),
            }

            // Plan 13 background worker — generates thumbnails + extracts
            // metadata for any image in the 'pending' state. Polls every
            // 5s when idle; emits `background:progress` events.
            let _bg_shutdown = background_jobs::spawn_worker(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Bug reporting (Debugging tab)
            bug_reports::submit_bug_report,
            // Settings
            settings::get_setting,
            settings::get_public_setting,
            settings::set_setting,
            settings::reset_catalog,
            // Scanner
            scanner::scan_directory,
            scanner::get_scan_stats,
            // Thumbnails — only the on-demand path remains; batch / single
            // extraction live in the Plan 13 background worker now.
            thumbnails::generate_full_thumbnails,
            // Queries
            queries::query_images,
            queries::get_image,
            // Collections (Phase 3: full CRUD)
            collections::get_collections,
            collections::create_collection,
            collections::rename_collection,
            collections::delete_collection,
            collections::add_to_collection,
            collections::remove_from_collection,
            collections::get_image_collections,
            // Editor (Phase 2)
            editor::update_image_metadata,
            editor::get_audit_log,
            editor::get_recent_activity,
            editor::get_audit_log_global,
            editor::export_audit_log_csv,
            editor::write_metadata_to_file,
            editor::log_image_view,
            editor::get_recently_viewed,
            editor::get_filter_options,
            // Sharing (Phase 4)
            sharing::fetch_orders,
            sharing::fulfill_order,
            sharing::fail_order,
            // Ad-hoc share dialog (Plan 5)
            sharing::create_share_link,
            // OpenSFHistory metadata sync (Plan 9)
            opensf_sync::sync_image_from_opensf,
            // Smart collections (saved filter presets)
            smart_collections::list_smart_collections,
            smart_collections::create_smart_collection,
            smart_collections::delete_smart_collection,
            // Source directories (Plan 12)
            source_directories::list_source_directories,
            source_directories::add_source_directory,
            source_directories::remove_source_directory,
            source_directories::rename_source_directory,
            source_directories::get_source_directory_tree,
            // Background jobs (Plan 13)
            background_jobs::get_background_progress,
            background_jobs::list_thumbnail_failures,
            background_jobs::list_metadata_failures,
            background_jobs::retry_failed_thumbnails,
            background_jobs::retry_failed_metadata,
            // Drive monitoring (Plan 6)
            drive::get_drive_status,
            drive::retry_drive_connection,
            drive::reveal_drive_in_finder,
            // Auth + user management (Plan 10)
            auth::is_setup_required,
            auth::create_first_admin,
            auth::login,
            auth::logout,
            auth::get_current_user,
            user_management::list_users,
            user_management::create_user,
            user_management::update_user_role,
            user_management::update_user_password,
            user_management::delete_user,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
