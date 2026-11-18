//! Noticias (SPEC 4.5): titulares de feeds RSS, filtrados y ordenados según
//! los intereses del usuario.
//!
//! - `rss`: baja y lee los feeds (la única parte que toca la red).
//! - `crate::dominio::noticias`: elige los candidatos, arma el pedido a la IA,
//!   lee su respuesta y ordena sin IA cuando no hay (todo puro, con tests).

pub mod rss;

use serde::{Deserialize, Serialize};

/// Claves de la caché: los titulares de todos los feeds y lo elegido.
pub const CLAVE_TITULARES: &str = "noticias:titulares";
pub const CLAVE_ELEGIDAS: &str = "noticias:elegidas";

/// Lo que recibe el widget: lo elegido, con la fecha y el error de los feeds
/// (si los feeds fallan, se ve lo último con el aviso).
pub fn vista(conexion: &rusqlite::Connection) -> crate::error::Resultado<crate::repos::cache::Cacheado<Vec<Noticia>>> {
    use crate::repos::cache;
    let titulares: cache::Cacheado<Vec<Titular>> = cache::leer(conexion, CLAVE_TITULARES)?;
    let elegidas: cache::Cacheado<Elegidas> = cache::leer(conexion, CLAVE_ELEGIDAS)?;
    Ok(cache::Cacheado {
        datos: elegidas.datos.map(|e| e.noticias),
        obtenido_en: titulares.obtenido_en.or(elegidas.obtenido_en),
        error: titulares.error,
    })
}

/// Un feed configurado en Ajustes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    /// "Olé", "The Verge"…: es lo que se muestra al lado del titular.
    pub nombre: String,
    pub url: String,
}

/// Lo que se guarda en preferencias (clave `noticias`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesNoticias {
    /// Texto libre: "Boca, Selección Argentina, IA, desarrollo web".
    pub intereses: String,
    pub feeds: Vec<Feed>,
}

pub const INTERESES_POR_DEFECTO: &str = "Boca, Selección Argentina, fútbol argentino, IA, desarrollo web";

fn feed(nombre: &str, url: &str) -> Feed {
    Feed { nombre: nombre.into(), url: url.into() }
}

impl Default for AjustesNoticias {
    fn default() -> Self {
        AjustesNoticias {
            intereses: INTERESES_POR_DEFECTO.into(),
            feeds: vec![
                feed("Olé", "https://www.ole.com.ar/rss/boca-juniors/"),
                feed("Olé", "https://www.ole.com.ar/rss/ultimas-noticias/"),
                feed("La Nación", "https://www.lanacion.com.ar/arc/outboundfeeds/rss/category/deportes/futbol/"),
                feed("Infobae", "https://www.infobae.com/arc/outboundfeeds/rss/category/deportes/"),
                feed("Clarín", "https://www.clarin.com/rss/deportes/"),
                feed("Xataka", "https://www.xataka.com/feedburner.xml"),
                feed("Hipertextual", "https://hipertextual.com/feed"),
                feed("The Verge", "https://www.theverge.com/rss/ai-artificial-intelligence/index.xml"),
                feed("Hacker News", "https://news.ycombinator.com/rss"),
                feed("Smashing", "https://www.smashingmagazine.com/feed/"),
                feed("web.dev", "https://web.dev/feed.xml"),
                feed("CSS-Tricks", "https://css-tricks.com/feed/"),
            ],
        }
    }
}

pub const MAX_FEEDS: usize = 30;
pub const MAX_INTERESES: usize = 300;

impl AjustesNoticias {
    /// Limpia y valida lo que llega de Ajustes.
    pub fn validar(self) -> Result<Self, String> {
        let intereses = self.intereses.trim().to_string();
        if intereses.chars().count() > MAX_INTERESES {
            return Err(format!("Los intereses pueden tener hasta {MAX_INTERESES} letras."));
        }
        if self.feeds.len() > MAX_FEEDS {
            return Err(format!("Podés tener hasta {MAX_FEEDS} feeds."));
        }
        let mut feeds: Vec<Feed> = Vec::new();
        for f in self.feeds {
            let url = f.url.trim().to_string();
            let nombre = f.nombre.trim().to_string();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(format!("«{url}» no es una dirección web (tiene que empezar con https://)."));
            }
            if nombre.is_empty() {
                return Err("Cada feed necesita un nombre.".into());
            }
            if feeds.iter().any(|g| g.url == url) {
                continue;
            }
            feeds.push(Feed { nombre, url });
        }
        Ok(AjustesNoticias { intereses, feeds })
    }
}

/// Un titular tal cual sale del feed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Titular {
    /// Sale de la dirección de la nota: el mismo titular en dos vueltas tiene
    /// el mismo id (así se sabe si hay novedades).
    pub id: String,
    pub titulo: String,
    /// La bajada, sin HTML y recortada.
    pub bajada: Option<String>,
    pub fuente: String,
    pub url: String,
    /// ISO 8601.
    pub publicado: Option<String>,
}

/// Una noticia elegida para el widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Noticia {
    pub id: String,
    pub titulo: String,
    /// Una línea. Vacío si no hubo IA para escribirlo.
    pub resumen: String,
    pub fuente: String,
    pub url: String,
    pub publicado: Option<String>,
}

/// Lo que queda en la caché después de elegir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Elegidas {
    pub noticias: Vec<Noticia>,
    /// Las eligió la IA (si no, el orden por palabras clave).
    pub con_ia: bool,
    /// Los ids de los candidatos que se miraron: si no cambian, no hace falta
    /// volver a preguntarle a la IA.
    pub candidatos: Vec<String>,
}

/// Un id corto y estable para una dirección (FNV-1a de 64 bits). No hace
/// falta que sea criptográfico: solo que la misma nota dé el mismo id.
pub fn id_de(url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in url.trim().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_estables() {
        assert_eq!(id_de("https://a.com/x"), id_de(" https://a.com/x "));
        assert_ne!(id_de("https://a.com/x"), id_de("https://a.com/y"));
    }

    #[test]
    fn valida_ajustes() {
        let bien = AjustesNoticias {
            intereses: "  IA  ".into(),
            feeds: vec![feed(" Olé ", " https://ole.com/rss "), feed("Otra vez", "https://ole.com/rss")],
        };
        let v = bien.validar().unwrap();
        assert_eq!(v.intereses, "IA");
        assert_eq!(v.feeds, vec![feed("Olé", "https://ole.com/rss")]);
        let mal = AjustesNoticias { feeds: vec![feed("X", "ole.com")], ..Default::default() };
        assert!(mal.validar().is_err());
        let sin_nombre = AjustesNoticias { feeds: vec![feed(" ", "https://x.com")], ..Default::default() };
        assert!(sin_nombre.validar().is_err());
    }
}
