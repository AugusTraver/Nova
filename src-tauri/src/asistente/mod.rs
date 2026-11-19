//! El asistente por texto (v0.3).
//!
//! - `mensajes`: los mensajes de una conversación, sin atarse a un proveedor.
//! - `proveedor`: el trait `ProveedorIa` y los ajustes de IA.
//! - `openai`: el cliente compatible con OpenAI (Groq y Ollama).
//! - `herramientas`: lo que el modelo puede hacer, sobre `repos::*`.
//! - `prompt`: las instrucciones de sistema de cada turno.
//! - `seleccion`: qué herramientas se mandan según el pedido (gasta menos).
//! - `ciclo`: un turno completo.

pub mod ciclo;
pub mod consultas;
pub mod herramientas;
pub mod mensajes;
pub mod openai;
pub mod prompt;
pub mod proveedor;
pub mod seleccion;
