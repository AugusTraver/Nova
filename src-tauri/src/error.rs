//! Errores que pueden devolver los comandos.
//!
//! `thiserror` genera el código repetitivo: cada `#[error("...")]` es el texto
//! del error y `#[from]` permite usar `?` con errores de rusqlite o serde, que se
//! convierten solos en nuestro tipo.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error de la base de datos: {0}")]
    Base(#[from] rusqlite::Error),

    #[error("dato guardado inválido: {0}")]
    Json(#[from] serde_json::Error),

    /// Algo que mandó la interfaz no es válido. El texto se muestra tal cual.
    #[error("{0}")]
    Validacion(String),

    #[error("no existe {0}")]
    NoEncontrado(&'static str),

    #[error("{0}")]
    Sistema(String),
}

/// Así llega el error al frontend: `{ tipo: "validacion", mensaje: "..." }`.
/// Tauri necesita que el error se pueda serializar; lo hacemos a mano para
/// mandar solo lo útil y no detalles internos de rusqlite.
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let tipo = match self {
            Error::Base(_) | Error::Json(_) => "base",
            Error::Validacion(_) => "validacion",
            Error::NoEncontrado(_) => "no_encontrado",
            Error::Sistema(_) => "sistema",
        };
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("tipo", tipo)?;
        s.serialize_field("mensaje", &self.to_string())?;
        s.end()
    }
}

pub type Resultado<T> = Result<T, Error>;

/// Atajo para devolver un error de validación: `return invalido("...")`.
pub fn invalido<T>(mensaje: impl Into<String>) -> Resultado<T> {
    Err(Error::Validacion(mensaje.into()))
}
