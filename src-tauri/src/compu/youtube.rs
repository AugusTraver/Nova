//! "Poneme el primer video que sale cuando buscás Riquelme" (v0.7.1): la API
//! oficial de YouTube (`search.list`) con la key del usuario (en el
//! llavero). Desde junio de 2026 esa búsqueda tiene su propia cuota: 100 por
//! día gratis. Solo se pide el id del primer video; no se lee nada más.

use std::time::Duration;

use serde::Deserialize;

/// La cuenta del llavero donde va la key.
pub const CUENTA_LLAVERO: &str = "youtube";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub id: String,
    pub titulo: String,
    pub canal: String,
}

/// Por qué no se pudo elegir el video. En todos los casos Nova abre la lista
/// de resultados y lo dice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinVideo {
    SinKey,
    SinCuota,
    KeyMala,
    SinResultados,
    SinInternet(String),
}

impl SinVideo {
    pub fn explicacion(&self) -> &'static str {
        match self {
            SinVideo::SinKey => "no tengo la key de YouTube (se carga en Ajustes)",
            SinVideo::SinCuota => "se terminaron las búsquedas gratis de YouTube de hoy",
            SinVideo::KeyMala => "YouTube no aceptó la key",
            SinVideo::SinResultados => "no salió ningún video",
            SinVideo::SinInternet(_) => "no pude hablar con YouTube",
        }
    }
}

#[derive(Deserialize)]
struct Respuesta {
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: IdItem,
    snippet: Option<Snippet>,
}

#[derive(Deserialize)]
struct IdItem {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Deserialize)]
struct Snippet {
    #[serde(default)]
    title: String,
    #[serde(rename = "channelTitle", default)]
    channel_title: String,
}

/// YouTube devuelve los títulos con `&amp;`, `&#39;`…
fn sin_entidades(texto: &str) -> String {
    texto.replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">")
}

/// Lee la respuesta de la API (separado para testearlo sin red).
fn leer(estado: u16, cuerpo: &str) -> Result<Video, SinVideo> {
    if estado == 403 && (cuerpo.contains("quotaExceeded") || cuerpo.contains("rateLimitExceeded")) {
        return Err(SinVideo::SinCuota);
    }
    if estado == 400 || estado == 403 {
        return Err(SinVideo::KeyMala);
    }
    if estado != 200 {
        return Err(SinVideo::SinInternet(format!("YouTube contestó {estado}")));
    }
    let respuesta: Respuesta = serde_json::from_str(cuerpo).map_err(|e| SinVideo::SinInternet(e.to_string()))?;
    respuesta
        .items
        .into_iter()
        .find_map(|item| {
            let id = item.id.video_id?;
            let snippet = item.snippet;
            Some(Video {
                id,
                titulo: snippet.as_ref().map(|s| sin_entidades(&s.title)).unwrap_or_default(),
                canal: snippet.map(|s| sin_entidades(&s.channel_title)).unwrap_or_default(),
            })
        })
        .ok_or(SinVideo::SinResultados)
}

/// El primer video de la búsqueda.
pub async fn primer_video(consulta: &str, key: Option<&str>) -> Result<Video, SinVideo> {
    let key = key.filter(|k| !k.is_empty()).ok_or(SinVideo::SinKey)?;
    let cliente = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| SinVideo::SinInternet(e.to_string()))?;
    let direccion = url::Url::parse_with_params(
        "https://www.googleapis.com/youtube/v3/search",
        &[
            ("part", "snippet"),
            ("type", "video"),
            ("maxResults", "1"),
            ("regionCode", "AR"),
            ("relevanceLanguage", "es"),
            ("q", consulta),
            ("key", key),
        ],
    )
    .map_err(|e| SinVideo::SinInternet(e.to_string()))?;
    // `without_url` en los errores: la dirección lleva la key y no tiene que
    // terminar en la consola.
    let respuesta = cliente
        .get(direccion)
        .send()
        .await
        .map_err(|e| SinVideo::SinInternet(e.without_url().to_string()))?;
    let estado = respuesta.status().as_u16();
    let cuerpo = respuesta.text().await.map_err(|e| SinVideo::SinInternet(e.without_url().to_string()))?;
    leer(estado, &cuerpo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_el_primer_video() {
        let cuerpo = r#"{"items":[{"id":{"kind":"youtube#video","videoId":"abcdefghijk"},
            "snippet":{"title":"Riquelme: los mejores goles &amp; caños","channelTitle":"Boca"}}]}"#;
        let v = leer(200, cuerpo).unwrap();
        assert_eq!(v, Video { id: "abcdefghijk".into(), titulo: "Riquelme: los mejores goles & caños".into(), canal: "Boca".into() });
        assert_eq!(leer(200, r#"{"items":[]}"#), Err(SinVideo::SinResultados));
        assert_eq!(leer(403, r#"{"error":{"errors":[{"reason":"quotaExceeded"}]}}"#), Err(SinVideo::SinCuota));
        assert_eq!(leer(400, r#"{"error":{"errors":[{"reason":"keyInvalid"}]}}"#), Err(SinVideo::KeyMala));
    }

    #[tokio::test]
    async fn sin_key_no_sale_a_internet() {
        assert_eq!(primer_video("riquelme", None).await, Err(SinVideo::SinKey));
    }

    /// Con la key de verdad (gasta 1 de las 100 búsquedas del día):
    /// `cargo test busca_en_youtube -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn busca_en_youtube() {
        let key = crate::llavero::leer(CUENTA_LLAVERO).unwrap();
        let v = primer_video("Riquelme", key.as_deref()).await.unwrap();
        println!("{v:?}");
        assert_eq!(v.id.len(), 11);
    }
}
