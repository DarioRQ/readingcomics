//! Conexión opcional con la base de datos de cómics Metron.
//!
//! Metron (<https://metron.cloud>) es una base de datos comunitaria y abierta,
//! nacida como alternativa a Comic Vine. Se usa para completar los datos que no
//! trae el `ComicInfo.xml` incrustado, sobre todo cuántos números tiene una
//! serie.
//!
//! Es **opcional y bajo demanda**: sin token configurado la app no hace ni una
//! petición, y aun con token solo consulta cuando el usuario lo pide
//! explícitamente. Mandar los títulos de tu biblioteca a un servidor ajeno es
//! decisión del usuario, no algo que deba pasar de fondo.
//!
//! Límites que publica el proyecto: 20 peticiones/minuto y 5.000/día. El
//! servidor los informa en cabeceras `X-RateLimit-*`, que aquí se leen para
//! poder avisar antes de chocar contra el límite.

use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_BASE: &str = "https://metron.cloud/api";
const TIMEOUT: Duration = Duration::from_secs(20);

/// Servicio y usuario con los que se guarda el token en el llavero del sistema
/// (Credential Manager en Windows, Secret Service en Linux, Keychain en macOS).
/// Nunca se escribe en config.json, que va en texto plano.
const KEYRING_SERVICE: &str = "readingcomics";
const KEYRING_USER: &str = "metron-api-token";

#[derive(Serialize, Clone, Default)]
pub struct MetronStatus {
    pub connected: bool,
    /// Peticiones que quedan en el minuto y en el día, si se conocen.
    pub burst_remaining: Option<u32>,
    pub sustained_remaining: Option<u32>,
}

/// Serie tal y como la devuelve Metron, recortada a lo que se usa.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetronSeries {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub year_began: Option<i32>,
    /// Número de ejemplares que Metron tiene registrados de la serie.
    #[serde(default)]
    pub issue_count: Option<u32>,
    #[serde(default)]
    pub publisher: Option<String>,
    /// Estado de la serie según Metron ("Completed", "Ongoing"…). Solo si una
    /// serie está terminada tiene sentido dar su total por definitivo: en una
    /// serie en emisión el recuento de hoy no es el final.
    #[serde(default)]
    pub status: Option<String>,
}

/// Una de las series que devuelve la búsqueda. Es lo que se le enseña al
/// usuario cuando hay varias candidatas: una misma cabecera suele tener varios
/// volúmenes y solo el año o el número de volumen los distinguen.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetronCandidate {
    pub id: i64,
    pub name: String,
    pub year_began: Option<i32>,
    pub volume: Option<u32>,
    pub issue_count: Option<u32>,
}

#[derive(Deserialize)]
struct SeriesListResponse {
    #[serde(default)]
    results: Vec<SeriesListItem>,
}

/// La lista trae ya `issue_count`, `volume` y `year_began`; el detalle solo hace
/// falta para el estado de la serie y la editorial. El nombre viene en `series`
/// (y en `name` en las respuestas antiguas), así que se aceptan los dos.
#[derive(Deserialize)]
struct SeriesListItem {
    id: i64,
    #[serde(default)]
    series: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    year_began: Option<i32>,
    #[serde(default)]
    volume: Option<u32>,
    #[serde(default)]
    issue_count: Option<u32>,
}

impl SeriesListItem {
    fn into_candidate(self) -> MetronCandidate {
        MetronCandidate {
            name: self
                .series
                .or(self.name)
                .unwrap_or_else(|| format!("serie {}", self.id)),
            id: self.id,
            year_began: self.year_began,
            volume: self.volume,
            issue_count: self.issue_count,
        }
    }
}

#[derive(Deserialize)]
struct SeriesDetail {
    id: i64,
    name: String,
    #[serde(default)]
    year_began: Option<i32>,
    #[serde(default)]
    issue_count: Option<u32>,
    #[serde(default)]
    publisher: Option<PublisherRef>,
    #[serde(default)]
    status: Option<StatusField>,
}

/// Metron puede devolver el estado como texto suelto o como objeto con nombre;
/// se aceptan ambas formas para no romper si cambia.
#[derive(Deserialize)]
#[serde(untagged)]
enum StatusField {
    Text(String),
    Object { name: Option<String> },
}

