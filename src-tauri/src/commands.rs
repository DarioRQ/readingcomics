use crate::archive;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
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
    /// Metadatos del ComicInfo.xml incrustado, si el archivo lo trae.
    pub meta: Option<crate::comicinfo::ComicInfoXml>,
}

/// Estado de una colección dentro de una carpeta.
#[derive(Serialize, Clone, Default)]
pub struct SeriesInfo {
    pub series: Option<String>,
    pub publisher: Option<String>,
    /// Volumen y año de la edición, cuando se conocen. No se muestran: sirven
    /// para distinguir entre los varios volúmenes que comparten cabecera al
    /// buscar la serie en Metron.
    pub volume: Option<u32>,
    pub year: Option<i32>,
    /// Total de números que declara ComicInfo.xml, si lo declara.
    pub total: Option<u32>,
    /// Números que tienes, ordenados.
    pub owned: Vec<u32>,
    /// Huecos detectados entre el primero y el último que tienes, más los que
    /// falten hasta `total` si se conoce.
    pub missing: Vec<u32>,
    /// Cuántos cómics traían metadatos y cuántos no.
    pub tagged: usize,
    pub untagged: usize,
    /// `true` si la serie y los números salen del nombre de los ficheros en vez
    /// de metadatos incrustados. La interfaz lo advierte: es una deducción.
    pub guessed: bool,
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

/// Lo que se guarda junto a la miniatura para no volver a abrir el archivo.
///
/// Cachear solo la imagen no bastaba: el número de páginas obligaba a abrir y
/// recorrer el cómic en cada visita, que en CBR es justo la parte cara porque
/// hay que escanear todas las cabeceras. El error también se cachea, para no
/// reintentar en bucle los archivos rotos.
#[derive(Serialize, Deserialize, Clone)]
struct CachedMeta {
    page_count: usize,
    has_cover: bool,
    error: Option<String>,
    #[serde(default)]
    meta: Option<crate::comicinfo::ComicInfoXml>,
}

fn cached_meta_as<T: serde::de::DeserializeOwned>(
    app: &tauri::AppHandle,
    key: &str,
) -> Option<T> {
    let path = cache_dir(app)?.join(format!("{key}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn store_meta_as<T: Serialize>(app: &tauri::AppHandle, key: &str, value: &T) {
    if let Some(dir) = cache_dir(app) {
        if let Ok(json) = serde_json::to_string(value) {
            let _ = std::fs::write(dir.join(format!("{key}.json")), json);
        }
    }
}

fn cached_meta(app: &tauri::AppHandle, key: &str) -> Option<CachedMeta> {
    cached_meta_as(app, key)
}

fn store_meta(app: &tauri::AppHandle, key: &str, meta: &CachedMeta) {
    store_meta_as(app, key, meta)
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

/// Portada y metadatos de un cómic, abriendo el archivo una sola vez y solo
/// cuando no está ya en caché.
fn comic_info_cached(app: &tauri::AppHandle, path: &Path) -> ComicInfo {
    let key = cache_key(path);

    // Acierto de caché: ni se toca el archivo.
    if let Some(k) = &key {
        if let Some(meta) = cached_meta(app, k) {
            return ComicInfo {
                cover: if meta.has_cover {
                    cached_thumb(app, k).as_deref().map(to_data_uri)
                } else {
                    None
                },
                page_count: meta.page_count,
                error: meta.error,
                meta: meta.meta,
            };
        }
    }

    let store = |meta: CachedMeta| {
        if let Some(k) = &key {
            store_meta(app, k, &meta);
        }
        meta
    };

    // Una sola apertura: de aquí salen tanto el recuento como la portada.
    let pages = match archive::list_pages(path) {
        Ok(p) => p,
        Err(e) => {
            let meta = store(CachedMeta {
                page_count: 0,
                has_cover: false,
                error: Some(e.clone()),
                meta: None,
            });
            return ComicInfo {
                cover: None,
                page_count: 0,
                error: meta.error,
                meta: None,
            };
        }
    };

    let page_count = pages.len();
    let Some(first) = pages.first() else {
        store(CachedMeta {
            page_count: 0,
            has_cover: false,
            error: Some("el archivo no contiene páginas legibles".into()),
            meta: None,
        });
        return ComicInfo {
            cover: None,
            page_count: 0,
            error: Some("el archivo no contiene páginas legibles".into()),
            meta: None,
        };
    };

    let thumb = archive::read_page(path, first)
        .ok()
        .and_then(|raw| make_thumb(&raw));

    if let (Some(k), Some(bytes)) = (&key, &thumb) {
        store_thumb(app, k, bytes);
    }

    let error = if thumb.is_none() {
        Some("no se pudo decodificar la portada".to_string())
    } else {
        None
    };

    // ComicInfo.xml: una lectura extra del archivo, pero solo la primera vez;
    // a partir de ahí sale de la caché junto al resto.
    let xml = crate::comicinfo::read(path);

    store(CachedMeta {
        page_count,
        has_cover: thumb.is_some(),
        error: error.clone(),
        meta: xml.clone(),
    });

    ComicInfo {
        cover: thumb.as_deref().map(to_data_uri),
        page_count,
        error,
        meta: xml,
    }
}

/// Solo la miniatura, para usar la portada de un cómic como portada de carpeta.
fn comic_thumb(app: &tauri::AppHandle, path: &Path) -> Option<Vec<u8>> {
    let key = cache_key(path);
    if let Some(k) = &key {
        if let Some(bytes) = cached_thumb(app, k) {
            return Some(bytes);
        }
        // Si ya sabemos que no tiene portada, no se reabre el archivo.
        if let Some(meta) = cached_meta(app, k) {
            if !meta.has_cover {
                return None;
            }
        }
    }

    let pages = archive::list_pages(path).ok()?;
    let raw = archive::read_page(path, pages.first()?).ok()?;
    let thumb = make_thumb(&raw)?;
    if let Some(k) = &key {
        store_thumb(app, k, &thumb);
    }
    Some(thumb)
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

/// Recuento de una carpeta ya calculado.
///
/// Se cachea porque contar obliga a recorrer todo el subárbol, y eso se repetía
/// en cada visita. La clave incluye la fecha de la propia carpeta: si añades o
/// quitas cómics **directamente** en ella, el recuento se recalcula solo. Un
/// cambio en una subcarpeta anidada no la invalida — el precio de no volver a
/// recorrer miles de ficheros cada vez que se mira una estantería.
#[derive(Serialize, Deserialize, Clone)]
struct CachedFolder {
    comic_count: usize,
    first_comic: Option<String>,
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
    tauri::async_runtime::spawn_blocking(move || comic_info_cached(&app, &file))
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
        let key = cache_key(&dir).map(|k| format!("dir-{k}"));

        // El recorrido del subárbol solo se hace si no está ya calculado.
        let cached: Option<CachedFolder> = key
            .as_deref()
            .and_then(|k| cached_meta_as::<CachedFolder>(&app, k));

        let (first_comic, comic_count) = match cached {
            Some(c) => (c.first_comic.map(PathBuf::from), c.comic_count),
            None => {
                let (first, count) = folder_stats(&dir);
                if let Some(k) = &key {
                    store_meta_as(
                        &app,
                        k,
                        &CachedFolder {
                            comic_count: count,
                            first_comic: first
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string()),
                        },
                    );
                }
                (first, count)
            }
        };

        let cover = manual_folder_cover(&app, &dir)
            .or_else(|| first_comic.as_deref().and_then(|c| comic_thumb(&app, c)));

        FolderInfo {
            cover: cover.as_deref().map(to_data_uri),
            comic_count,
        }
    })
    .await
    .map_err(|e| format!("fallo al leer la carpeta: {e}"))
}

/// Analiza los cómics de una carpeta y deduce qué colección es y si está
/// completa, usando el ComicInfo.xml incrustado en los archivos.
///
/// Solo mira los cómics de ese nivel, no del subárbol: una carpeta suele ser
/// una serie, y mezclar niveles daría recuentos sin sentido.
#[tauri::command]
pub async fn get_series_info(
    app: tauri::AppHandle,
    root: String,
    path: String,
) -> Result<SeriesInfo, String> {
    let (_, dir) = resolve_within(&root, Some(path))?;

    tauri::async_runtime::spawn_blocking(move || {
        use std::collections::HashMap;

        let mut owned: Vec<u32> = Vec::new();
        let mut declared_total: Option<u32> = None;
        let mut publisher: Option<String> = None;
        let mut volume: Option<u32> = None;
        let mut year: Option<i32> = None;
        // La serie se decide por mayoría: alguna release suelta puede traer el
        // nombre mal escrito y no debe cambiar el de toda la carpeta.
        let mut series_votes: HashMap<String, usize> = HashMap::new();
        let mut guessed_series: HashMap<String, usize> = HashMap::new();
        let mut guessed_numbers: Vec<u32> = Vec::new();
        let (mut tagged, mut untagged) = (0usize, 0usize);

        let Ok(entries) = std::fs::read_dir(&dir) else {
            return SeriesInfo::default();
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() || !archive::is_comic(&p) {
                continue;
            }

            match comic_info_cached(&app, &p).meta {
                Some(m) => {
                    tagged += 1;
                    if let Some(s) = m.series.as_deref() {
                        *series_votes.entry(s.to_string()).or_default() += 1;
                    }
                    if publisher.is_none() {
                        publisher = m.publisher.clone();
                    }
                    if volume.is_none() {
                        volume = m.volume.filter(|v| *v > 0).map(|v| v as u32);
                    }
                    if year.is_none() {
                        year = m.year.filter(|y| (1900..=2100).contains(y));
                    }
                    // Se queda el total más alto declarado: si una release trae
                    // el dato desactualizado, no debe recortar la serie.
                    if let Some(c) = m.count.filter(|c| *c > 0) {
                        let c = c as u32;
                        declared_total = Some(declared_total.map_or(c, |d: u32| d.max(c)));
                    }
                    if let Some(n) = m.number.as_deref().and_then(crate::comicinfo::issue_number)
                    {
                        owned.push(n);
                    }
                }
                None => {
                    untagged += 1;
                    // Sin metadatos, se deduce del nombre del fichero. Es lo
                    // único que permite detectar colecciones en bibliotecas sin
                    // etiquetar, que son la mayoría.
                    let stem = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let guess = crate::comicinfo::guess_from_filename(&stem);
                    if let Some(n) = guess.number {
                        guessed_numbers.push(n);
                    }
                    if let Some(sname) = guess.series {
                        *guessed_series.entry(sname).or_default() += 1;
                    }
                    volume = volume.or(guess.volume);
                    year = year.or(guess.year);
                }
            }
        }

        // Si no hubo ni un metadato, se tira de lo deducido del nombre.
        let guessed = owned.is_empty() && !guessed_numbers.is_empty();
        if guessed {
            owned = guessed_numbers;
        }

        owned.sort_unstable();
        owned.dedup();

        // Huecos: lo que falta entre el primero y el último que tienes, y
        // además hasta el total si la serie lo declara.
        let missing = if owned.is_empty() {
            Vec::new()
        } else {
            let hasta = declared_total
                .unwrap_or(*owned.last().unwrap())
                .max(*owned.last().unwrap());
            (*owned.first().unwrap()..=hasta)
                .filter(|n| !owned.contains(n))
                .collect()
        };

        let best = |votes: HashMap<String, usize>| {
            votes
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(name, _)| name)
        };

        // Último recurso: el nombre de la carpeta suele ser la serie, pero
        // viene con adornos ("Doctor Strange Vol 1 (1974)"). Se limpia igual
        // que los nombres de fichero, y el volumen y el año que traiga se
        // aprovechan para la búsqueda.
        let folder = dir
            .file_name()
            .map(|n| crate::comicinfo::clean_series_name(&n.to_string_lossy()))
            .unwrap_or_default();

        SeriesInfo {
            series: best(series_votes)
                .or_else(|| best(guessed_series))
                .or(folder.name),
            publisher,
            volume: volume.or(folder.volume),
            year: year.or(folder.year),
            total: declared_total,
            owned,
            missing,
            tagged,
            untagged,
            guessed,
        }
    })
    .await
    .map_err(|e| format!("fallo al analizar la colección: {e}"))
}

/// Rutas de todos los cómics de una carpeta, incluidas sus subcarpetas.
/// Sirve para marcar una colección entera como leída de una vez.
#[tauri::command]
pub fn list_comics_deep(root: String, path: String) -> Result<Vec<String>, String> {
    let (_, dir) = resolve_within(&root, Some(path))?;
    Ok(walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && archive::is_comic(e.path()))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect())
}

