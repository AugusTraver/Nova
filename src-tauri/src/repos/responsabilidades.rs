use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use super::{Ahora, fecha, materias, opcional};
use crate::dominio::texto::{EstadoResponsabilidad, OrigenResponsabilidad, Prioridad, TipoRegla};
use crate::error::{Error, Resultado, invalido};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Responsabilidad {
    pub id: i64,
    pub titulo: String,
    pub descripcion: Option<String>,
    /// Fecha sola ("2026-11-20").
    pub fecha_limite: Option<String>,
    pub prioridad: Prioridad,
    pub estado: EstadoResponsabilidad,
    pub hecha_en: Option<String>,
    pub evento_id: Option<i64>,
    pub materia_id: Option<i64>,
    pub origen: OrigenResponsabilidad,
    pub creado_en: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosResponsabilidad {
    pub titulo: String,
    pub descripcion: Option<String>,
    pub fecha_limite: Option<String>,
    pub prioridad: Prioridad,
    pub materia_id: Option<i64>,
    pub evento_id: Option<i64>,
}

fn validar(conexion: &Connection, datos: DatosResponsabilidad) -> Resultado<DatosResponsabilidad> {
    let titulo = datos.titulo.trim().to_string();
    if titulo.is_empty() {
        return invalido("La responsabilidad necesita un título.");
    }
    if titulo.chars().count() > 200 {
        return invalido("El título es muy largo.");
    }
    let fecha_limite = opcional(datos.fecha_limite);
    if let Some(texto) = &fecha_limite
        && (texto.len() != 10 || fecha(texto).is_none())
    {
        return invalido("La fecha límite no es válida.");
    }
    if let Some(id) = datos.materia_id
        && !materias::existe(conexion, id)?
    {
        return invalido("Esa materia no existe.");
    }
    if let Some(id) = datos.evento_id {
        let n: i64 = conexion.query_row("SELECT count(*) FROM eventos WHERE id = ?1", [id], |f| f.get(0))?;
        if n == 0 {
            return invalido("Ese evento no existe.");
        }
    }
    Ok(DatosResponsabilidad { titulo, descripcion: opcional(datos.descripcion), fecha_limite, ..datos })
}

const COLUMNAS: &str = "id, titulo, descripcion, fecha_limite, prioridad, estado, hecha_en, evento_id, \
                        materia_id, origen, creado_en";

fn desde_fila(fila: &Row) -> rusqlite::Result<Responsabilidad> {
    Ok(Responsabilidad {
        id: fila.get("id")?,
        titulo: fila.get("titulo")?,
        descripcion: fila.get("descripcion")?,
        fecha_limite: fila.get("fecha_limite")?,
        prioridad: fila.get("prioridad")?,
        estado: fila.get("estado")?,
        hecha_en: fila.get("hecha_en")?,
        evento_id: fila.get("evento_id")?,
        materia_id: fila.get("materia_id")?,
        origen: fila.get("origen")?,
        creado_en: fila.get("creado_en")?,
    })
}

/// Todas menos las automáticas descartadas. El orden para mostrar lo decide
/// el frontend (vencidas → hoy → próximas → sin fecha).
pub fn listar(conexion: &Connection) -> Resultado<Vec<Responsabilidad>> {
    let sql = format!("SELECT {COLUMNAS} FROM responsabilidades WHERE descartada = 0 ORDER BY id");
    let lista = conexion.prepare(&sql)?.query_map([], desde_fila)?.collect::<Result<_, _>>()?;
    Ok(lista)
}

pub fn obtener(conexion: &Connection, id: i64) -> Resultado<Responsabilidad> {
    let sql = format!("SELECT {COLUMNAS} FROM responsabilidades WHERE id = ?1 AND descartada = 0");
    conexion.query_row(&sql, [id], desde_fila).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Error::NoEncontrado("esa responsabilidad"),
        otro => otro.into(),
    })
}

pub fn crear(
    conexion: &Connection,
    datos: DatosResponsabilidad,
    origen: OrigenResponsabilidad,
    ahora: &Ahora,
) -> Resultado<Responsabilidad> {
    let d = validar(conexion, datos)?;
    conexion.execute(
        "INSERT INTO responsabilidades (titulo, descripcion, fecha_limite, prioridad, evento_id, materia_id,
                                        origen, creado_en, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![d.titulo, d.descripcion, d.fecha_limite, d.prioridad, d.evento_id, d.materia_id, origen, ahora.marca],
    )?;
    obtener(conexion, conexion.last_insert_rowid())
}

