//! Persistencia de configuración y de progreso de lectura.
//!
//! Se guarda como JSON en el directorio de config del sistema
//! (`%APPDATA%\com.root.readingcomics\` en Windows, `~/.config/...` en Linux),
//! que es el sitio que el SO reserva para esto y que sobrevive a las
//! actualizaciones de la app.
//!
//! Ajustes y progreso viven en ficheros distintos a propósito: el progreso
//! crece con la biblioteca y se escribe a cada rato, y no conviene arriesgar
//! los ajustes en cada una de esas escrituras.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct AppConfig {
    /// Biblioteca abierta ahora mismo.
    pub library_root: Option<String>,
    /// Bibliotecas guardadas, para poder saltar entre ellas sin volver a
    /// buscar la carpeta. `serde(default)` mantiene la compatibilidad con los
    /// config.json de versiones anteriores, que no traían este campo.
    pub libraries: Vec<String>,
}

/// Estado de lectura de un cómic concreto.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct ReadState {
    pub read: bool,
    /// Última página vista, para poder reanudar donde se dejó.
    pub last_page: usize,
}

pub type ProgressMap = HashMap<String, ReadState>;

/// Progreso cargado en memoria. Evita releer el fichero en cada marcado y,
/// sobre todo, evita que dos escrituras seguidas se pisen.
pub struct Progress(pub Mutex<ProgressMap>);

fn config_file(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no se pudo resolver el directorio de config: {e}"))?;
    Ok(dir.join(name))
}

/// Lee un JSON del directorio de config. Si no existe (primera ejecución) o
/// está corrupto, devuelve el valor por defecto en vez de dejar la app
/// inutilizable.
fn read_json<T: Default + serde::de::DeserializeOwned>(path: &PathBuf) -> T {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

/// Escritura atómica: si la app muere a media escritura, el fichero anterior
/// sigue intacto en vez de quedarse truncado.
fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| "ruta de config sin directorio padre".to_string())?;
    fs::create_dir_all(dir).map_err(|e| format!("no se pudo crear el directorio: {e}"))?;

    let json =
        serde_json::to_string_pretty(value).map_err(|e| format!("no se pudo serializar: {e}"))?;

    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json).map_err(|e| format!("no se pudo escribir: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("no se pudo guardar: {e}"))?;
    Ok(())
}

/* ---------- Ajustes ---------- */

#[tauri::command]
pub fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    Ok(read_json(&config_file(&app, "config.json")?))
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    write_json(&config_file(&app, "config.json")?, &config)
}

/* ---------- Progreso de lectura ---------- */

/// Carga inicial del progreso, para dejarlo en memoria al arrancar.
pub fn load_progress(app: &tauri::AppHandle) -> ProgressMap {
    match config_file(app, "progress.json") {
        Ok(path) => read_json(&path),
        Err(_) => ProgressMap::default(),
    }
}

fn persist(app: &tauri::AppHandle, map: &ProgressMap) -> Result<(), String> {
    write_json(&config_file(app, "progress.json")?, map)
}

#[tauri::command]
pub fn get_progress(state: tauri::State<'_, Progress>) -> Result<ProgressMap, String> {
    let map = state.0.lock().map_err(|_| "estado bloqueado".to_string())?;
    Ok(map.clone())
}

/// Marca o desmarca un cómic como leído.
#[tauri::command]
pub fn set_read(
    app: tauri::AppHandle,
    state: tauri::State<'_, Progress>,
    path: String,
    read: bool,
) -> Result<(), String> {
    let snapshot = {
        let mut map = state.0.lock().map_err(|_| "estado bloqueado".to_string())?;
        let entry = map.entry(path).or_default();
        entry.read = read;
        // Desmarcar como leído devuelve el cómic al principio: si lo marcas
        // como no leído es porque quieres volver a empezarlo.
        if !read {
            entry.last_page = 0;
        }
        map.clone()
    };
    persist(&app, &snapshot)
}

/// Marca o desmarca de golpe todos los cómics de una lista.
///
/// Recibe las rutas ya resueltas por el backend de comandos, que es quien sabe
/// recorrer carpetas y validar que están dentro de la biblioteca.
#[tauri::command]
pub fn set_many_read(
    app: tauri::AppHandle,
    state: tauri::State<'_, Progress>,
    paths: Vec<String>,
    read: bool,
) -> Result<(), String> {
    let snapshot = {
        let mut map = state.0.lock().map_err(|_| "estado bloqueado".to_string())?;
        for path in paths {
            let entry = map.entry(path).or_default();
            entry.read = read;
            if !read {
                entry.last_page = 0;
            }
        }
        map.clone()
    };
    // Una sola escritura para todo el lote, en vez de una por cómic.
    persist(&app, &snapshot)
}

/// Guarda por dónde va el usuario. Al llegar a la última página el cómic queda
/// marcado como leído solo, que es lo que se espera al terminarlo.
#[tauri::command]
pub fn set_progress(
    app: tauri::AppHandle,
    state: tauri::State<'_, Progress>,
    path: String,
    page: usize,
    total: usize,
) -> Result<bool, String> {
    let (snapshot, read) = {
        let mut map = state.0.lock().map_err(|_| "estado bloqueado".to_string())?;
        let entry = map.entry(path).or_default();
        entry.last_page = page;
        if total > 0 && page + 1 >= total {
            entry.read = true;
        }
        let read = entry.read;
        (map.clone(), read)
    };
    persist(&app, &snapshot)?;
    Ok(read)
}
