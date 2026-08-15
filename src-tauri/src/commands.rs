use crate::archive;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Serialize, Clone)]
pub struct ComicMeta {
    pub path: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct FolderMeta {
    pub path: String,
    pub name: String,
}

/// Contenido de un nivel de la biblioteca. `parent` es `None` en la raíz, que
/// es lo que impide navegar por encima de la carpeta elegida.
///
/// Deliberadamente NO trae portadas ni recuentos: solo lo que se saca del
/// sistema de ficheros sin abrir un solo archivo. Todo lo caro se pide luego,
/// pieza a pieza, para que la rejilla aparezca al instante.
#[derive(Serialize, Clone)]
pub struct DirListing {
    pub path: String,
    pub parent: Option<String>,
    pub folders: Vec<FolderMeta>,
    pub comics: Vec<ComicMeta>,
}

/// Datos caros de un cómic. `error` viaja hasta la interfaz en vez de hacer
/// desaparecer el cómic: si un archivo está corrupto el usuario tiene que
/// verlo y saber por qué, no encontrarse un hueco.
#[derive(Serialize, Clone)]
pub struct ComicInfo {
    pub cover: Option<String>,
    pub page_count: usize,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct FolderInfo {
    pub cover: Option<String>,
    pub comic_count: usize,
}

const THUMB_MAX_DIM: u32 = 340;
const THUMB_QUALITY: u8 = 78;

/// Nombres usados como portada manual de una carpeta, en orden de preferencia.
/// Es la convención de siempre (`folder.jpg` de Windows).
const FOLDER_COVER_STEMS: [&str; 2] = ["cover", "folder"];
const FOLDER_COVER_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

fn to_data_uri(bytes: &[u8]) -> String {
    format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))
}

/// Clave de caché: ruta + tamaño + fecha de modificación. Si el archivo cambia,
/// la clave cambia y la miniatura se regenera sola.
fn cache_key(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    mtime.hash(&mut hasher);
    Some(format!("{:016x}", hasher.finish()))
}

fn cache_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?.join("thumbs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn cached_thumb(app: &tauri::AppHandle, key: &str) -> Option<Vec<u8>> {
    let path = cache_dir(app)?.join(format!("{key}.jpg"));
    std::fs::read(path).ok()
}

fn store_thumb(app: &tauri::AppHandle, key: &str, bytes: &[u8]) {
    if let Some(dir) = cache_dir(app) {
        let _ = std::fs::write(dir.join(format!("{key}.jpg")), bytes);
    }
}

/// Reescala unos bytes de imagen a miniatura JPEG.
fn make_thumb(raw: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(raw).ok()?;
    let thumb = img.thumbnail(THUMB_MAX_DIM, THUMB_MAX_DIM);
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, THUMB_QUALITY);
    encoder.encode_image(&thumb.to_rgb8()).ok()?;
    Some(buf.into_inner())
}

/// Miniatura de la primera página de un cómic, pasando por la caché en disco.
fn comic_thumb(app: &tauri::AppHandle, path: &Path) -> Result<Option<Vec<u8>>, String> {
    let key = cache_key(path);
    if let Some(k) = &key {
        if let Some(bytes) = cached_thumb(app, k) {
            return Ok(Some(bytes));
        }
    }

    let pages = archive::list_pages(path)?;
    let Some(first) = pages.first() else {
        return Ok(None);
    };
    let raw = archive::read_page(path, first)?;
    let Some(thumb) = make_thumb(&raw) else {
        return Err("no se pudo decodificar la primera página".into());
    };

    if let Some(k) = &key {
        store_thumb(app, k, &thumb);
    }
    Ok(Some(thumb))
}

/// Portada manual de una carpeta: `cover.jpg`, `folder.png`, etc.
fn manual_folder_cover(app: &tauri::AppHandle, dir: &Path) -> Option<Vec<u8>> {
    for stem in FOLDER_COVER_STEMS {
        for ext in FOLDER_COVER_EXTS {
            let candidate = dir.join(format!("{stem}.{ext}"));
            if !candidate.is_file() {
                continue;
            }
            let key = cache_key(&candidate);
            if let Some(k) = &key {
                if let Some(bytes) = cached_thumb(app, k) {
                    return Some(bytes);
                }
            }
            if let Ok(raw) = std::fs::read(&candidate) {
                if let Some(thumb) = make_thumb(&raw) {
                    if let Some(k) = &key {
                        store_thumb(app, k, &thumb);
                    }
                    return Some(thumb);
                }
            }
        }
    }
    None
}

