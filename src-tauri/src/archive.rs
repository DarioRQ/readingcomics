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

/// Formato real del archivo, que no tiene por qué ser el que dice la extensión.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Zip,
    Rar,
    SevenZip,
    Unknown,
}

/// Averigua el formato mirando los primeros bytes, no la extensión.
///
/// Un `.cbr` que en realidad es un zip es de lo más corriente: mucha gente
/// reempaqueta el contenido y conserva el nombre. Los lectores de toda la vida
/// (CDisplayEx, Komga…) abren esos ficheros porque miran el contenido; fiarse
/// de la extensión era justo el motivo de que aquí salieran como ilegibles.
pub fn detect(path: &Path) -> Format {
    use std::io::Read as _;

    let mut head = [0u8; 8];
    let read = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    let head = &head[..read];

    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        return Format::Zip;
    }
    // RAR 4 termina la firma en 0x00; RAR 5, en 0x01 0x00.
    if head.starts_with(b"Rar!\x1a\x07") {
        return Format::Rar;
    }
    if head.starts_with(b"7z\xbc\xaf\x27\x1c") {
        return Format::SevenZip;
    }

    // Sin firma reconocible, la extensión es lo único que queda.
    match ext_of(path.to_string_lossy().as_ref()).as_str() {
        "cbz" => Format::Zip,
        "cbr" => Format::Rar,
        _ => Format::Unknown,
    }
}

/// Explica en castellano por qué un archivo no se puede abrir, para que el
/// aviso de la biblioteca diga algo más útil que "no se pudo leer".
fn unsupported(format: Format) -> String {
    match format {
        Format::SevenZip => {
            "el archivo es un 7-Zip, un formato que la aplicación todavía no lee".into()
        }
        _ => "el archivo no es un zip ni un rar; puede estar incompleto o corrupto".into(),
    }
}

/// Sorted list of image entry names inside the archive.
pub fn list_pages(path: &Path) -> Result<Vec<String>, String> {
    match detect(path) {
        Format::Zip => list_pages_zip(path),
        Format::Rar => list_pages_rar(path),
        other => Err(unsupported(other)),
    }
}

/// Raw bytes of a single page, looked up by its entry name.
pub fn read_page(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    match detect(path) {
        Format::Zip => read_page_zip(path, name),
        Format::Rar => read_page_rar(path, name),
        other => Err(unsupported(other)),
    }
}

