//! Clima (SPEC 4.7, opcional): temperatura y cielo de Open-Meteo, gratis y
//! sin API key. Leer las respuestas es puro (con ejemplos reales); bajar es
//! lo único con red.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Resultado};

/// La clave de la caché.
pub const CLAVE: &str = "clima:actual";

/// Lo que recibe el widget: si se muestra y el dato.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VistaClima {
    pub mostrar: bool,
    pub clima: crate::repos::cache::Cacheado<Clima>,
}

/// Lo que se guarda en preferencias (clave `clima`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesClima {
    pub mostrar: bool,
    pub ciudad: String,
    pub latitud: f64,
    pub longitud: f64,
}

impl Default for AjustesClima {
    fn default() -> Self {
        AjustesClima { mostrar: true, ciudad: "Buenos Aires".into(), latitud: -34.61, longitud: -58.38 }
    }
}

impl AjustesClima {
    pub fn validar(self) -> Result<Self, String> {
        let ciudad = self.ciudad.trim().to_string();
        if ciudad.is_empty() {
            return Err("Elegí una ciudad.".into());
        }
        if !(-90.0..=90.0).contains(&self.latitud) || !(-180.0..=180.0).contains(&self.longitud) {
            return Err("Esa ubicación no existe.".into());
        }
        Ok(AjustesClima { ciudad, ..self })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clima {
    pub ciudad: String,
    pub temperatura: f64,
    pub sensacion: Option<f64>,
    /// Código WMO de Open-Meteo.
    pub codigo: u32,
    /// "despejado", "lluvia leve"…
    pub descripcion: String,
    pub es_de_dia: bool,
    pub maxima: Option<f64>,
    pub minima: Option<f64>,
    /// Probabilidad de lluvia del día (%).
    pub lluvia: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiudadEncontrada {
    pub nombre: String,
    /// "Buenos Aires, Argentina"
    pub region: String,
    pub latitud: f64,
    pub longitud: f64,
}

pub fn url_pronostico(a: &AjustesClima) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.4}&longitude={:.4}\
         &current=temperature_2m,apparent_temperature,weather_code,is_day\
         &daily=temperature_2m_max,temperature_2m_min,precipitation_probability_max&timezone=auto&forecast_days=1",
        a.latitud, a.longitud
    )
}

/// El nombre de la ciudad va como parámetro, codificado (con tildes y espacios).
pub fn url_ciudades(texto: &str) -> Result<reqwest::Url, String> {
    reqwest::Url::parse_with_params(
        "https://geocoding-api.open-meteo.com/v1/search",
        &[("name", texto.trim()), ("count", "6"), ("language", "es"), ("format", "json")],
    )
    .map_err(|_| "No pude armar la búsqueda.".to_string())
}

/// Códigos WMO → palabras. https://open-meteo.com/en/docs (tabla "WMO Weather interpretation codes").
pub fn describir(codigo: u32) -> &'static str {
    match codigo {
        0 => "despejado",
        1 => "casi despejado",
        2 => "parcialmente nublado",
        3 => "nublado",
        45 | 48 => "niebla",
        51 | 53 | 55 => "llovizna",
        56 | 57 => "llovizna helada",
        61 => "lluvia leve",
        63 => "lluvia",
        65 => "lluvia fuerte",
        66 | 67 => "lluvia helada",
        71 | 73 | 75 | 77 => "nieve",
        80 | 81 => "chaparrones",
        82 => "chaparrones fuertes",
        85 | 86 => "nevadas",
        95 => "tormenta",
        96 | 99 => "tormenta con granizo",
        _ => "sin datos del cielo",
    }
}

fn primero(json: &Value, campo: &str) -> Option<f64> {
    json.get("daily")?.get(campo)?.get(0)?.as_f64()
}

