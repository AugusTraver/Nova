//! Los mensajes de una conversación con el modelo, sin atarse a ningún
//! proveedor. Cada proveedor los traduce a su formato.

use serde::{Deserialize, Serialize};

/// Una herramienta que el modelo pidió ejecutar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Llamada {
    /// Lo pone el proveedor; la respuesta de la herramienta lo repite.
    pub id: String,
    pub nombre: String,
    /// Los argumentos como texto JSON, tal cual los mandó el modelo.
    pub argumentos: String,
    /// Lo que el proveedor pide que se le devuelva tal cual (la firma de
    /// Gemini, v0.7.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mensaje {
    Sistema(String),
    Usuario(String),
    /// Texto final, o (si `llamadas` no está vacía) un paso intermedio.
    Asistente { texto: String, llamadas: Vec<Llamada> },
    Herramienta { llamada_id: String, nombre: String, contenido: String },
}

/// Lo que contesta el modelo en una vuelta.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RespuestaModelo {
    pub texto: String,
    pub llamadas: Vec<Llamada>,
}
