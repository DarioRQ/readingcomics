mod archive;
mod comicinfo;
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
            commands::open_comic,
            commands::get_page,
            config::load_config,
            config::save_config,
            config::get_progress,
            config::set_read,
            config::set_progress,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
