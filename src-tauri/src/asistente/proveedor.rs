//! La capa de proveedores de IA (SPEC 5.3).
//!
//! `ProveedorIa` es un *trait*: una interfaz que cualquier proveedor puede
//! implementar. El ciclo del asistente solo conoce el trait, así que cambiar
//! de Groq a Ollama (o sumar otro) no toca el resto del código.

use std::future::Future;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cadena::CadenaIa;
use super::mensajes::{Mensaje, RespuestaModelo};
use super::openai::ClienteCompatible;
use crate::error::Resultado;

pub trait ProveedorIa {
    /// Manda la conversación y las herramientas; devuelve texto o llamadas.
    ///
    /// `impl Future + Send` es la forma de escribir "una función async" en un
    /// trait, con la promesa de que se puede mover entre hilos (Tauri corre
    /// los comandos async en un grupo de hilos).
    fn completar(&self, mensajes: &[Mensaje], herramientas: &Value) -> impl Future<Output = Resultado<RespuestaModelo>> + Send;
}

/// Qué proveedor eligió el usuario en Ajustes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TipoProveedor {
    #[default]
    Groq,
    Ollama,
    /// v0.7.3: Gemini, por su API compatible con OpenAI.
    Gemini,
}

impl TipoProveedor {
    /// Nombre de la cuenta en el llavero para su API key (Ollama no usa).
    pub fn cuenta_llavero(self) -> Option<&'static str> {
        match self {
            TipoProveedor::Groq => Some("groq-api-key"),
            TipoProveedor::Gemini => Some("gemini-api-key"),
            TipoProveedor::Ollama => None,
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            TipoProveedor::Groq => "Groq",
            TipoProveedor::Ollama => "Ollama",
            TipoProveedor::Gemini => "Gemini",
        }
    }

    /// Una key que "parece" de este proveedor (para avisar si se pegó mal).
    pub fn key_valida(self, key: &str) -> bool {
        let forma = !key.is_empty() && key.len() <= 300 && !key.chars().any(char::is_whitespace);
        match self {
            TipoProveedor::Gemini => forma && key.len() >= 20,
            _ => forma,
        }
    }
}

pub const URL_GROQ: &str = "https://api.groq.com/openai/v1";
pub const MODELO_GROQ: &str = "openai/gpt-oss-120b";
pub const URL_OLLAMA: &str = "http://localhost:11434";
pub const MODELO_OLLAMA: &str = "qwen3:8b";
pub const URL_GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
/// El de más cupo en el plan gratis (15 pedidos y 250.000 tokens por minuto).
pub const MODELO_GEMINI: &str = "gemini-3.5-flash-lite";

/// Lo que se guarda en preferencias (clave `asistente`). Sin las keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesIa {
    pub proveedor: TipoProveedor,
    pub modelo_groq: String,
    pub modelo_ollama: String,
    pub url_ollama: String,
    /// v0.7.3.
    pub modelo_gemini: String,
    /// Al que se pasa cuando el principal se queda sin cupo (v0.7.3). Si no
    /// tiene key, no se usa.
    pub respaldo: Option<TipoProveedor>,
}

impl Default for AjustesIa {
    fn default() -> Self {
        AjustesIa {
            proveedor: TipoProveedor::Groq,
            modelo_groq: MODELO_GROQ.into(),
            modelo_ollama: MODELO_OLLAMA.into(),
            url_ollama: URL_OLLAMA.into(),
            modelo_gemini: MODELO_GEMINI.into(),
            respaldo: Some(TipoProveedor::Gemini),
        }
    }
}

impl AjustesIa {
    pub fn validar(self) -> Result<Self, String> {
        let modelo_groq = self.modelo_groq.trim().to_string();
        let modelo_ollama = self.modelo_ollama.trim().to_string();
        let modelo_gemini = self.modelo_gemini.trim().to_string();
        let url_ollama = self.url_ollama.trim().trim_end_matches('/').to_string();
        if modelo_groq.is_empty() || modelo_ollama.is_empty() || modelo_gemini.is_empty() {
            return Err("Elegí un modelo.".into());
        }
        if !(url_ollama.starts_with("http://") || url_ollama.starts_with("https://")) {
            return Err("La dirección de Ollama tiene que empezar con http://".into());
        }
        // El respaldo de uno mismo no sirve de nada.
        let respaldo = self.respaldo.filter(|r| *r != self.proveedor);
        Ok(AjustesIa { modelo_groq, modelo_ollama, modelo_gemini, url_ollama, respaldo, ..self })
    }

