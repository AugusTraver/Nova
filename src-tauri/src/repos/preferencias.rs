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

/// Voz (`AjustesVoz`) y el día en que se leyó el resumen por última vez.
pub const VOZ: &str = "voz";
pub const RESUMEN_LEIDO: &str = "resumen_leido_el";

pub fn voz(conexion: &Connection) -> Resultado<crate::voz::AjustesVoz> {
    Ok(leer(conexion, VOZ)?.unwrap_or_default())
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

/// Qué notificar y "no molestar" (`AjustesAvisos`).
pub const AVISOS: &str = "avisos";

pub fn avisos(conexion: &Connection) -> Resultado<crate::dominio::avisos::AjustesAvisos> {
    Ok(leer(conexion, AVISOS)?.unwrap_or_default())
}

/// Cómo llamarte, el tono y "Sobre vos" (`Perfil`).
pub const PERFIL: &str = "perfil";

pub fn perfil(conexion: &Connection) -> Resultado<crate::dominio::perfil::Perfil> {
    Ok(leer(conexion, PERFIL)?.unwrap_or_default())
}

/// El acceso a la compu: interruptor, carpetas y páginas (`AjustesCompu`).
/// Si nunca se guardó: activado, con Documentos, Escritorio y Descargas.
pub const COMPU: &str = "compu";

pub fn compu(conexion: &Connection) -> Resultado<crate::compu::AjustesCompu> {
    Ok(leer(conexion, COMPU)?.unwrap_or_else(crate::compu::AjustesCompu::por_defecto))
}

/// Dónde dejó cada widget (`Disposicion`). Vacía = todos en su lugar.
pub const DISPOSICION: &str = "disposicion_widgets";

pub fn disposicion(conexion: &Connection) -> Resultado<crate::dominio::disposicion::Disposicion> {
    Ok(leer(conexion, DISPOSICION)?.unwrap_or_default())
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
