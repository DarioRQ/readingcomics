use crate::archive;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
pub struct ComicMeta {
    pub path: String,
    pub name: String,
    pub cover: Option<String>,
    pub page_count: usize,
}

const THUMB_MAX_DIM: u32 = 340;

fn to_data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

fn build_meta(path: &std::path::Path) -> Option<ComicMeta> {
    let pages = archive::list_pages(path).ok()?;
    let cover = pages.first().and_then(|first| {
        let raw = archive::read_page(path, first).ok()?;
        let img = image::load_from_memory(&raw).ok()?;
        let thumb = img.thumbnail(THUMB_MAX_DIM, THUMB_MAX_DIM);
        let mut buf = std::io::Cursor::new(Vec::new());
        thumb
            .to_rgb8()
            .write_to(&mut buf, image::ImageFormat::Jpeg)
            .ok()?;
        Some(to_data_uri("image/jpeg", buf.get_ref()))
    });
    Some(ComicMeta {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        cover,
        page_count: pages.len(),
    })
}

#[tauri::command]
pub fn scan_library(root: String) -> Result<Vec<ComicMeta>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("la carpeta no existe".into());
    }
    let mut out: Vec<ComicMeta> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && archive::is_comic(e.path()))
        .filter_map(|e| build_meta(e.path()))
        .collect();
    out.sort_by(|a, b| archive::natural_cmp(&a.name, &b.name));
    Ok(out)
}

#[tauri::command]
pub fn open_comic(path: String) -> Result<Vec<String>, String> {
    archive::list_pages(std::path::Path::new(&path))
}

#[tauri::command]
pub fn get_page(path: String, name: String) -> Result<String, String> {
    let bytes = archive::read_page(std::path::Path::new(&path), &name)?;
    Ok(to_data_uri(archive::mime_for(&name), &bytes))
}
