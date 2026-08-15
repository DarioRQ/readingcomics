//! Lectura de `ComicInfo.xml`, el fichero de metadatos que muchas releases
//! incluyen dentro del propio CBZ/CBR.
//!
//! Es el estándar que nació con ComicRack y que hoy usan Komga, Kavita,
//! ComicTagger y compañía. Leerlo sale gratis: no hace falta clave de API, ni
//! red, ni mandarle a nadie los nombres de tu biblioteca.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Subconjunto de ComicInfo.xml que la app usa. El estándar tiene bastantes
/// más campos; aquí solo están los que se muestran o sirven para saber si una
/// colección está completa.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename = "ComicInfo", default)]
pub struct ComicInfoXml {
    pub series: Option<String>,
    pub number: Option<String>,
    /// Total de números de la serie. Es el campo que permite decir "tienes 12
    /// de 24" sin consultar ninguna base de datos externa.
    pub count: Option<i32>,
    pub volume: Option<i32>,
    pub title: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<i32>,
    pub writer: Option<String>,
    pub summary: Option<String>,
    pub language_iso: Option<String>,
    /// `Yes`, `YesAndRightToLeft`, `No`… Sirve para saber si se lee al revés.
    pub manga: Option<String>,
}

/// Extrae el texto de los elementos de primer nivel.
///
/// ComicInfo.xml es plano —una lista de etiquetas con texto— así que no hace
/// falta deserializar el documento entero: recorrer los eventos es más tolerante
/// con los ficheros que traen campos raros, espacios de nombres o basura, que
/// en la práctica son muchos.
pub fn parse(xml: &[u8]) -> Option<ComicInfoXml> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let text = String::from_utf8_lossy(xml);
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);

    let mut info = ComicInfoXml::default();
    let mut current: Option<String> = None;
    let mut found_any = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                current = Some(name.to_lowercase());
            }
            Ok(Event::Text(e)) => {
                let Some(field) = current.as_deref() else {
                    continue;
                };
                let Ok(value) = e.decode() else { continue };
                let value = value.trim().to_string();
                if value.is_empty() {
                    continue;
                }
                found_any = true;

                let as_int = || value.parse::<i32>().ok();
                match field {
                    "series" => info.series = Some(value),
                    "number" => info.number = Some(value),
                    "count" => info.count = as_int(),
                    "volume" => info.volume = as_int(),
                    "title" => info.title = Some(value),
                    "publisher" => info.publisher = Some(value),
                    "year" => info.year = as_int(),
                    "writer" => info.writer = Some(value),
                    "summary" => info.summary = Some(value),
                    "languageiso" => info.language_iso = Some(value),
                    "manga" => info.manga = Some(value),
                    _ => {}
                }
            }
            Ok(Event::End(_)) => current = None,
            Ok(Event::Eof) => break,
            // Un XML corrupto no debe tumbar nada: se devuelve lo leído hasta
            // ese punto, que suele ser suficiente.
            Err(_) => break,
            _ => {}
        }
    }

    if found_any {
        Some(info)
    } else {
        None
    }
}

/// Lee y parsea el ComicInfo.xml de un cómic, si lo trae.
pub fn read(path: &Path) -> Option<ComicInfoXml> {
    parse(&crate::archive::read_metadata(path)?)
}

/// Número del cómic como entero, para poder ordenar y detectar huecos.
/// Acepta las formas habituales: `7`, `007`, `7a`, `7.5` (se queda con el 7).
pub fn issue_number(raw: &str) -> Option<u32> {
    let digits: String = raw
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <Series>Saga</Series>
  <Number>12</Number>
  <Count>54</Count>
  <Volume>1</Volume>
  <Publisher>Image Comics</Publisher>
  <Year>2013</Year>
  <LanguageISO>en</LanguageISO>
</ComicInfo>"#;

    #[test]
    fn parses_the_usual_fields() {
        let info = parse(SAMPLE.as_bytes()).expect("debería parsear");
        assert_eq!(info.series.as_deref(), Some("Saga"));
        assert_eq!(info.number.as_deref(), Some("12"));
        assert_eq!(info.count, Some(54));
        assert_eq!(info.publisher.as_deref(), Some("Image Comics"));
        assert_eq!(info.year, Some(2013));
    }

    #[test]
    fn broken_xml_does_not_panic() {
        assert!(parse(b"<ComicInfo><Series>Sin cerrar").is_some());
        assert!(parse(b"esto no es xml").is_none());
    }

    #[test]
    fn issue_numbers_of_all_shapes() {
        assert_eq!(issue_number("7"), Some(7));
        assert_eq!(issue_number("007"), Some(7));
        assert_eq!(issue_number("7a"), Some(7));
        assert_eq!(issue_number("7.5"), Some(7));
        assert_eq!(issue_number("anual"), None);
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use std::path::PathBuf;

    /// Comprueba de punta a punta que se lee el ComicInfo.xml de dentro de un
    /// CBZ real y que los huecos de la coleccion salen bien.
    #[test]
    fn detects_gaps_in_a_real_series() {
        let Ok(base) = std::env::var("READINGCOMICS_SERIES_LIB") else {
            eprintln!("define READINGCOMICS_SERIES_LIB");
            return;
        };
        let dir = PathBuf::from(base).join("Saga");

        let mut owned: Vec<u32> = Vec::new();
        let mut total = None;
        let mut untagged = 0;

        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        files.sort();

        for f in files {
            match read(&f) {
                Some(info) => {
                    if let Some(c) = info.count {
                        total = Some(c as u32);
                    }
                    if let Some(n) = info.number.as_deref().and_then(issue_number) {
                        owned.push(n);
                    }
                }
                None => untagged += 1,
            }
        }
        owned.sort_unstable();

        assert_eq!(owned, vec![1, 2, 3, 5, 8, 9], "numeros leidos del XML");
        assert_eq!(total, Some(12), "total declarado por la serie");
        assert_eq!(untagged, 1, "el cbz sin metadatos");

        let missing: Vec<u32> = (1..=12).filter(|n| !owned.contains(n)).collect();
        assert_eq!(missing, vec![4, 6, 7, 10, 11, 12], "huecos detectados");
    }
}

/* ---------- Deducción a partir del nombre de fichero ---------- */

/// Serie y número deducidos del nombre de un archivo.
///
/// Es un apaño, no una fuente fiable: se usa solo cuando el cómic no trae
/// `ComicInfo.xml`. Sin esto no habría forma de detectar una colección en las
/// bibliotecas sin etiquetar, que son la mayoría.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct NameGuess {
    pub series: Option<String>,
    pub number: Option<u32>,
}

