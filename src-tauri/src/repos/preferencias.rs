//! Preferencias: pares clave → valor JSON.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::dominio::reglas::Reglas;
use crate::error::Resultado;

pub const ATAJO: &str = "atajo";
pub const REGLAS: &str = "reglas_auto_responsabilidades";

/// Proveedor y modelo de IA (`AjustesIa`). La API key va en el llavero.
pub const IA: &str = "proveedor_ia";

pub const ATAJO_POR_DEFECTO: &str = "Ctrl+Shift+Space";

/// Jugadores seguidos (`Vec<JugadorSeguido>`).
pub const JUGADORES: &str = "jugadores_seguidos";

pub fn jugadores(conexion: &Connection) -> Resultado<Vec<crate::futbol::JugadorSeguido>> {
    Ok(leer(conexion, JUGADORES)?.unwrap_or_else(crate::futbol::jugadores_por_defecto))
}

/// Intereses y feeds de noticias (`AjustesNoticias`).
pub const NOTICIAS: &str = "noticias";
/// Ciudad del clima y si se muestra (`AjustesClima`).
pub const CLIMA: &str = "clima";

pub fn noticias(conexion: &Connection) -> Resultado<crate::noticias::AjustesNoticias> {
    Ok(leer(conexion, NOTICIAS)?.unwrap_or_default())
}

pub fn clima(conexion: &Connection) -> Resultado<crate::clima::AjustesClima> {
    Ok(leer(conexion, CLIMA)?.unwrap_or_default())
}

pub fn ajustes_ia(conexion: &Connection) -> Resultado<crate::asistente::proveedor::AjustesIa> {
    Ok(leer(conexion, IA)?.unwrap_or_default())
}

/// Lee una preferencia. `None` si todavía no se guardó.
///
/// `T: DeserializeOwned` quiere decir "cualquier tipo que serde sepa armar
/// desde JSON": la misma función sirve para un texto, las reglas, etc.
pub fn leer<T: DeserializeOwned>(conexion: &Connection, clave: &str) -> Resultado<Option<T>> {
    let json: Option<String> = conexion
        .query_row("SELECT valor_json FROM preferencias WHERE clave = ?1", [clave], |f| f.get(0))
        .optional()?; // `optional()` convierte "no hay filas" en None en vez de error
    Ok(match json {
        Some(json) => Some(serde_json::from_str(&json)?),
        None => None,
    })
}

pub fn guardar<T: Serialize>(conexion: &Connection, clave: &str, valor: &T) -> Resultado<()> {
    conexion.execute(
        "INSERT INTO preferencias (clave, valor_json) VALUES (?1, ?2)
         ON CONFLICT (clave) DO UPDATE SET valor_json = excluded.valor_json",
        params![clave, serde_json::to_string(valor)?],
    )?;
    Ok(())
}

pub fn reglas(conexion: &Connection) -> Resultado<Reglas> {
    Ok(leer(conexion, REGLAS)?.unwrap_or_default())
}

pub fn atajo(conexion: &Connection) -> Resultado<String> {
    Ok(leer(conexion, ATAJO)?.unwrap_or_else(|| ATAJO_POR_DEFECTO.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn valores_por_defecto_y_guardados() {
        let base = db::en_memoria();
        assert_eq!(atajo(&base).unwrap(), ATAJO_POR_DEFECTO);
        assert_eq!(reglas(&base).unwrap(), Reglas::default());

        guardar(&base, ATAJO, &"Alt+Space").unwrap();
        guardar(&base, ATAJO, &"Ctrl+Alt+N").unwrap(); // pisa el anterior
        assert_eq!(atajo(&base).unwrap(), "Ctrl+Alt+N");

        let mut nuevas = Reglas::default();
        nuevas.cumpleanos.dias_antes = 5;
        guardar(&base, REGLAS, &nuevas).unwrap();
        assert_eq!(reglas(&base).unwrap().cumpleanos.dias_antes, 5);
    }
}
