//! Acceso a la base: una función por operación, todas reciben la conexión.
//! No saben nada de Tauri, así que se testean con una base en memoria.

pub mod automaticas;
pub mod avisos;
pub mod cache;
pub mod cambios_archivos;
pub mod eventos;
pub mod google;
pub mod materias;
pub mod mensajes;
pub mod preferencias;
pub mod responsabilidades;
pub mod temporizadores;

use chrono::{DateTime, FixedOffset, Local, NaiveDate, SecondsFormat};

/// "Ahora" se pasa como parámetro en vez de leer el reloj adentro de cada
/// función: así los tests pueden fijar el día y los resultados no cambian.
#[derive(Debug, Clone)]
pub struct Ahora {
    /// El día de hoy en la zona horaria del sistema.
    pub hoy: NaiveDate,
    /// Marca de tiempo ISO 8601 con zona, para `creado_en` y similares.
    pub marca: String,
    /// El momento exacto con la zona del sistema (para la hora y para armar
    /// fechas con hora desde lo que dice el usuario).
    pub momento: DateTime<FixedOffset>,
}

impl Ahora {
    pub fn del_sistema() -> Self {
        // `fixed_offset()` se queda con el huso actual (-03:00) en vez de
        // la zona "Local", que no se puede guardar ni comparar fácil.
        Self::desde(Local::now().fixed_offset())
    }

    pub fn desde(momento: DateTime<FixedOffset>) -> Self {
        Ahora {
            hoy: momento.date_naive(),
            marca: momento.to_rfc3339_opts(SecondsFormat::Secs, false),
            momento,
        }
    }

    /// Martes 17 de noviembre de 2026 a las 12:00 en Argentina, salvo que se
    /// pida otro día.
    #[cfg(test)]
    pub fn fijo(hoy: &str) -> Self {
        Self::fijo_a_las(hoy, "12:00")
    }

    #[cfg(test)]
    pub fn fijo_a_las(hoy: &str, hora: &str) -> Self {
        let momento = DateTime::parse_from_rfc3339(&format!("{hoy}T{hora}:00-03:00")).expect("fecha de test");
        Self::desde(momento)
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