fn looks_like_year(n: u32, digits: usize) -> bool {
    digits == 4 && (1900..=2100).contains(&n)
}

/// Intenta sacar serie y número de algo tipo `Saga 003 (2013)`.
///
/// Reglas, en este orden:
/// 1. Se quita lo que va entre paréntesis o corchetes: años, `[Digital]`, etc.
/// 2. Se ignoran los marcadores de volumen (`v01`, `Vol.2`), que no son el número.
/// 3. Se coge el último número suelto que no parezca un año.
/// 4. La serie es lo que quede por delante.
pub fn guess_from_filename(stem: &str) -> NameGuess {
    // 1. Fuera los grupos entre paréntesis y corchetes.
    let mut cleaned = String::with_capacity(stem.len());
    let mut depth = 0i32;
    for c in stem.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => cleaned.push(c),
            _ => {}
        }
    }

    let tokens: Vec<&str> = cleaned
        .split(|c: char| c.is_whitespace() || c == '_')
        .filter(|t| !t.is_empty())
        .collect();

    let mut number = None;
    let mut number_at = None;

    for (i, tok) in tokens.iter().enumerate() {
        let t = tok.trim_start_matches('#');

        // 2. `v01`, `vol.2`… es el volumen, no el número del ejemplar.
        let lower = t.to_lowercase();
        if lower.starts_with('v') && lower.trim_start_matches("vol").trim_start_matches('v')
            .trim_start_matches('.')
            .chars()
            .all(|c| c.is_ascii_digit())
        {
            continue;
        }

        let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() || digits.len() != t.trim_end_matches(|c: char| !c.is_ascii_digit()).len()
        {
            // Solo cuentan los tokens que son número (admitiendo sufijo tipo `3a`).
            if digits.is_empty() {
                continue;
            }
        }

        let Ok(n) = digits.parse::<u32>() else {
            continue;
        };
        // 3. Un año suelto no es el número del ejemplar.
        if looks_like_year(n, digits.len()) {
            continue;
        }
        number = Some(n);
        number_at = Some(i);
    }

    // 4. La serie es lo de delante, sin separadores colgando.
    let series = match number_at {
        Some(0) | None => None,
        Some(i) => {
            let name = tokens[..i]
                .join(" ")
                .trim()
                .trim_end_matches(['-', '–', '—', '.', ','])
                .trim()
                .to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }
    };

    NameGuess { series, number }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    fn guess(s: &str) -> (Option<String>, Option<u32>) {
        let g = guess_from_filename(s);
        (g.series, g.number)
    }

    #[test]
    fn parses_common_naming_patterns() {
        assert_eq!(guess("Saga 003"), (Some("Saga".into()), Some(3)));
        assert_eq!(guess("Saga #12"), (Some("Saga".into()), Some(12)));
        assert_eq!(guess("Saga 003 (2013)"), (Some("Saga".into()), Some(3)));
        assert_eq!(
            guess("Batman - 042 [Digital]"),
            (Some("Batman".into()), Some(42))
        );
        assert_eq!(
            guess("Asterix 12 - La Cizana"),
            (Some("Asterix".into()), Some(12))
        );
    }

    #[test]
    fn ignores_volume_markers() {
        assert_eq!(guess("Saga v01 003"), (Some("Saga v01".into()), Some(3)));
    }

    #[test]
    fn a_lone_year_is_not_an_issue_number() {
        assert_eq!(guess("Watchmen (1986)"), (None, None));
    }

    #[test]
    fn gives_up_cleanly_when_there_is_nothing_to_read() {
        assert_eq!(guess("portada"), (None, None));
        assert_eq!(guess("007"), (None, Some(7)));
    }
}
