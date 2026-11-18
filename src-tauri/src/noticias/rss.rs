//! Bajar y leer feeds RSS o Atom. Leer es una función pura (`leer_feed`,
//! testeada con feeds reales en `ejemplos/`); bajar es lo único con red.

use std::time::Duration;

use chrono::SecondsFormat;

use super::{Feed, Titular, id_de};
use crate::error::{Error, Resultado};

/// Hasta cuántas letras de bajada se guardan (para la IA alcanza con poco).
const LARGO_BAJADA: usize = 160;
/// Hasta cuántos titulares por feed.
const POR_FEED: usize = 25;

/// Lee un feed (RSS 1/2, Atom o JSON Feed: `feed-rs` entiende todos).
pub fn leer_feed(bytes: &[u8], fuente: &str) -> Result<Vec<Titular>, String> {
    let feed = feed_rs::parser::parse(bytes).map_err(|_| format!("El feed de {fuente} no se puede leer."))?;
    let titulares = feed
        .entries
        .into_iter()
        .filter_map(|entrada| {
            let titulo = limpiar(&entrada.title?.content);
            // El primer link "alternate" (o el primero que haya) es la nota.
            let url = entrada
                .links
                .iter()
                .find(|l| l.rel.as_deref().is_none_or(|r| r == "alternate"))
                .or(entrada.links.first())?
                .href
                .trim()
                .to_string();
            if titulo.is_empty() || !url.starts_with("http") {
                return None;
            }
            let bajada = entrada
                .summary
                .map(|s| recortar(&limpiar(&s.content), LARGO_BAJADA))
                .filter(|b| !b.is_empty() && *b != titulo);
            let publicado = entrada
                .published
                .or(entrada.updated)
                .map(|f| f.to_rfc3339_opts(SecondsFormat::Secs, true));
            Some(Titular { id: id_de(&url), titulo, bajada, fuente: fuente.to_string(), url, publicado })
        })
        .take(POR_FEED)
        .collect();
    Ok(titulares)
}

/// Saca etiquetas HTML y las entidades más comunes, y junta los espacios.
pub fn limpiar(texto: &str) -> String {
    let mut sin_etiquetas = String::with_capacity(texto.len());
    let mut dentro = false;
    for c in texto.chars() {
        match c {
            '<' => dentro = true,
            '>' if dentro => {
                dentro = false;
                sin_etiquetas.push(' ');
            }
            _ if !dentro => sin_etiquetas.push(c),
            _ => {}
        }
    }
    let decodificado = sin_etiquetas
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&#8217;", "’")
        .replace("&#8216;", "‘")
        .replace("&#8220;", "“")
        .replace("&#8221;", "”")
        .replace("&#8230;", "…")
        .replace("&hellip;", "…")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    decodificado.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Corta en `largo` letras (no bytes: no parte una tilde) y agrega "…".
pub fn recortar(texto: &str, largo: usize) -> String {
    if texto.chars().count() <= largo {
        return texto.to_string();
    }
    let corte: String = texto.chars().take(largo).collect();
    // Mejor cortar en el último espacio que en mitad de una palabra.
    let corte = corte.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&corte);
    format!("{}…", corte.trim_end_matches([',', '.', ';', ':']))
}

pub struct ClienteNoticias {
    http: reqwest::Client,
}

impl ClienteNoticias {
    pub fn nuevo() -> Resultado<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .connect_timeout(Duration::from_secs(8))
            // Algunos sitios rechazan pedidos sin un navegador conocido.
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Nova/0.4")
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteNoticias { http })
    }

    pub async fn bajar(&self, feed: &Feed) -> Resultado<Vec<Titular>> {
        let sin_conexion = |_| Error::Sistema(format!("No me pude conectar con {}.", feed.nombre));
        let respuesta = self.http.get(&feed.url).send().await.map_err(sin_conexion)?;
        if !respuesta.status().is_success() {
            return Err(Error::Sistema(format!("{} respondió con un error ({}).", feed.nombre, respuesta.status().as_u16())));
        }
        let bytes = respuesta.bytes().await.map_err(sin_conexion)?;
        leer_feed(&bytes, &feed.nombre).map_err(Error::Sistema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_rss_de_ole() {
        let t = leer_feed(include_bytes!("ejemplos/ole_boca.xml"), "Olé").unwrap();
        assert!(t.len() > 3);
        let primero = &t[0];
        assert!(primero.titulo.starts_with("Battaglia"));
        assert!(!primero.titulo.ends_with(' '));
        assert!(primero.url.starts_with("https://www.ole.com.ar/boca-juniors/"));
        assert_eq!(primero.fuente, "Olé");
        assert_eq!(primero.publicado.as_deref(), Some("2026-11-18T21:18:57Z"));
        let bajada = primero.bajada.as_deref().unwrap();
        assert!(!bajada.contains("<p>"));
    }

    #[test]
    fn lee_atom_y_hn() {
        let verge = leer_feed(include_bytes!("ejemplos/verge_ai.xml"), "The Verge").unwrap();
        assert!(!verge.is_empty());
        assert!(verge.iter().all(|t| t.url.starts_with("https://www.theverge.com/")));
        let hn = leer_feed(include_bytes!("ejemplos/hn.xml"), "Hacker News").unwrap();
        assert!(hn.len() >= 20);
    }

    #[test]
    fn un_feed_roto_es_un_error() {
        assert!(leer_feed(b"<html>no</html>", "X").is_err());
    }

    #[test]
    fn limpia_y_recorta() {
        assert_eq!(limpiar("<p>Hola &amp; chau</p>\n  <b>x</b>"), "Hola & chau x");
        assert_eq!(recortar("uno dos tres", 100), "uno dos tres");
        assert_eq!(recortar("canción dos tres cuatro", 13), "canción dos…");
    }
}