/// ¿Merece la pena mostrar esta carpeta? Una sola lectura del directorio, sin
/// recorrer el subárbol: basta con que tenga un cómic o alguna subcarpeta que
/// pueda tenerlo. El recuento real se calcula después, ya en diferido.
fn worth_listing(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(meta) = entry.path().symlink_metadata() else {
            continue;
        };
        if meta.is_dir() || (meta.is_file() && archive::is_comic(&entry.path())) {
            return true;
        }
    }
    false
}

/// Recorre una carpeta y devuelve (primer cómic, total de cómics).
/// No sigue enlaces simbólicos: es el comportamiento por defecto de walkdir y
/// evita bucles y salidas fuera de la biblioteca.
fn folder_stats(dir: &Path) -> (Option<PathBuf>, usize) {
    let mut first: Option<PathBuf> = None;
    let mut count = 0usize;

    let walker = walkdir::WalkDir::new(dir).sort_by(|a, b| {
        archive::natural_cmp(
            &a.file_name().to_string_lossy(),
            &b.file_name().to_string_lossy(),
        )
    });

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && archive::is_comic(entry.path()) {
            count += 1;
            if first.is_none() {
                first = Some(entry.path().to_path_buf());
            }
        }
    }

    (first, count)
}

fn display_name(path: &Path, strip_ext: bool) -> String {
    let name = if strip_ext {
        path.file_stem()
    } else {
        path.file_name()
    };
    name.map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Comprueba que `path` cae dentro de `root`. Sin esto, cualquier ruta que
/// llegara del frontend permitiría recorrer el disco entero.
fn resolve_within(root: &str, path: Option<String>) -> Result<(PathBuf, PathBuf), String> {
    let root = PathBuf::from(root)
        .canonicalize()
        .map_err(|_| "la carpeta de la biblioteca no existe".to_string())?;

    let current = match path {
        Some(p) => PathBuf::from(p)
            .canonicalize()
            .map_err(|_| "la carpeta no existe".to_string())?,
        None => root.clone(),
    };

    if !current.starts_with(&root) {
        return Err("ruta fuera de la biblioteca".into());
    }
    Ok((root, current))
}

/// Lista un nivel de la biblioteca. Solo toca el sistema de ficheros: no abre
/// ningún cómic, así que responde igual de rápido con 10 que con 10.000.
#[tauri::command]
pub fn list_dir(root: String, path: Option<String>) -> Result<DirListing, String> {
    let (root, current) = resolve_within(&root, path)?;
    if !current.is_dir() {
        return Err("la ruta no es una carpeta".into());
    }

    let mut folders: Vec<FolderMeta> = Vec::new();
    let mut comics: Vec<ComicMeta> = Vec::new();

    let entries = std::fs::read_dir(&current).map_err(|e| format!("no se pudo leer: {e}"))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        // symlink_metadata: no seguimos enlaces, para no salir de la biblioteca.
        let Ok(meta) = p.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            if worth_listing(&p) {
                folders.push(FolderMeta {
                    path: p.to_string_lossy().to_string(),
                    name: display_name(&p, false),
                });
            }
        } else if meta.is_file() && archive::is_comic(&p) {
            // Aquí no se abre el archivo: un cómic corrupto entra igual en la
            // lista y ya informará get_comic_info de lo que le pasa.
            comics.push(ComicMeta {
                path: p.to_string_lossy().to_string(),
                name: display_name(&p, true),
            });
        }
    }

    folders.sort_by(|a, b| archive::natural_cmp(&a.name, &b.name));
    comics.sort_by(|a, b| archive::natural_cmp(&a.name, &b.name));

    let parent = if current == root {
        None
    } else {
        current.parent().map(|p| p.to_string_lossy().to_string())
    };

    Ok(DirListing {
        path: current.to_string_lossy().to_string(),
        parent,
        folders,
        comics,
    })
}

