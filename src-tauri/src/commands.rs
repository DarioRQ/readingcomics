use crate::archive;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct ComicMeta {
    pub path: String,
    pub name: String,
    pub cover: Option<String>,
    pub page_count: usize,
}

#[derive(Serialize, Clone)]
pub struct FolderMeta {
    pub path: String,
    pub name: String,
    pub cover: Option<String>,
    pub comic_count: usize,
}

/// Contenido de un nivel de la biblioteca. `parent` es `None` en la raíz, que
/// es lo que impide navegar por encima de la carpeta elegida.
#[derive(Serialize, Clone)]
pub struct DirListing {
    pub path: String,
    pub parent: Option<String>,
    pub folders: Vec<FolderMeta>,
    pub comics: Vec<ComicMeta>,
}

const THUMB_MAX_DIM: u32 = 340;

/// Nombres de fichero que se usan como portada manual de una carpeta, en orden
/// de preferencia. Es la convención de toda la vida (`folder.jpg` de Windows),
/// así el usuario asigna icono sin que la app invente formatos propios.
const FOLDER_COVER_STEMS: [&str; 2] = ["cover", "folder"];
const FOLDER_COVER_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

fn to_data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

/// Reescala unos bytes de imagen a miniatura JPEG en data URI.
fn thumbnail(raw: &[u8]) -> Option<String> {
    let img = image::load_from_memory(raw).ok()?;
    let thumb = img.thumbnail(THUMB_MAX_DIM, THUMB_MAX_DIM);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .to_rgb8()
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .ok()?;
    Some(to_data_uri("image/jpeg", buf.get_ref()))
}

fn comic_cover(path: &Path) -> Option<String> {
    let pages = archive::list_pages(path).ok()?;
    let first = pages.first()?;
    let raw = archive::read_page(path, first).ok()?;
    thumbnail(&raw)
}

fn build_meta(path: &Path) -> Option<ComicMeta> {
    let pages = archive::list_pages(path).ok()?;
    Some(ComicMeta {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        cover: comic_cover(path),
        page_count: pages.len(),
    })
}

/// Portada manual de una carpeta: `cover.jpg`, `folder.png`, etc.
fn manual_folder_cover(dir: &Path) -> Option<String> {
    for stem in FOLDER_COVER_STEMS {
        for ext in FOLDER_COVER_EXTS {
            let candidate = dir.join(format!("{stem}.{ext}"));
            if candidate.is_file() {
                if let Ok(raw) = std::fs::read(&candidate) {
                    if let Some(uri) = thumbnail(&raw) {
                        return Some(uri);
                    }
                }
            }
        }
    }
    None
}

/// Recorre una carpeta una sola vez y devuelve (primer cómic, total de cómics).
/// No sigue enlaces simbólicos, que es el comportamiento por defecto de walkdir
/// y evita bucles y salidas fuera de la biblioteca.
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

fn build_folder_meta(dir: &Path) -> Option<FolderMeta> {
    let (first_comic, comic_count) = folder_stats(dir);

    // Una carpeta sin ningún cómic dentro no pinta nada en la biblioteca.
    if comic_count == 0 {
        return None;
    }

    let cover = manual_folder_cover(dir).or_else(|| first_comic.as_deref().and_then(comic_cover));

    Some(FolderMeta {
        path: dir.to_string_lossy().to_string(),
        name: dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string()),
        cover,
        comic_count,
    })
}

/// Lista un nivel de la biblioteca: subcarpetas y cómics sueltos de `path`.
///
/// `path` se valida siempre contra `root`: sin esa comprobación, cualquier
/// ruta que llegara desde el frontend permitiría listar el disco entero.
#[tauri::command]
pub fn list_dir(root: String, path: Option<String>) -> Result<DirListing, String> {
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
    if !current.is_dir() {
        return Err("la ruta no es una carpeta".into());
    }

    let mut folders: Vec<FolderMeta> = Vec::new();
    let mut comics: Vec<ComicMeta> = Vec::new();

    let entries = std::fs::read_dir(&current).map_err(|e| format!("no se pudo leer: {e}"))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        // symlink_metadata: no seguimos enlaces, para no salir de la biblioteca.
        let Ok(meta) = entry.path().symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            if let Some(folder) = build_folder_meta(&p) {
                folders.push(folder);
            }
        } else if meta.is_file() && archive::is_comic(&p) {
            if let Some(comic) = build_meta(&p) {
                comics.push(comic);
            }
        }
    }

    folders.sort_by(|a, b| archive::natural_cmp(&a.name, &b.name));
    comics.sort_by(|a, b| archive::natural_cmp(&a.name, &b.name));

    let parent = if current == root {
        None
    } else {
        current
            .parent()
            .map(|p| p.to_string_lossy().to_string())
    };

    Ok(DirListing {
        path: current.to_string_lossy().to_string(),
        parent,
        folders,
        comics,
    })
}

#[tauri::command]
pub fn open_comic(path: String) -> Result<Vec<String>, String> {
    archive::list_pages(Path::new(&path))
}

#[tauri::command]
pub fn get_page(path: String, name: String) -> Result<String, String> {
    let bytes = archive::read_page(Path::new(&path), &name)?;
    Ok(to_data_uri(archive::mime_for(&name), &bytes))
}
