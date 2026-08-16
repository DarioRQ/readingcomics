mod archive;
mod comicinfo;
mod metron;
mod commands;
mod config;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // El progreso se carga una sola vez al arrancar y vive en memoria.
            let progress = config::load_progress(app.handle());
            app.manage(config::Progress(Mutex::new(progress)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_dir,
            commands::get_comic_info,
            commands::get_folder_info,
            commands::get_series_info,
            commands::list_comics_deep,
            commands::set_folder_cover,
            commands::open_comic,
            commands::get_page,
            config::load_config,
            config::save_config,
            config::get_progress,
            config::set_read,
            config::set_progress,
            config::set_many_read,
            config::get_series_cache,
            config::save_series,
            config::forget_series,
            metron::metron_connect,
            metron::metron_status,
            metron::metron_disconnect,
            metron::metron_find_series,
            metron::metron_search_series,
            metron::metron_series_detail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
