//! Los cambios que Nova hizo en archivos (v0.7.1), para "deshacé eso".
//! Las copias en sí están en el disco (`compu::archivos`); acá queda qué se
//! hizo, cuándo y dónde está la copia.

use rusqlite::{Connection, OptionalExtension, params};

use super::Ahora;
use crate::error::Resultado;

/// Cuánto se puede deshacer para atrás, y cuántos cambios se guardan.
pub const DIAS_DE_COPIAS: i64 = 7;
pub const MAXIMO_DE_COPIAS: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionArchivo {
    Cambiar,
    Crear,
    Borrar,
    Deshacer,
}

impl AccionArchivo {
    fn texto(self) -> &'static str {
        match self {
            AccionArchivo::Cambiar => "cambiar",
            AccionArchivo::Crear => "crear",
            AccionArchivo::Borrar => "borrar",
            AccionArchivo::Deshacer => "deshacer",
        }
    }

    fn desde(texto: &str) -> Self {
        match texto {
            "crear" => AccionArchivo::Crear,
            "borrar" => AccionArchivo::Borrar,
            "deshacer" => AccionArchivo::Deshacer,
            _ => AccionArchivo::Cambiar,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioGuardado {
    pub id: i64,
    pub ruta: String,
    pub accion: AccionArchivo,
    pub copia: Option<String>,
    pub huella_despues: Option<String>,
    pub hecho_en: String,
}

pub fn anotar(
    conexion: &Connection,
    ruta: &str,
    accion: AccionArchivo,
    copia: Option<&str>,
    huella_despues: Option<&str>,
    ahora: &Ahora,
) -> Resultado<i64> {
    conexion.execute(
        "INSERT INTO cambios_archivos (ruta, accion, copia, huella_despues, hecho_en) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![ruta, accion.texto(), copia, huella_despues, ahora.marca],
    )?;
    Ok(conexion.last_insert_rowid())
}

/// El último cambio que se puede deshacer (de los últimos 7 días). Deshacer
/// un "deshacer" no vale: sería rehacer, y eso confunde más de lo que ayuda.
pub fn ultimo(conexion: &Connection, ahora: &Ahora) -> Resultado<Option<CambioGuardado>> {
    let desde = (ahora.momento - chrono::Duration::days(DIAS_DE_COPIAS)).to_rfc3339();
    Ok(conexion
        .query_row(
            "SELECT id, ruta, accion, copia, huella_despues, hecho_en FROM cambios_archivos
             WHERE deshecho = 0 AND accion != 'deshacer' AND hecho_en >= ?1
             ORDER BY id DESC LIMIT 1",
            [desde],
            |f| {
                Ok(CambioGuardado {
                    id: f.get(0)?,
                    ruta: f.get(1)?,
                    accion: AccionArchivo::desde(&f.get::<_, String>(2)?),
                    copia: f.get(3)?,
                    huella_despues: f.get(4)?,
                    hecho_en: f.get(5)?,
                })
            },
        )
        .optional()?)
}

pub fn marcar_deshecho(conexion: &Connection, id: i64) -> Resultado<()> {
    conexion.execute("UPDATE cambios_archivos SET deshecho = 1 WHERE id = ?1", [id])?;
    Ok(())
}

/// Saca los cambios viejos (más de 7 días, o más allá de los últimos 50) y
/// devuelve sus copias, para borrarlas del disco.
pub fn limpiar(conexion: &Connection, ahora: &Ahora) -> Resultado<Vec<String>> {
    let desde = (ahora.momento - chrono::Duration::days(DIAS_DE_COPIAS)).to_rfc3339();
    let condicion = "hecho_en < ?1 OR id NOT IN (SELECT id FROM cambios_archivos ORDER BY id DESC LIMIT ?2)";
    let copias: Vec<String> = conexion
        .prepare(&format!("SELECT copia FROM cambios_archivos WHERE copia IS NOT NULL AND ({condicion})"))?
        .query_map(params![desde, MAXIMO_DE_COPIAS], |f| f.get(0))?
        .collect::<Result<_, _>>()?;
    conexion.execute(&format!("DELETE FROM cambios_archivos WHERE {condicion}"), params![desde, MAXIMO_DE_COPIAS])?;
    Ok(copias)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn el_ultimo_que_se_puede_deshacer() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-20");
        assert!(ultimo(&base, &ahora).unwrap().is_none());
        anotar(&base, r"C:\a.txt", AccionArchivo::Cambiar, Some("c1"), Some("h1"), &ahora).unwrap();
        let id = anotar(&base, r"C:\b.txt", AccionArchivo::Crear, None, Some("h2"), &ahora).unwrap();
        let u = ultimo(&base, &ahora).unwrap().unwrap();
        assert_eq!((u.id, u.accion, u.copia.as_deref()), (id, AccionArchivo::Crear, None));
        marcar_deshecho(&base, id).unwrap();
        anotar(&base, r"C:\b.txt", AccionArchivo::Deshacer, None, None, &ahora).unwrap();
        // El "deshacer" no cuenta: queda el anterior.
        assert_eq!(ultimo(&base, &ahora).unwrap().unwrap().ruta, r"C:\a.txt");
        // A los 8 días ya no se puede.
        assert!(ultimo(&base, &Ahora::fijo("2026-11-28")).unwrap().is_none());
    }

    #[test]
    fn limpia_los_viejos_y_devuelve_sus_copias() {
        let base = db::en_memoria();
        anotar(&base, "a", AccionArchivo::Cambiar, Some("vieja"), None, &Ahora::fijo("2026-11-01")).unwrap();
        let hoy = Ahora::fijo("2026-11-20");
        for i in 0..MAXIMO_DE_COPIAS + 2 {
            anotar(&base, "b", AccionArchivo::Cambiar, Some(&format!("c{i}")), None, &hoy).unwrap();
        }
        let borradas = limpiar(&base, &hoy).unwrap();
        assert_eq!(borradas, ["vieja", "c0", "c1"]);
        let quedan: i64 = base.query_row("SELECT count(*) FROM cambios_archivos", [], |f| f.get(0)).unwrap();
        assert_eq!(quedan, MAXIMO_DE_COPIAS);
    }
}
