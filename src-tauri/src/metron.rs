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
}

#[derive(Deserialize)]
struct SeriesListResponse {
    #[serde(default)]
    results: Vec<SeriesListItem>,
}

/// La lista devuelve menos campos que el detalle; `issue_count` sale del
/// detalle, así que aquí solo se recoge lo justo para identificar la serie.
#[derive(Deserialize)]
struct SeriesListItem {
    id: i64,
    #[serde(default)]
    series: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    year_began: Option<i32>,
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

/// Busca una serie por nombre y devuelve la mejor coincidencia con su número
/// total de ejemplares.
///
/// Son dos peticiones: la búsqueda no trae `issue_count`, hay que pedir el
/// detalle. Se filtra en el servidor con `name=`, como pide su guía de buenas
/// prácticas, en vez de traerse listados enteros.
#[tauri::command]
pub async fn metron_find_series(name: String) -> Result<Option<MetronSeries>, String> {
    let Some(token) = stored_token() else {
        return Err("no hay ninguna cuenta de Metron conectada".into());
    };

    let query = urlencoding_encode(name.trim());
    if query.is_empty() {
        return Ok(None);
    }

    let (list, _) =
        get_json::<SeriesListResponse>(&token, &format!("{API_BASE}/series/?name={query}")).await?;

    let Some(first) = list.results.first() else {
        return Ok(None);
    };

    let (detail, _) =
        get_json::<SeriesDetail>(&token, &format!("{API_BASE}/series/{}/", first.id)).await?;

    Ok(Some(MetronSeries {
        id: detail.id,
        name: detail.name,
        year_began: detail.year_began.or(first.year_began),
        issue_count: detail.issue_count,
        publisher: detail.publisher.and_then(|p| p.name),
    }))
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

    #[test]
    fn http_errors_are_explained_in_plain_language() {
        let msg = describe_status(reqwest::StatusCode::UNAUTHORIZED);
        assert!(msg.contains("token"), "{msg}");
        let msg = describe_status(reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert!(msg.contains("límite"), "{msg}");
    }
}
