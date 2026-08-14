use std::cmp::Ordering;
use std::io::Read;
use std::path::Path;

const IMAGE_EXTS: [&str; 6] = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];

fn ext_of(name: &str) -> String {
    name.to_lowercase()
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_string()
}

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    if lower.contains("__macosx") || base.starts_with('.') {
        return false;
    }
    IMAGE_EXTS.contains(&ext_of(name).as_str())
}

pub fn mime_for(name: &str) -> &'static str {
    match ext_of(name).as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Natural sort so "page2.jpg" comes before "page10.jpg".
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ac = a.chars().peekable();
    let mut bc = b.chars().peekable();
    loop {
        match (ac.peek(), bc.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut na = String::new();
                    while let Some(&c) = ac.peek() {
                        if c.is_ascii_digit() {
                            na.push(c);
                            ac.next();
                        } else {
                            break;
                        }
                    }
                    let mut nb = String::new();
                    while let Some(&c) = bc.peek() {
                        if c.is_ascii_digit() {
                            nb.push(c);
                            bc.next();
                        } else {
                            break;
                        }
                    }
                    let numa: u64 = na.parse().unwrap_or(0);
                    let numb: u64 = nb.parse().unwrap_or(0);
                    match numa.cmp(&numb) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    match ca.cmp(&cb) {
                        Ordering::Equal => {
                            ac.next();
                            bc.next();
                            continue;
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

pub fn is_comic(path: &Path) -> bool {
    matches!(ext_of(path.to_string_lossy().as_ref()).as_str(), "cbz" | "cbr")
}

/// Sorted list of image entry names inside the archive.
pub fn list_pages(path: &Path) -> Result<Vec<String>, String> {
    match ext_of(path.to_string_lossy().as_ref()).as_str() {
        "cbz" => list_pages_zip(path),
        "cbr" => list_pages_rar(path),
        other => Err(format!("unsupported archive type: {other}")),
    }
}

/// Raw bytes of a single page, looked up by its entry name.
pub fn read_page(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    match ext_of(path.to_string_lossy().as_ref()).as_str() {
        "cbz" => read_page_zip(path, name),
        "cbr" => read_page_rar(path, name),
        other => Err(format!("unsupported archive type: {other}")),
    }
}

fn list_pages_zip(path: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut names: Vec<String> = archive
        .file_names()
        .filter(|n| is_image(n))
        .map(|n| n.to_string())
        .collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    Ok(names)
}

fn read_page_zip(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = archive.by_name(name).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn list_pages_rar(path: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut archive = unrar::Archive::new(path)
        .open_for_listing()
        .map_err(|e| e.to_string())?;
    loop {
        match archive.read_header() {
            Ok(Some(next)) => {
                let entry = next.entry();
                let name = entry.filename.to_string_lossy().replace('\\', "/");
                if !entry.is_directory() && is_image(&name) {
                    names.push(name);
                }
                archive = next.skip().map_err(|e| e.to_string())?;
            }
            Ok(None) => break,
            Err(e) => return Err(e.to_string()),
        }
    }
    names.sort_by(|a, b| natural_cmp(a, b));
    Ok(names)
}

fn read_page_rar(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let mut archive = unrar::Archive::new(path)
        .open_for_processing()
        .map_err(|e| e.to_string())?;
    loop {
        match archive.read_header() {
            Ok(Some(next)) => {
                let entry_name = next.entry().filename.to_string_lossy().replace('\\', "/");
                if entry_name == name {
                    let (data, _rest) = next.read().map_err(|e| e.to_string())?;
                    return Ok(data);
                }
                archive = next.skip().map_err(|e| e.to_string())?;
            }
            Ok(None) => return Err(format!("page not found: {name}")),
            Err(e) => return Err(e.to_string()),
        }
    }
}
