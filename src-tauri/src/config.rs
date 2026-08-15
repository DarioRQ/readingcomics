//! Persistencia de la configuración de la app.
//!
//! Se guarda como JSON en el directorio de config del sistema
//! (`%APPDATA%\com.root.readingcomics\` en Windows, `~/.config/...` en Linux),
//! que es el sitio que el SO reserva para esto y que sobrevive a las
//! actualizaciones de la app.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct AppConfig {
    /// Carpeta de la biblioteca elegida por el usuario.
    pub library_root: Option<String>,
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no se pudo resolver el directorio de config: {e}"))?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
pub fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let path = config_path(&app)?;

    // Primera ejecución: todavía no hay fichero, no es un error.
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(e) => return Err(format!("no se pudo leer la config: {e}")),
    };

    // Un config.json corrupto no debe dejar la app inutilizable: se descarta y
    // se arranca en limpio.
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let dir = path
        .parent()
        .ok_or_else(|| "ruta de config sin directorio padre".to_string())?;
    fs::create_dir_all(dir).map_err(|e| format!("no se pudo crear el directorio de config: {e}"))?;

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("no se pudo serializar la config: {e}"))?;

    // Escritura atómica: si la app muere a media escritura, el config.json
    // anterior sigue intacto en vez de quedarse truncado.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("no se pudo escribir la config: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("no se pudo guardar la config: {e}"))?;

    Ok(())
}