/// Portada y número de páginas de un cómic. Se pide en diferido, una por
/// tarjeta, para no bloquear el listado.
#[tauri::command]
pub async fn get_comic_info(
    app: tauri::AppHandle,
    root: String,
    path: String,
) -> Result<ComicInfo, String> {
    let (_, file) = resolve_within(&root, Some(path))?;

    // spawn_blocking: descomprimir y decodificar es trabajo intensivo de CPU y
    // no debe ocupar el hilo del runtime asíncrono.
    tauri::async_runtime::spawn_blocking(move || {
        let page_count = archive::list_pages(&file).map(|p| p.len()).unwrap_or(0);

        match comic_thumb(&app, &file) {
            Ok(thumb) => ComicInfo {
                cover: thumb.as_deref().map(to_data_uri),
                page_count,
                error: if page_count == 0 {
                    Some("el archivo no contiene páginas legibles".into())
                } else {
                    None
                },
            },
            Err(e) => ComicInfo {
                cover: None,
                page_count,
                error: Some(e),
            },
        }
    })
    .await
    .map_err(|e| format!("fallo al procesar el cómic: {e}"))
}

/// Portada y recuento de una carpeta, también en diferido.
#[tauri::command]
pub async fn get_folder_info(
    app: tauri::AppHandle,
    root: String,
    path: String,
) -> Result<FolderInfo, String> {
    let (_, dir) = resolve_within(&root, Some(path))?;

    tauri::async_runtime::spawn_blocking(move || {
        let (first_comic, comic_count) = folder_stats(&dir);

        let cover = manual_folder_cover(&app, &dir).or_else(|| {
            first_comic
                .as_deref()
                .and_then(|c| comic_thumb(&app, c).ok().flatten())
        });

        FolderInfo {
            cover: cover.as_deref().map(to_data_uri),
            comic_count,
        }
    })
    .await
    .map_err(|e| format!("fallo al leer la carpeta: {e}"))
}

#[tauri::command]
pub fn open_comic(path: String) -> Result<Vec<String>, String> {
    archive::list_pages(Path::new(&path))
}

#[tauri::command]
pub fn get_page(path: String, name: String) -> Result<String, String> {
    let bytes = archive::read_page(Path::new(&path), &name)?;
    Ok(format!(
        "data:{};base64,{}",
        archive::mime_for(&name),
        STANDARD.encode(&bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(
            std::env::var("READINGCOMICS_TEST_LIB")
                .expect("define READINGCOMICS_TEST_LIB con la ruta de la biblioteca de prueba"),
        )
    }

    /// Un cómic ilegible debe seguir apareciendo en el listado. Antes se
    /// descartaba en silencio y el usuario veía un hueco sin explicación.
    #[test]
    fn broken_comics_are_still_listed() {
        let root = fixture_root();
        let serie = root.join("Serie A");

        let listing = list_dir(
            root.to_string_lossy().to_string(),
            Some(serie.to_string_lossy().to_string()),
        )
        .expect("list_dir debería funcionar");

        let names: Vec<&str> = listing.comics.iter().map(|c| c.name.as_str()).collect();

        assert!(names.contains(&"bueno"), "falta el cómic sano: {names:?}");
        assert!(names.contains(&"corrupto"), "el cómic corrupto desapareció: {names:?}");
        assert!(names.contains(&"imagen_rota"), "falta imagen_rota: {names:?}");
        assert!(names.contains(&"sin_paginas"), "falta sin_paginas: {names:?}");
        assert_eq!(names.len(), 4);
    }

    /// Las carpetas sin nada dentro no se listan, pero las que tienen cómics sí.
    #[test]
    fn empty_folders_are_hidden() {
        let root = fixture_root();
        let listing = list_dir(root.to_string_lossy().to_string(), None).unwrap();
        let names: Vec<&str> = listing.folders.iter().map(|f| f.name.as_str()).collect();

        assert!(names.contains(&"Serie A"));
        assert!(!names.contains(&"vacia"), "carpeta vacía no debería listarse");
    }

    /// No se puede salir de la raíz elegida.
    #[test]
    fn cannot_escape_root() {
        let root = fixture_root();
        let outside = root.parent().unwrap().to_string_lossy().to_string();

        let result = list_dir(root.to_string_lossy().to_string(), Some(outside));
        assert!(result.is_err(), "debería rechazar rutas fuera de la biblioteca");
    }
}