impl StatusField {
    fn into_name(self) -> Option<String> {
        match self {
            StatusField::Text(s) => Some(s),
            StatusField::Object { name } => name,
        }
    }
}

#[derive(Deserialize)]
struct PublisherRef {
    #[serde(default)]
    name: Option<String>,
}

/* ---------- Token en el llavero del sistema ---------- */

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("no se pudo abrir el llavero del sistema: {e}"))
}

fn stored_token() -> Option<String> {
    entry().ok()?.get_password().ok()
}

/* ---------- Cliente ---------- */

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        // Identificarse es de buena educación con una API comunitaria y ayuda
        // al proyecto a saber quién la usa.
        .user_agent(concat!("readingcomics/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("no se pudo crear el cliente HTTP: {e}"))
}

fn header_u32(res: &reqwest::Response, name: &str) -> Option<u32> {
    res.headers().get(name)?.to_str().ok()?.parse().ok()
}

/// Traduce el estado HTTP a un mensaje que el usuario pueda entender.
fn describe_status(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => "el token de Metron no es válido o ha caducado".into(),
        429 => "has alcanzado el límite de peticiones de Metron; espera un poco".into(),
        s if s >= 500 => "Metron está teniendo problemas; inténtalo más tarde".into(),
        s => format!("Metron respondió con el código {s}"),
    }
}

async fn get_json<T: serde::de::DeserializeOwned>(
    token: &str,
    url: &str,
) -> Result<(T, MetronStatus), String> {
    let res = client()?
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Metron no respondió a tiempo".to_string()
            } else {
                format!("no se pudo conectar con Metron: {e}")
            }
        })?;

    let status = MetronStatus {
        connected: true,
        burst_remaining: header_u32(&res, "x-ratelimit-burst-remaining"),
        sustained_remaining: header_u32(&res, "x-ratelimit-sustained-remaining"),
    };

    if !res.status().is_success() {
        return Err(describe_status(res.status()));
    }

    let body = res
        .json::<T>()
        .await
        .map_err(|e| format!("respuesta inesperada de Metron: {e}"))?;

    Ok((body, status))
}

/* ---------- Comandos ---------- */

/// Guarda el token y lo valida contra la API antes de darlo por bueno.
#[tauri::command]
pub async fn metron_connect(token: String) -> Result<MetronStatus, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("el token está vacío".into());
    }

    // Una consulta mínima que sirve de comprobación de credenciales.
    let (_, status) =
        get_json::<serde_json::Value>(&token, &format!("{API_BASE}/series/?page=1")).await?;

    entry()?
        .set_password(&token)
        .map_err(|e| format!("no se pudo guardar el token en el llavero: {e}"))?;

    Ok(status)
}

#[tauri::command]
pub fn metron_status() -> MetronStatus {
    MetronStatus {
        connected: stored_token().is_some(),
        ..Default::default()
    }
}

#[tauri::command]
pub fn metron_disconnect() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        // Que no hubiera nada guardado no es un error para quien desconecta.
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("no se pudo borrar el token: {e}")),
    }
}

/* ---------- Búsqueda ---------- */

/// Quita acentos y todo lo que no sea letra o dígito, para poder comparar
/// nombres que solo se diferencian en la puntuación: `Spider-Man`, `Spiderman`
/// y `Spider Man` tienen que contar como el mismo.
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(|c| c.to_lowercase())
        .filter_map(|c| {
            let c = match c {
                'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ñ' => 'n',
                'ç' => 'c',
                other => other,
            };
            c.is_alphanumeric().then_some(c)
        })
        .collect()
}