    /// El cliente de un proveedor. Groq, Gemini y Ollama hablan el mismo
    /// formato (el de OpenAI), así que comparten la implementación.
    pub fn cliente_de(&self, tipo: TipoProveedor, api_key: Option<String>) -> Resultado<ClienteCompatible> {
        let (base, modelo) = match tipo {
            TipoProveedor::Groq => (URL_GROQ.to_string(), self.modelo_groq.clone()),
            TipoProveedor::Gemini => (URL_GEMINI.to_string(), self.modelo_gemini.clone()),
            TipoProveedor::Ollama => (format!("{}/v1", self.url_ollama), self.modelo_ollama.clone()),
        };
        ClienteCompatible::nuevo(tipo, base, modelo, api_key)
    }

    /// El principal y, si tiene key, el respaldo. `leer_key` recibe la
    /// cuenta del llavero (en los tests, cualquier otra cosa).
    pub fn cadena(&self, leer_key: impl Fn(&str) -> Resultado<Option<String>>) -> Resultado<CadenaIa> {
        let key_de = |tipo: TipoProveedor| match tipo.cuenta_llavero() {
            Some(cuenta) => leer_key(cuenta),
            None => Ok(None),
        };
        let principal = self.cliente_de(self.proveedor, key_de(self.proveedor)?)?;
        // Si al respaldo le falta la key, no está: no es un error.
        let respaldo = match self.respaldo.filter(|r| *r != self.proveedor) {
            Some(tipo) => self.cliente_de(tipo, key_de(tipo)?).ok(),
            None => None,
        };
        Ok(CadenaIa::nueva(principal, respaldo))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_y_normaliza() {
        let ajustes = AjustesIa { url_ollama: " http://localhost:11434/ ".into(), ..Default::default() };
        assert_eq!(ajustes.validar().unwrap().url_ollama, "http://localhost:11434");
        let mal = AjustesIa { url_ollama: "localhost".into(), ..Default::default() };
        assert!(mal.validar().is_err());
        let sin_modelo = AjustesIa { modelo_groq: " ".into(), ..Default::default() };
        assert!(sin_modelo.validar().is_err());
        let sin_gemini = AjustesIa { modelo_gemini: "".into(), ..Default::default() };
        assert!(sin_gemini.validar().is_err());
    }

    #[test]
    fn lee_ajustes_viejos_o_incompletos() {
        let leidos: AjustesIa = serde_json::from_str(r#"{ "proveedor": "ollama" }"#).unwrap();
        assert_eq!(leidos.proveedor, TipoProveedor::Ollama);
        assert_eq!(leidos.modelo_groq, MODELO_GROQ);
        // Los de antes de v0.7.3 no tenían respaldo: queda Gemini.
        assert_eq!(leidos.modelo_gemini, MODELO_GEMINI);
        assert_eq!(leidos.respaldo, Some(TipoProveedor::Gemini));
        let sin: AjustesIa = serde_json::from_str(r#"{ "respaldo": null }"#).unwrap();
        assert_eq!(sin.respaldo, None);
    }

    #[test]
    fn el_respaldo_no_puede_ser_el_mismo() {
        let ajustes = AjustesIa { proveedor: TipoProveedor::Gemini, ..Default::default() };
        assert_eq!(ajustes.validar().unwrap().respaldo, None);
    }

    #[test]
    fn arma_la_cadena_con_las_keys_que_hay() {
        let ajustes = AjustesIa::default();
        let con_las_dos = ajustes.cadena(|_| Ok(Some("una-key-de-prueba-bastante-larga".into()))).unwrap();
        assert_eq!(con_las_dos.tipos(), (TipoProveedor::Groq, Some(TipoProveedor::Gemini)));
        let sin_gemini = ajustes.cadena(|c| Ok((c == "groq-api-key").then(|| "gsk".to_string()))).unwrap();
        assert_eq!(sin_gemini.tipos(), (TipoProveedor::Groq, None));
        // Sin la key del principal no arranca.
        assert!(ajustes.cadena(|_| Ok(None)).is_err());
        let local = AjustesIa { proveedor: TipoProveedor::Gemini, respaldo: Some(TipoProveedor::Ollama), ..Default::default() };
        let cadena = local.cadena(|_| Ok(Some("k".repeat(30)))).unwrap();
        assert_eq!(cadena.tipos(), (TipoProveedor::Gemini, Some(TipoProveedor::Ollama)));
    }

    #[test]
    fn reconoce_las_keys() {
        assert!(TipoProveedor::Gemini.key_valida("AIzaSyA-una-key-de-prueba-123"));
        assert!(!TipoProveedor::Gemini.key_valida("corta"));
        assert!(!TipoProveedor::Groq.key_valida("con espacio"));
        assert!(!TipoProveedor::Groq.key_valida(""));
    }
}