/// Edita a mano. Si era automática, queda marcada como editada y la
/// sincronización deja de tocarla.
pub fn editar(conexion: &Connection, id: i64, datos: DatosResponsabilidad, ahora: &Ahora) -> Resultado<Responsabilidad> {
    let anterior = obtener(conexion, id)?;
    let d = validar(conexion, datos)?;
    let cambio = anterior.titulo != d.titulo
        || anterior.fecha_limite != d.fecha_limite
        || anterior.prioridad != d.prioridad
        || anterior.materia_id != d.materia_id
        || anterior.descripcion != d.descripcion;
    conexion.execute(
        "UPDATE responsabilidades
         SET titulo = ?1, descripcion = ?2, fecha_limite = ?3, prioridad = ?4, materia_id = ?5, evento_id = ?6,
             editada_a_mano = editada_a_mano OR (?7 AND regla IS NOT NULL), actualizado_en = ?8
         WHERE id = ?9",
        params![d.titulo, d.descripcion, d.fecha_limite, d.prioridad, d.materia_id, d.evento_id, cambio, ahora.marca, id],
    )?;
    obtener(conexion, id)
}

pub fn marcar(conexion: &Connection, id: i64, hecha: bool, ahora: &Ahora) -> Resultado<Responsabilidad> {
    let (estado, hecha_en) = if hecha {
        (EstadoResponsabilidad::Hecha, Some(ahora.marca.as_str()))
    } else {
        (EstadoResponsabilidad::Pendiente, None)
    };
    let cambiadas = conexion.execute(
        "UPDATE responsabilidades SET estado = ?1, hecha_en = ?2, actualizado_en = ?3 WHERE id = ?4 AND descartada = 0",
        params![estado, hecha_en, ahora.marca, id],
    )?;
    if cambiadas == 0 {
        return Err(Error::NoEncontrado("esa responsabilidad"));
    }
    obtener(conexion, id)
}

/// Las manuales se borran. Las automáticas quedan ocultas ("descartadas")
/// para que la sincronización no las vuelva a crear.
pub fn borrar(conexion: &Connection, id: i64) -> Resultado<()> {
    let regla: Option<Option<TipoRegla>> = conexion
        .query_row("SELECT regla FROM responsabilidades WHERE id = ?1", [id], |f| f.get(0))
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            otro => Err(otro),
        })?;
    match regla {
        None => Ok(()), // ya no existía
        Some(None) => {
            conexion.execute("DELETE FROM responsabilidades WHERE id = ?1", [id])?;
            Ok(())
        }
        Some(Some(_)) => {
            conexion.execute("UPDATE responsabilidades SET descartada = 1 WHERE id = ?1", [id])?;
            Ok(())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db;

    pub fn datos(titulo: &str, fecha_limite: Option<&str>) -> DatosResponsabilidad {
        DatosResponsabilidad {
            titulo: titulo.into(),
            descripcion: None,
            fecha_limite: fecha_limite.map(Into::into),
            prioridad: Prioridad::Media,
            materia_id: None,
            evento_id: None,
        }
    }

    #[test]
    fn ciclo_completo_de_una_manual() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let r = crear(&base, datos("Llamar al dentista", None), OrigenResponsabilidad::Manual, &ahora).unwrap();
        assert_eq!(r.estado, EstadoResponsabilidad::Pendiente);

        let r = marcar(&base, r.id, true, &ahora).unwrap();
        assert_eq!(r.estado, EstadoResponsabilidad::Hecha);
        assert!(r.hecha_en.is_some());
        let r = marcar(&base, r.id, false, &ahora).unwrap();
        assert_eq!(r.hecha_en, None);

        let r = editar(&base, r.id, datos("Llamar al dentista (urgente)", Some("2026-11-18")), &ahora).unwrap();
        assert_eq!(r.fecha_limite.as_deref(), Some("2026-11-18"));

        borrar(&base, r.id).unwrap();
        assert!(listar(&base).unwrap().is_empty());
        let filas: i64 = base.query_row("SELECT count(*) FROM responsabilidades", [], |f| f.get(0)).unwrap();
        assert_eq!(filas, 0, "una manual se borra de verdad");
    }

    #[test]
    fn valida_lo_que_llega() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let origen = OrigenResponsabilidad::Manual;
        assert!(crear(&base, datos("  ", None), origen, &ahora).is_err());
        assert!(crear(&base, datos("x", Some("viernes")), origen, &ahora).is_err());
        assert!(crear(&base, datos("x", Some("2026-11-18T10:00:00-03:00")), origen, &ahora).is_err());
        let mut con_materia = datos("x", None);
        con_materia.materia_id = Some(7);
        assert!(crear(&base, con_materia, origen, &ahora).is_err());
    }
}
