// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod commands;
pub mod downloader;

use downloader::DownloadManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let download_manager = Arc::new(Mutex::new(DownloadManager::new(10)));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let win = app.get_webview_window("main").unwrap();
            if let Some(monitor) = win.current_monitor().unwrap() {
                let size = monitor.size();
                let width = (size.width as f64 * 0.5) as u32;
                let height = (size.height as f64 * 0.7) as u32;
                win.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
                    .unwrap();
                win.center().unwrap();
            }
            Ok(())
        })
        .manage(download_manager)
        .invoke_handler(tauri::generate_handler![
            commands::get_schools,
            commands::start_oauth,
            commands::exchange_token,
            commands::get_user_info,
            commands::fetch_matches,
            commands::fetch_stages,
            commands::fetch_works,
            commands::download_works,
            commands::get_system_info,
            commands::pause_downloads,
            commands::resume_downloads,
            commands::stop_downloads,
            commands::get_download_state,
            commands::get_current_concurrency
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