pub fn leer_pronostico(json: &Value, ciudad: &str) -> Option<Clima> {
    let actual = json.get("current")?;
    let temperatura = actual.get("temperature_2m")?.as_f64()?;
    let codigo = actual.get("weather_code")?.as_u64()? as u32;
    Some(Clima {
        ciudad: ciudad.to_string(),
        temperatura,
        sensacion: actual.get("apparent_temperature").and_then(Value::as_f64),
        codigo,
        descripcion: describir(codigo).to_string(),
        es_de_dia: actual.get("is_day").and_then(Value::as_u64).unwrap_or(1) == 1,
        maxima: primero(json, "temperature_2m_max"),
        minima: primero(json, "temperature_2m_min"),
        lluvia: primero(json, "precipitation_probability_max").map(|p| p.round() as u32),
    })
}

pub fn leer_ciudades(json: &Value) -> Vec<CiudadEncontrada> {
    let Some(lista) = json.get("results").and_then(Value::as_array) else { return Vec::new() };
    lista
        .iter()
        .filter_map(|c| {
            let texto = |k: &str| c.get(k).and_then(Value::as_str).map(String::from);
            let nombre = texto("name")?;
            let region = [texto("admin1"), texto("country")].into_iter().flatten().collect::<Vec<_>>().join(", ");
            Some(CiudadEncontrada {
                nombre,
                region,
                latitud: c.get("latitude")?.as_f64()?,
                longitud: c.get("longitude")?.as_f64()?,
            })
        })
        .collect()
}

pub struct ClienteClima {
    http: reqwest::Client,
}

impl ClienteClima {
    pub fn nuevo() -> Resultado<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Nova/0.4 (app de escritorio personal)")
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteClima { http })
    }

    async fn json(&self, url: reqwest::Url) -> Resultado<Value> {
        let sin_conexion = |_| Error::Sistema("Sin conexión con el servicio del clima.".into());
        let respuesta = self.http.get(url).send().await.map_err(sin_conexion)?;
        if !respuesta.status().is_success() {
            return Err(Error::Sistema(format!("El servicio del clima respondió con un error ({}).", respuesta.status().as_u16())));
        }
        respuesta.json().await.map_err(|_| Error::Sistema("El servicio del clima respondió algo que no entiendo.".into()))
    }

    pub async fn pronostico(&self, a: &AjustesClima) -> Resultado<Clima> {
        let url = reqwest::Url::parse(&url_pronostico(a)).map_err(|e| Error::Sistema(e.to_string()))?;
        let json = self.json(url).await?;
        leer_pronostico(&json, &a.ciudad)
            .ok_or_else(|| Error::Sistema("El servicio del clima respondió algo que no entiendo.".into()))
    }

    pub async fn buscar(&self, texto: &str) -> Resultado<Vec<CiudadEncontrada>> {
        let url = url_ciudades(texto).map_err(Error::Validacion)?;
        Ok(leer_ciudades(&self.json(url).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_el_pronostico() {
        let json: Value = serde_json::from_str(include_str!("ejemplos/pronostico.json")).unwrap();
        let c = leer_pronostico(&json, "Buenos Aires").unwrap();
        assert_eq!(c.temperatura, 22.8);
        assert_eq!(c.descripcion, "despejado");
        assert!(c.es_de_dia);
        assert_eq!(c.maxima, Some(26.0));
        assert_eq!(c.lluvia, Some(65));
        assert!(leer_pronostico(&serde_json::json!({}), "X").is_none());
    }

    #[test]
    fn lee_ciudades() {
        let json: Value = serde_json::from_str(include_str!("ejemplos/ciudades.json")).unwrap();
        let c = leer_ciudades(&json);
        assert_eq!(c[0].nombre, "La Plata");
        assert_eq!(c[0].region, "Buenos Aires, Argentina");
        assert!(url_ciudades("San Martín").unwrap().as_str().contains("name=San+Mart%C3%ADn"));
    }

    #[test]
    fn valida_ajustes() {
        assert!(AjustesClima { ciudad: " ".into(), ..Default::default() }.validar().is_err());
        assert!(AjustesClima { latitud: 100.0, ..Default::default() }.validar().is_err());
        assert_eq!(AjustesClima { ciudad: " Rosario ".into(), ..Default::default() }.validar().unwrap().ciudad, "Rosario");
    }
}