/// Varias páginas de una vez, abriendo el archivo **una sola vez**.
///
/// Existe por la tira de miniaturas: pedirlas de una en una con `read_page`
/// significaría reabrir el archivo por cada una, y en CBR cada apertura obliga
/// a recorrer las cabeceras hasta dar con la entrada. Se devuelve lo que se
/// haya podido leer; las que fallen simplemente no salen.
pub fn read_pages(path: &Path, names: &[String]) -> Result<Vec<(String, Vec<u8>)>, String> {
    match detect(path) {
        Format::Zip => {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let mut out = Vec::with_capacity(names.len());
            for name in names {
                let Ok(mut entry) = archive.by_name(name) else {
                    continue;
                };
                let mut buf = Vec::new();
                if entry.read_to_end(&mut buf).is_ok() {
                    out.push((name.clone(), buf));
                }
            }
            Ok(out)
        }
        Format::Rar => {
            // Una sola pasada secuencial: el rar no tiene acceso directo.
            let mut out = Vec::with_capacity(names.len());
            let mut archive = unrar::Archive::new(path)
                .open_for_processing()
                .map_err(rar_error)?;
            while let Ok(Some(next)) = archive.read_header() {
                let entry_name = next.entry().filename.to_string_lossy().replace('\\', "/");
                if names.contains(&entry_name) {
                    let (data, rest) = next.read().map_err(rar_error)?;
                    out.push((entry_name, data));
                    archive = rest;
                } else {
                    archive = next.skip().map_err(rar_error)?;
                }
                if out.len() == names.len() {
                    break;
                }
            }
            Ok(out)
        }
        other => Err(unsupported(other)),
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

/// Traduce el error del descompresor de rar. Sus mensajes originales son
/// códigos secos en inglés, y este texto acaba en la tarjeta del cómic.
fn rar_error(e: unrar::error::UnrarError) -> String {
    use unrar::error::Code;
    match e.code {
        Code::MissingPassword | Code::BadPassword => {
            "el archivo está protegido con contraseña".into()
        }
        Code::BadArchive | Code::UnknownFormat => {
            "el archivo está dañado o usa una variante de rar que no se reconoce".into()
        }
        Code::BadData => "el archivo tiene datos corruptos".into(),
        Code::EOpen | Code::ERead => "no se pudo leer el archivo del disco".into(),
        Code::NoMemory => "no hay memoria suficiente para abrir el archivo".into(),
        _ => format!("el lector de rar falló ({e})"),
    }
}

fn list_pages_rar(path: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut archive = unrar::Archive::new(path)
        .open_for_listing()
        .map_err(rar_error)?;
    loop {
        match archive.read_header() {
            Ok(Some(next)) => {
                let entry = next.entry();
                let name = entry.filename.to_string_lossy().replace('\\', "/");
                if !entry.is_directory() && is_image(&name) {
                    names.push(name);
                }
                archive = next.skip().map_err(rar_error)?;
            }
            Ok(None) => break,
            Err(e) => return Err(rar_error(e)),
        }
    }
    names.sort_by(|a, b| natural_cmp(a, b));
    Ok(names)
}

fn read_page_rar(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let mut archive = unrar::Archive::new(path)
        .open_for_processing()
        .map_err(rar_error)?;
    loop {
        match archive.read_header() {
            Ok(Some(next)) => {
                let entry_name = next.entry().filename.to_string_lossy().replace('\\', "/");
                if entry_name == name {
                    let (data, _rest) = next.read().map_err(rar_error)?;
                    return Ok(data);
                }
                archive = next.skip().map_err(rar_error)?;
            }
            Ok(None) => return Err(format!("page not found: {name}")),
            Err(e) => return Err(rar_error(e)),
        }
    }
}

/// Nombre del fichero de metadatos que muchas releases incluyen dentro del
/// propio CBZ/CBR. Es el estándar de ComicRack, adoptado de facto por todo el
/// ecosistema (Komga, Kavita, ComicTagger...).
const METADATA_ENTRY: &str = "comicinfo.xml";

fn is_metadata_entry(name: &str) -> bool {
    name.to_lowercase()
        .rsplit('/')
        .next()
        .map(|base| base == METADATA_ENTRY)
        .unwrap_or(false)
}

/// Devuelve el ComicInfo.xml del archivo, si lo trae.
pub fn read_metadata(path: &Path) -> Option<Vec<u8>> {
    match detect(path) {
        Format::Zip => read_metadata_zip(path),
        Format::Rar => read_metadata_rar(path),
        _ => None,
    }
}

fn read_metadata_zip(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let name = archive
        .file_names()
        .find(|n| is_metadata_entry(n))?
        .to_string();
    let mut entry = archive.by_name(&name).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Escribe un zip con una página dentro, con el nombre que se le pida.
    fn write_zip(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("001.jpg", options).unwrap();
        zip.write_all(b"no es un jpeg de verdad, pero ocupa sitio")
            .unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn a_cbr_that_is_really_a_zip_still_opens() {
        // El caso real: cómics reempaquetados como zip que conservan el nombre
        // `.cbr`. Fiándose de la extensión salían como ilegibles, mientras que
        // cualquier otro lector los abre sin rechistar.
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("Doctor Strange 001.cbr");
        write_zip(&fake);

        assert_eq!(detect(&fake), Format::Zip);
        assert_eq!(list_pages(&fake).unwrap(), vec!["001.jpg".to_string()]);
        assert!(!read_page(&fake, "001.jpg").unwrap().is_empty());
    }

    #[test]
    fn the_extension_still_decides_when_there_is_no_signature() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("vacio.cbz");
        std::fs::write(&empty, b"").unwrap();
        assert_eq!(detect(&empty), Format::Zip);
    }

    #[test]
    fn other_formats_say_what_they_are() {
        let dir = tempfile::tempdir().unwrap();
        let seven = dir.path().join("comic.cbr");
        std::fs::write(&seven, b"7z\xbc\xaf\x27\x1c\x00\x04").unwrap();

        assert_eq!(detect(&seven), Format::SevenZip);
        let err = list_pages(&seven).unwrap_err();
        assert!(err.contains("7-Zip"), "{err}");
    }
}

fn read_metadata_rar(path: &Path) -> Option<Vec<u8>> {
    let mut archive = unrar::Archive::new(path).open_for_processing().ok()?;
    loop {
        match archive.read_header() {
            Ok(Some(next)) => {
                let name = next.entry().filename.to_string_lossy().replace('\\', "/");
                if is_metadata_entry(&name) {
                    let (data, _rest) = next.read().ok()?;
                    return Some(data);
                }
                archive = next.skip().ok()?;
            }
            _ => return None,
        }
    }
}
