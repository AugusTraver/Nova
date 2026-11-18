//! La capa de proveedores de IA (SPEC 5.3).
//!
//! `ProveedorIa` es un *trait*: una interfaz que cualquier proveedor puede
//! implementar. El ciclo del asistente solo conoce el trait, así que cambiar
//! de Groq a Ollama (o sumar otro) no toca el resto del código.

use std::future::Future;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TipoProveedor {
    #[default]
    Groq,
    Ollama,
}

impl TipoProveedor {
    /// Nombre de la cuenta en el llavero para su API key (Ollama no usa).
    pub fn cuenta_llavero(self) -> Option<&'static str> {
        match self {
            TipoProveedor::Groq => Some("groq-api-key"),
            TipoProveedor::Ollama => None,
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            TipoProveedor::Groq => "Groq",
            TipoProveedor::Ollama => "Ollama",
        }
    }
}

pub const URL_GROQ: &str = "https://api.groq.com/openai/v1";
pub const MODELO_GROQ: &str = "openai/gpt-oss-120b";
pub const URL_OLLAMA: &str = "http://localhost:11434";
pub const MODELO_OLLAMA: &str = "qwen3:8b";

/// Lo que se guarda en preferencias (clave `asistente`). Sin la key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesIa {
    pub proveedor: TipoProveedor,
    pub modelo_groq: String,
    pub modelo_ollama: String,
    pub url_ollama: String,
}

impl Default for AjustesIa {
    fn default() -> Self {
        AjustesIa {
            proveedor: TipoProveedor::Groq,
            modelo_groq: MODELO_GROQ.into(),
            modelo_ollama: MODELO_OLLAMA.into(),
            url_ollama: URL_OLLAMA.into(),
        }
    }
}

impl AjustesIa {
    pub fn validar(self) -> Result<Self, String> {
        let modelo_groq = self.modelo_groq.trim().to_string();
        let modelo_ollama = self.modelo_ollama.trim().to_string();
        let url_ollama = self.url_ollama.trim().trim_end_matches('/').to_string();
        if modelo_groq.is_empty() || modelo_ollama.is_empty() {
            return Err("Elegí un modelo.".into());
        }
        if !(url_ollama.starts_with("http://") || url_ollama.starts_with("https://")) {
            return Err("La dirección de Ollama tiene que empezar con http://".into());
        }
        Ok(AjustesIa { modelo_groq, modelo_ollama, url_ollama, ..self })
    }

    /// Arma el cliente para el proveedor elegido. Groq y Ollama hablan el
    /// mismo formato (el de OpenAI), así que comparten la implementación.
    pub fn cliente(&self, api_key: Option<String>) -> Resultado<ClienteCompatible> {
        match self.proveedor {
            TipoProveedor::Groq => {
                ClienteCompatible::nuevo(TipoProveedor::Groq, URL_GROQ.into(), self.modelo_groq.clone(), api_key)
            }
            TipoProveedor::Ollama => ClienteCompatible::nuevo(
                TipoProveedor::Ollama,
                format!("{}/v1", self.url_ollama),
                self.modelo_ollama.clone(),
                None,
            ),
        }
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
    }

    #[test]
    fn lee_ajustes_viejos_o_incompletos() {
        let leidos: AjustesIa = serde_json::from_str(r#"{ "proveedor": "ollama" }"#).unwrap();
        assert_eq!(leidos.proveedor, TipoProveedor::Ollama);
        assert_eq!(leidos.modelo_groq, MODELO_GROQ);
    }
}
