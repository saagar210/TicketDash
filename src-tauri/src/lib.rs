mod commands;
mod db;
mod errors;
mod jira;
mod models;
mod services;

use commands::*;
use db::DbPool;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::disallowed_methods)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // Initialize database
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {}", e))?;

            std::fs::create_dir_all(&app_dir)
                .map_err(|e| format!("Failed to create app directory at {:?}: {}", app_dir, e))?;

            let db_path: PathBuf = app_dir.join("tickets.db");
            let db_path_str = db_path
                .to_str()
                .ok_or_else(|| format!("Invalid DB path: {:?}", db_path))?;

            let db_pool = DbPool::new(db_path_str)
                .map_err(|e| format!("Failed to initialize database at {:?}: {}", db_path, e))?;

            app.manage(db_pool);
            app.manage(SyncLock(tokio::sync::Mutex::new(false)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            store_jira_token,
            delete_jira_token,
            verify_jira_connection,
            save_jira_settings,
            load_jira_settings,
            trigger_sync,
            get_sync_status,
            get_dashboard_data,
            get_all_tickets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
