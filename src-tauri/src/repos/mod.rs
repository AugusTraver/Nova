//! Acceso a la base: una función por operación, todas reciben la conexión.
//! No saben nada de Tauri, así que se testean con una base en memoria.

pub mod automaticas;
pub mod eventos;
pub mod materias;
pub mod preferencias;
pub mod responsabilidades;

use chrono::{Local, NaiveDate, SecondsFormat};

/// "Ahora" se pasa como parámetro en vez de leer el reloj adentro de cada
/// función: así los tests pueden fijar el día y los resultados no cambian.
#[derive(Debug, Clone)]
pub struct Ahora {
    /// El día de hoy en la zona horaria del sistema.
    pub hoy: NaiveDate,
    /// Marca de tiempo ISO 8601 con zona, para `creado_en` y similares.
    pub marca: String,
}

impl Ahora {
    pub fn del_sistema() -> Self {
        let ahora = Local::now();
        Ahora {
            hoy: ahora.date_naive(),
            marca: ahora.to_rfc3339_opts(SecondsFormat::Secs, false),
        }
    }

    #[cfg(test)]
    pub fn fijo(hoy: &str) -> Self {
        Ahora {
            hoy: fecha(hoy).expect("fecha de test"),
            marca: format!("{hoy}T12:00:00-03:00"),
        }
    }
}

/// "2026-11-20" → fecha. También acepta una marca completa ("2026-11-20T...")
/// y se queda con el día.
pub fn fecha(texto: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texto.get(..10)?, "%Y-%m-%d").ok()
}

pub fn texto_fecha(fecha: NaiveDate) -> String {
    fecha.format("%Y-%m-%d").to_string()
}

/// Recorta espacios y convierte "" en `None` (para campos opcionales de texto).
pub fn opcional(texto: Option<String>) -> Option<String> {
    texto.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}