/// Puntúa cuánto se parece una candidata a lo que se buscaba. Lo que más pesa
/// es que el nombre coincida entero; el año y el volumen deciden entre los
/// varios volúmenes de una misma cabecera, que es justo donde se fallaba.
fn score(c: &MetronCandidate, wanted: &str, year: Option<i32>, volume: Option<u32>) -> i32 {
    let wanted = fold(wanted);
    // El nombre de la lista suele venir como "Doctor Strange (1974)".
    let bare = c.name.split('(').next().unwrap_or(&c.name);
    let name = fold(bare);

    let mut points = if name == wanted {
        100
    } else if name.starts_with(&wanted) || wanted.starts_with(&name) {
        40
    } else if name.contains(&wanted) || wanted.contains(&name) {
        10
    } else {
        0
    };

    if let (Some(a), Some(b)) = (year, c.year_began) {
        points += if a == b { 30 } else { -10 };
    }
    if let (Some(a), Some(b)) = (volume, c.volume) {
        points += if a == b { 20 } else { -5 };
    }
    // Entre dos iguales, mejor la que trae el recuento hecho.
    if c.issue_count.is_some() {
        points += 2;
    }
    points
}

async fn search_once(token: &str, query: &str) -> Result<Vec<MetronCandidate>, String> {
    let (list, _) =
        get_json::<SeriesListResponse>(token, &format!("{API_BASE}/series/?{query}")).await?;
    Ok(list.results.into_iter().map(|r| r.into_candidate()).collect())
}

/// Busca series candidatas, de la consulta más precisa a la más amplia.
///
/// Se para en cuanto una devuelve algo, así que lo normal es gastar **una sola
/// petición**. El orden importa: `q=` busca también en los nombres alternativos
/// —donde están las ediciones en otros idiomas—, y los filtros de año y volumen
/// se sueltan antes que el nombre porque son los datos más propensos a no
/// coincidir con lo que tiene Metron.
#[tauri::command]
pub async fn metron_search_series(
    name: String,
    year: Option<i32>,
    volume: Option<u32>,
) -> Result<Vec<MetronCandidate>, String> {
    let Some(token) = stored_token() else {
        return Err("no hay ninguna cuenta de Metron conectada".into());
    };

    // El nombre puede llegar tal cual está la carpeta ("Doctor Strange Vol 1"),
    // y así Metron no encuentra nada: su filtro es por subcadena del nombre, y
    // el volumen y el año son campos aparte.
    let clean = crate::comicinfo::clean_series_name(&name);
    let Some(base) = clean.name.or_else(|| {
        let t = name.trim();
        (!t.is_empty()).then(|| t.to_string())
    }) else {
        return Ok(Vec::new());
    };
    let year = year.or(clean.year);
    let volume = volume.or(clean.volume);

    let encoded = urlencoding_encode(&base);
    let mut attempts: Vec<String> = Vec::new();

    if year.is_some() || volume.is_some() {
        let mut q = format!("q={encoded}");
        if let Some(y) = year {
            q.push_str(&format!("&year_began={y}"));
        }
        if let Some(v) = volume {
            q.push_str(&format!("&volume={v}"));
        }
        attempts.push(q);
    }
    // Sin filtros: el nombre limpio ya suele bastar.
    attempts.push(format!("q={encoded}"));
    // Último intento: soltar el subtítulo, que casi nunca está en Metron.
    if let Some((head, _)) = base.split_once(" - ").or_else(|| base.split_once(": ")) {
        let head = head.trim();
        if head.len() >= 3 && head != base {
            attempts.push(format!("q={}", urlencoding_encode(head)));
        }
    }

    for query in attempts {
        let mut found = search_once(&token, &query).await?;
        if found.is_empty() {
            continue;
        }
        found.sort_by_key(|c| std::cmp::Reverse(score(c, &base, year, volume)));
        return Ok(found);
    }

    Ok(Vec::new())
}

/// Datos completos de una serie ya identificada por su id.
///
/// Va aparte porque la lista no trae el estado ni la editorial, y el estado es
/// lo que decide si una colección puede darse por completa para siempre.
#[tauri::command]
pub async fn metron_series_detail(id: i64) -> Result<MetronSeries, String> {
    let Some(token) = stored_token() else {
        return Err("no hay ninguna cuenta de Metron conectada".into());
    };

    let (detail, _) = get_json::<SeriesDetail>(&token, &format!("{API_BASE}/series/{id}/")).await?;

    Ok(MetronSeries {
        id: detail.id,
        name: detail.name,
        year_began: detail.year_began,
        issue_count: detail.issue_count,
        publisher: detail.publisher.and_then(|p| p.name),
        status: detail.status.and_then(|s| s.into_name()),
    })
}