/// Fija la portada de una carpeta copiando una imagen dentro como `cover.jpg`.
///
/// Se escribe con ese nombre a propósito, siguiendo la convención que ya lee
/// `manual_folder_cover`: así la portada es un fichero normal y corriente que
/// el usuario puede ver, cambiar o borrar desde el explorador, y que otras
/// aplicaciones de cómics también reconocen. Nada queda escondido en una base
/// de datos interna de la app.
#[tauri::command]
pub fn set_folder_cover(root: String, path: String, image: String) -> Result<(), String> {
    let (_, dir) = resolve_within(&root, Some(path))?;
    if !dir.is_dir() {
        return Err("la ruta no es una carpeta".into());
    }

    let raw = std::fs::read(&image).map_err(|e| format!("no se pudo leer la imagen: {e}"))?;

    // Se reconvierte a JPEG en vez de copiar el original: normaliza el formato
    // y evita meter en la biblioteca un fichero enorme o con un formato que
    // luego no se pueda decodificar.
    let img = image::load_from_memory(&raw)
        .map_err(|_| "el archivo no es una imagen que se pueda leer".to_string())?;

    let mut buf = std::io::Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    encoder
        .encode_image(&img.to_rgb8())
        .map_err(|e| format!("no se pudo convertir la imagen: {e}"))?;

    std::fs::write(dir.join("cover.jpg"), buf.into_inner())
        .map_err(|e| format!("no se pudo guardar la portada: {e}"))?;

    Ok(())
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
    use std::io::Write;

    /// PNG de 1x1 válido, lo mínimo para que cuente como página.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xF8,
        0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0xFB, 0x02, 0xFE, 0x4A, 0x2F, 0x1A, 0x93, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn write_cbz(path: &Path, pages: usize) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for i in 0..pages {
            zip.start_file(format!("page{i:03}.png"), opts).unwrap();
            zip.write_all(PNG).unwrap();
        }
        zip.finish().unwrap();
    }

    /// Biblioteca de prueba montada al vuelo, para que los tests no dependan
    /// de ficheros externos y puedan correr en CI.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let serie = dir.path().join("Serie A");
        std::fs::create_dir_all(&serie).unwrap();
        std::fs::create_dir_all(dir.path().join("vacia")).unwrap();

        write_cbz(&serie.join("bueno.cbz"), 3);

        // Es un zip válido pero sin ninguna imagen dentro.
        let f = std::fs::File::create(serie.join("sin_paginas.cbz")).unwrap();
        let mut zip = zip::ZipWriter::new(f);
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("leeme.txt", opts).unwrap();
        zip.write_all(b"nada").unwrap();
        zip.finish().unwrap();

        // Ni siquiera es un zip.
        std::fs::write(serie.join("corrupto.cbz"), b"no soy un zip").unwrap();

        dir
    }

    /// Un cómic ilegible debe seguir apareciendo en el listado. Antes se
    /// descartaba en silencio y el usuario veía un hueco sin explicación.
    #[test]
    fn broken_comics_are_still_listed() {
        let tmp = fixture();
        let root = tmp.path().to_string_lossy().to_string();
        let serie = tmp.path().join("Serie A").to_string_lossy().to_string();

        let listing = list_dir(root, Some(serie)).expect("list_dir debería funcionar");
        let mut names: Vec<&str> = listing.comics.iter().map(|c| c.name.as_str()).collect();
        names.sort_unstable();

        assert_eq!(names, vec!["bueno", "corrupto", "sin_paginas"]);
    }

    /// Las carpetas sin nada dentro no se listan, pero las que tienen cómics sí.
    #[test]
    fn empty_folders_are_hidden() {
        let tmp = fixture();
        let listing = list_dir(tmp.path().to_string_lossy().to_string(), None).unwrap();
        let names: Vec<&str> = listing.folders.iter().map(|f| f.name.as_str()).collect();

        assert!(names.contains(&"Serie A"));
        assert!(!names.contains(&"vacia"), "carpeta vacía no debería listarse");
    }

    /// No se puede salir de la raíz elegida.
    #[test]
    fn cannot_escape_root() {
        let tmp = fixture();
        let root = tmp.path().join("Serie A").to_string_lossy().to_string();
        let outside = tmp.path().to_string_lossy().to_string();

        assert!(
            list_dir(root, Some(outside)).is_err(),
            "debería rechazar rutas fuera de la biblioteca"
        );
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;

    /// No es un test de correccion: mide el coste que la cache evita.
    /// Ignorado por defecto; se lanza con `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cost_of_opening_every_comic() {
        let Ok(root) = std::env::var("READINGCOMICS_BIG_LIB") else {
            eprintln!("define READINGCOMICS_BIG_LIB");
            return;
        };
        let dir = PathBuf::from(&root).join("Serie");

        let t = Instant::now();
        let listing = list_dir(root.clone(), Some(dir.to_string_lossy().to_string()))
            .expect("list_dir");
        let listing_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let mut pages = 0usize;
        for c in &listing.comics {
            pages += archive::list_pages(Path::new(&c.path)).map(|p| p.len()).unwrap_or(0);
        }
        let open_ms = t.elapsed().as_secs_f64() * 1000.0;

        println!("comics                     : {}", listing.comics.len());
        println!("list_dir (solo ficheros)   : {listing_ms:.1} ms");
        println!("abrir y listar cada comic  : {open_ms:.1} ms  ({pages} paginas)");
        println!("--> lo que ahorra la cache : {open_ms:.1} ms por visita");
    }
}
