//! Temporizadores guardados (v0.7.1). La tarea de fondo los revisa y los
//! borra apenas avisan.

use chrono::{DateTime, FixedOffset};
use rusqlite::{Connection, params};

use super::Ahora;
use crate::error::Resultado;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Temporizador {
    pub id: i64,
    pub texto: String,
    pub vence_en: String,
}

pub fn crear(conexion: &Connection, texto: &str, vence: DateTime<FixedOffset>, ahora: &Ahora) -> Resultado<i64> {
    conexion.execute(
        "INSERT INTO temporizadores (texto, vence_en, creado_en) VALUES (?1, ?2, ?3)",
        params![texto, vence.to_rfc3339(), ahora.marca],
    )?;
    Ok(conexion.last_insert_rowid())
}

fn todos(conexion: &Connection) -> Resultado<Vec<(Temporizador, DateTime<FixedOffset>)>> {
    let lista: Vec<Temporizador> = conexion
        .prepare("SELECT id, texto, vence_en FROM temporizadores")?
        .query_map([], |f| Ok(Temporizador { id: f.get(0)?, texto: f.get(1)?, vence_en: f.get(2)? }))?
        .collect::<Result<_, _>>()?;
    // Se compara como momento (no como texto): el huso podría ser otro.
    Ok(lista
        .into_iter()
        .filter_map(|t| DateTime::parse_from_rfc3339(&t.vence_en).ok().map(|v| (t, v)))
        .collect())
}

/// Los que ya vencieron. Se sacan de la tabla al devolverlos: así un error al
/// mostrar el aviso no lo repite cada vuelta.
pub fn tomar_vencidos(conexion: &Connection, ahora: &Ahora) -> Resultado<Vec<Temporizador>> {
    let vencidos: Vec<Temporizador> =
        todos(conexion)?.into_iter().filter(|(_, v)| *v <= ahora.momento).map(|(t, _)| t).collect();
    for t in &vencidos {
        conexion.execute("DELETE FROM temporizadores WHERE id = ?1", [t.id])?;
    }
    Ok(vencidos)
}

/// El próximo en vencer, para que la tarea de fondo se despierte a tiempo.
pub fn proximo(conexion: &Connection) -> Resultado<Option<DateTime<FixedOffset>>> {
    Ok(todos(conexion)?.into_iter().map(|(_, v)| v).min())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn avisa_los_vencidos_una_sola_vez() {
        let base = db::en_memoria();
        let antes = Ahora::fijo_a_las("2026-11-20", "17:00");
        crear(&base, "sacar la ropa", antes.momento + chrono::Duration::minutes(10), &antes).unwrap();
        crear(&base, "llamar a mamá", antes.momento + chrono::Duration::minutes(60), &antes).unwrap();
        assert!(tomar_vencidos(&base, &antes).unwrap().is_empty());
        assert_eq!(proximo(&base).unwrap().unwrap().to_rfc3339(), "2026-11-20T17:10:00-03:00");

        let despues = Ahora::fijo_a_las("2026-11-20", "17:10");
        let vencidos = tomar_vencidos(&base, &despues).unwrap();
        assert_eq!(vencidos.iter().map(|t| t.texto.as_str()).collect::<Vec<_>>(), ["sacar la ropa"]);
        assert!(tomar_vencidos(&base, &despues).unwrap().is_empty());
        assert_eq!(proximo(&base).unwrap().unwrap().to_rfc3339(), "2026-11-20T18:00:00-03:00");
    }
}