/// Busca una serie y devuelve directamente la mejor coincidencia, ya con su
/// detalle. Es el camino de un solo clic; si el usuario quiere otra, la
/// interfaz pide las candidatas con `metron_search_series`.
#[tauri::command]
pub async fn metron_find_series(
    name: String,
    year: Option<i32>,
    volume: Option<u32>,
) -> Result<Option<MetronSeries>, String> {
    let clean = crate::comicinfo::clean_series_name(&name);
    let wanted = clean.name.clone().unwrap_or_else(|| name.trim().to_string());
    let year = year.or(clean.year);
    let volume = volume.or(clean.volume);

    let found = metron_search_series(name, year, volume).await?;
    // Que la lista traiga algo no significa que sea lo que se buscaba: si el
    // servidor ignorase un filtro devolvería series sin relación, y darlas por
    // buenas sería peor que no encontrar nada. Sin parecido en el nombre, se
    // deja que elija el usuario.
    let best = match found.first() {
        Some(c) if score(c, &wanted, year, volume) >= 10 => c,
        _ => return Ok(None),
    };

    let mut detail = metron_series_detail(best.id).await?;
    // El recuento de la lista vale igual, y así no se pierde si el detalle no
    // lo trae por lo que sea.
    detail.issue_count = detail.issue_count.or(best.issue_count);
    detail.year_began = detail.year_began.or(best.year_began);
    Ok(Some(detail))
}

/// Codifica el texto para meterlo en la query. Solo se dejan pasar los
/// caracteres seguros; el resto va en porcentaje, para que un nombre con `&`
/// o espacios no rompa ni altere la URL.
fn urlencoding_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_safely() {
        assert_eq!(urlencoding_encode("Saga"), "Saga");
        assert_eq!(urlencoding_encode("V de Vendetta"), "V+de+Vendetta");
        // Lo importante: nada de lo que escriba el usuario puede añadir
        // parámetros ni salirse de la query.
        assert_eq!(urlencoding_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencoding_encode("../../etc"), "..%2F..%2Fetc");
    }

    fn candidate(id: i64, name: &str, year: Option<i32>, volume: Option<u32>) -> MetronCandidate {
        MetronCandidate {
            id,
            name: name.into(),
            year_began: year,
            volume,
            issue_count: Some(10),
        }
    }

    #[test]
    fn punctuation_and_accents_do_not_break_the_match() {
        assert_eq!(fold("Spider-Man"), fold("Spider Man"));
        assert_eq!(fold("Astérix"), fold("asterix"));
        assert_eq!(fold("Patrulla-X"), "patrullax");
    }

    #[test]
    fn the_right_volume_wins() {
        // El caso que fallaba: varios volúmenes de la misma cabecera, y hay que
        // quedarse con el que pide la carpeta, no con el primero que llegue.
        let v1 = candidate(1, "Doctor Strange (1974)", Some(1974), Some(1));
        let v3 = candidate(3, "Doctor Strange (2015)", Some(2015), Some(4));
        let otra = candidate(9, "Doctor Strange and the Sorcerers Supreme (2016)", Some(2016), Some(1));

        let by_year = |c: &MetronCandidate| score(c, "Doctor Strange", Some(1974), None);
        assert!(by_year(&v1) > by_year(&v3));

        let by_volume = |c: &MetronCandidate| score(c, "Doctor Strange", None, Some(4));
        assert!(by_volume(&v3) > by_volume(&v1));

        // Un nombre más largo que solo contiene al buscado nunca gana al exacto.
        let plain = |c: &MetronCandidate| score(c, "Doctor Strange", None, None);
        assert!(plain(&v1) > plain(&otra));
    }

    #[test]
    fn http_errors_are_explained_in_plain_language() {
        let msg = describe_status(reqwest::StatusCode::UNAUTHORIZED);
        assert!(msg.contains("token"), "{msg}");
        let msg = describe_status(reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert!(msg.contains("límite"), "{msg}");
    }
}
