use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use super::Ahora;
use crate::error::{Error, Resultado, invalido};

/// Colores posibles para una materia: nombres de la paleta de `tokens.css`
/// (`--color-materia-*`). Tiene que coincidir con `src/lib/tipos.ts`.
pub const PALETA: &[&str] = &["celeste", "coral", "menta", "lavanda", "ambar", "rosa"];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Materia {
    pub id: i64,
    pub nombre: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosMateria {
    pub nombre: String,
    pub color: String,
}

fn validar(datos: DatosMateria) -> Resultado<DatosMateria> {
    let nombre = datos.nombre.trim().to_string();
    if nombre.is_empty() {
        return invalido("La materia necesita un nombre.");
    }
    if nombre.chars().count() > 80 {
        return invalido("El nombre de la materia es muy largo.");
    }
    if !PALETA.contains(&datos.color.as_str()) {
        return invalido("Ese color no está en la paleta.");
    }
    Ok(DatosMateria { nombre, color: datos.color })
}

fn desde_fila(fila: &Row) -> rusqlite::Result<Materia> {
    Ok(Materia { id: fila.get("id")?, nombre: fila.get("nombre")?, color: fila.get("color")? })
}

pub fn listar(conexion: &Connection) -> Resultado<Vec<Materia>> {
    let mut consulta = conexion.prepare("SELECT id, nombre, color FROM materias ORDER BY nombre COLLATE NOCASE")?;
    // `query_map` devuelve un iterador de Result; `collect` lo junta en un solo
    // Result: si alguna fila falla, falla todo.
    let materias = consulta.query_map([], desde_fila)?.collect::<Result<_, _>>()?;
    Ok(materias)
}

pub fn obtener(conexion: &Connection, id: i64) -> Resultado<Materia> {
    conexion
        .query_row("SELECT id, nombre, color FROM materias WHERE id = ?1", [id], desde_fila)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NoEncontrado("esa materia"),
            otro => otro.into(),
        })
}

pub fn existe(conexion: &Connection, id: i64) -> Resultado<bool> {
    let n: i64 = conexion.query_row("SELECT count(*) FROM materias WHERE id = ?1", [id], |f| f.get(0))?;
    Ok(n > 0)
}

pub fn crear(conexion: &Connection, datos: DatosMateria, ahora: &Ahora) -> Resultado<Materia> {
    let datos = validar(datos)?;
    conexion.execute(
        "INSERT INTO materias (nombre, color, creado_en) VALUES (?1, ?2, ?3)",
        params![datos.nombre, datos.color, ahora.marca],
    )?;
    obtener(conexion, conexion.last_insert_rowid())
}

pub fn editar(conexion: &Connection, id: i64, datos: DatosMateria) -> Resultado<Materia> {
    let datos = validar(datos)?;
    let cambiadas = conexion.execute(
        "UPDATE materias SET nombre = ?1, color = ?2 WHERE id = ?3",
        params![datos.nombre, datos.color, id],
    )?;
    if cambiadas == 0 {
        return Err(Error::NoEncontrado("esa materia"));
    }
    obtener(conexion, id)
}

/// Los eventos y responsabilidades de la materia quedan, sin materia.
pub fn borrar(conexion: &Connection, id: i64) -> Resultado<()> {
    conexion.execute("DELETE FROM materias WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn datos(nombre: &str, color: &str) -> DatosMateria {
        DatosMateria { nombre: nombre.into(), color: color.into() }
    }

    #[test]
    fn crea_lista_edita_y_borra() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let analisis = crear(&base, datos("  Análisis II ", "celeste"), &ahora).unwrap();
        crear(&base, datos("Algoritmos", "coral"), &ahora).unwrap();
        assert_eq!(analisis.nombre, "Análisis II");

        let nombres: Vec<_> = listar(&base).unwrap().into_iter().map(|m| m.nombre).collect();
        assert_eq!(nombres, ["Algoritmos", "Análisis II"]);

        let editada = editar(&base, analisis.id, datos("Análisis Matemático II", "menta")).unwrap();
        assert_eq!(editada.color, "menta");

        borrar(&base, analisis.id).unwrap();
        assert_eq!(listar(&base).unwrap().len(), 1);
    }

    #[test]
    fn valida_nombre_y_color() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        assert!(matches!(crear(&base, datos(" ", "celeste"), &ahora), Err(Error::Validacion(_))));
        assert!(matches!(crear(&base, datos("Física", "#ff0000"), &ahora), Err(Error::Validacion(_))));
        assert!(matches!(editar(&base, 99, datos("Física", "rosa")), Err(Error::NoEncontrado(_))));
    }
}
