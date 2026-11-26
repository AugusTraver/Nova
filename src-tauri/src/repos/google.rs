//! Lo que la sincronización con Google Calendar guarda y lee de la base
//! (v0.7.2). Las decisiones (quién gana, cómo se traduce) están en
//! `dominio::google`; acá solo se escribe.
//!
//! La regla principal: lo que llega de Google entra por `guardar_de_google`,
//! que NO marca `cambiado_local`. Lo que cambiás en Nova pasa por
//! `repos::eventos`, que sí lo marca, y la tarea de fondo lo sube.

use rusqlite::{Connection, OptionalExtension, params};

use super::eventos::{self, DatosEvento, Evento};
use super::{Ahora, automaticas};
use crate::error::Resultado;

/// Cómo está un evento respecto de Google.
#[derive(Debug, Clone, PartialEq)]
pub struct Vinculo {
    pub id: i64,
    pub id_externo: Option<String>,
    pub etag: Option<String>,
    /// El `updated` de Google la última vez que lo vimos.
    pub actualizado_google: Option<String>,
    /// Cuándo cambió en Nova.
    pub actualizado_en: String,
    pub cambiado_local: bool,
    /// Si es una repetición movida: la serie y la fecha original.
    pub serie_id: Option<i64>,
    pub serie_fecha: Option<String>,
}

/// Un evento como llega de Google, ya traducido (ver `dominio::google`).
#[derive(Debug, Clone)]
pub struct DeGoogle {
    pub id_externo: String,
    pub etag: Option<String>,
    pub actualizado: Option<String>,
    pub datos: DatosEvento,
    /// Fechas salteadas de la serie ("2026-11-23").
    pub excepciones: Vec<String>,
    /// Repetición suelta de una serie que Nova no sabe repetir.
    pub solo_lectura: bool,
    /// Para las de solo lectura: el id de la serie en Google.
    pub serie_externa: Option<String>,
    /// Repetición movida de una serie que Nova sí sabe repetir: el id de la
    /// serie en Google y la fecha original de esa repetición.
    pub reemplaza: Option<(String, String)>,
}

fn vinculo_desde_fila(f: &rusqlite::Row) -> rusqlite::Result<Vinculo> {
    Ok(Vinculo {
        id: f.get("id")?,
        id_externo: f.get("id_externo")?,
        etag: f.get("etag_google")?,
        actualizado_google: f.get("actualizado_google")?,
        actualizado_en: f.get("actualizado_en")?,
        cambiado_local: f.get("cambiado_local")?,
        serie_id: f.get("serie_id")?,
        serie_fecha: f.get("serie_fecha")?,
    })
}

const COLUMNAS_VINCULO: &str =
    "id, id_externo, etag_google, actualizado_google, actualizado_en, cambiado_local, serie_id, serie_fecha";

pub fn vinculo(conexion: &Connection, id: i64) -> Resultado<Option<Vinculo>> {
    let sql = format!("SELECT {COLUMNAS_VINCULO} FROM eventos WHERE id = ?1");
    Ok(conexion.query_row(&sql, [id], vinculo_desde_fila).optional()?)
}

pub fn vinculo_por_externo(conexion: &Connection, id_externo: &str) -> Resultado<Option<Vinculo>> {
    let sql = format!("SELECT {COLUMNAS_VINCULO} FROM eventos WHERE id_externo = ?1");
    Ok(conexion.query_row(&sql, [id_externo], vinculo_desde_fila).optional()?)
}

/// Todos los eventos con su vínculo (para la primera sincronización y para
/// ver qué desapareció de Google).
pub fn todos_con_vinculo(conexion: &Connection) -> Resultado<Vec<(Evento, Vinculo)>> {
    emparejar(conexion, eventos::listar_todos(conexion)?)
}

/// Lo que cambió en Nova y falta subir. Los de solo lectura nunca se suben.
pub fn pendientes(conexion: &Connection) -> Resultado<Vec<(Evento, Vinculo)>> {
    let lista = eventos::leer(conexion, "WHERE cambiado_local = 1 AND solo_lectura = 0", [])?;
    emparejar(conexion, lista)
}

fn emparejar(conexion: &Connection, lista: Vec<Evento>) -> Resultado<Vec<(Evento, Vinculo)>> {
    let mut salida = Vec::with_capacity(lista.len());
    for evento in lista {
        if let Some(v) = vinculo(conexion, evento.id)? {
            salida.push((evento, v));
        }
    }
    Ok(salida)
}

/// Google confirmó que lo tiene. `leido_en` es el `actualizado_en` que tenía
/// el evento cuando se mandó: si lo cambiaste mientras tanto, sigue
/// pendiente y se sube otra vez en la próxima vuelta.
pub fn marcar_subido(
    conexion: &Connection,
    id: i64,
    id_externo: &str,
    etag: Option<&str>,
    actualizado_google: Option<&str>,
    leido_en: &str,
) -> Resultado<()> {
    conexion.execute(
        "UPDATE eventos SET id_externo = ?1, etag_google = ?2, actualizado_google = ?3,
                cambiado_local = CASE WHEN actualizado_en = ?4 THEN 0 ELSE 1 END
         WHERE id = ?5",
        params![id_externo, etag, actualizado_google, leido_en, id],
    )?;
    Ok(())
}

/// Vuelve a marcar un evento para subir (por ejemplo, si Google dice que no
/// lo tiene más pero acá cambió: gana la edición).
pub fn marcar_pendiente(conexion: &Connection, id: i64, sin_vinculo: bool) -> Resultado<()> {
    if sin_vinculo {
        conexion.execute(
            "UPDATE eventos SET cambiado_local = 1, id_externo = NULL, etag_google = NULL,
                    actualizado_google = NULL
             WHERE id = ?1",
            [id],
        )?;
    } else {
        conexion.execute("UPDATE eventos SET cambiado_local = 1 WHERE id = ?1", [id])?;
    }
    Ok(())
}

/// La primera vez: un evento de Nova y uno de Google que son el mismo. Se
/// vinculan y queda pendiente, así Google recibe el tipo y la materia de Nova.
pub fn juntar(conexion: &Connection, id: i64, id_externo: &str, etag: Option<&str>, actualizado: Option<&str>) -> Resultado<()> {
    conexion.execute(
        "UPDATE eventos SET id_externo = ?1, etag_google = ?2, actualizado_google = ?3, cambiado_local = 1
         WHERE id = ?4",
        params![id_externo, etag, actualizado, id],
    )?;
    Ok(())
}

pub fn borrados(conexion: &Connection) -> Resultado<Vec<String>> {
    Ok(conexion
        .prepare("SELECT id_externo FROM google_borrados ORDER BY borrado_en")?
        .query_map([], |f| f.get(0))?
        .collect::<Result<_, _>>()?)
}

pub fn quitar_borrado(conexion: &Connection, id_externo: &str) -> Resultado<()> {
    conexion.execute("DELETE FROM google_borrados WHERE id_externo = ?1", [id_externo])?;
    Ok(())
}

/// ¿Lo borraste en Nova y todavía no se avisó a Google? Entonces lo que
/// llegue de Google con ese id no tiene que volver a aparecer.
pub fn esta_borrado(conexion: &Connection, id_externo: &str) -> Resultado<bool> {
    Ok(conexion
        .query_row("SELECT 1 FROM google_borrados WHERE id_externo = ?1", [id_externo], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Crea o actualiza un evento con lo que llegó de Google, sin marcarlo para
/// subir. Sus responsabilidades automáticas se calculan igual que con uno de
/// Nova. Devuelve el id en Nova.
pub fn guardar_de_google(conexion: &mut Connection, nuevo: DeGoogle, ahora: &Ahora) -> Resultado<i64> {
    let tx = conexion.transaction()?;
    let d = eventos::validar(&tx, nuevo.datos)?;

    let serie = match &nuevo.reemplaza {
        Some((serie_externa, fecha)) => {
            vinculo_por_externo(&tx, serie_externa)?.map(|v| (v.id, fecha.clone()))
        }
        None => None,
    };
    let (serie_id, serie_fecha) = serie.clone().unzip();

    let existente = vinculo_por_externo(&tx, &nuevo.id_externo)?;
    let id = match existente {
        Some(v) => {
            tx.execute(
                "UPDATE eventos SET titulo = ?1, tipo = ?2, inicio = ?3, fin = ?4, todo_el_dia = ?5,
                        materia_id = ?6, persona = ?7, notas = ?8, recurrencia = ?9, repetir_hasta = ?10,
                        etag_google = ?11, actualizado_google = ?12, solo_lectura = ?13, serie_externa = ?14,
                        serie_id = ?15, serie_fecha = ?16, cambiado_local = 0, actualizado_en = ?17
                 WHERE id = ?18",
                params![
                    d.titulo, d.tipo, d.inicio, d.fin, d.todo_el_dia, d.materia_id, d.persona, d.notas,
                    d.recurrencia, d.repetir_hasta, nuevo.etag, nuevo.actualizado, nuevo.solo_lectura,
                    nuevo.serie_externa, serie_id, serie_fecha, ahora.marca, v.id
                ],
            )?;
            v.id
        }
        None => {
            tx.execute(
                "INSERT INTO eventos (titulo, tipo, inicio, fin, todo_el_dia, materia_id, persona, notas,
                                      recurrencia, repetir_hasta, origen, id_externo, etag_google,
                                      actualizado_google, solo_lectura, serie_externa, serie_id, serie_fecha,
                                      cambiado_local, creado_en, actualizado_en)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'google', ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                         0, ?18, ?18)",
                params![
                    d.titulo, d.tipo, d.inicio, d.fin, d.todo_el_dia, d.materia_id, d.persona, d.notas,
                    d.recurrencia, d.repetir_hasta, nuevo.id_externo, nuevo.etag, nuevo.actualizado,
                    nuevo.solo_lectura, nuevo.serie_externa, serie_id, serie_fecha, ahora.marca
                ],
            )?;
            tx.last_insert_rowid()
        }
    };

    tx.execute("DELETE FROM eventos_excepciones WHERE evento_id = ?1", [id])?;
    for fecha in &nuevo.excepciones {
        tx.execute(
            "INSERT OR IGNORE INTO eventos_excepciones (evento_id, fecha) VALUES (?1, ?2)",
            params![id, fecha],
        )?;
    }
    // La repetición movida reemplaza su fecha en la serie.
    if let Some((serie_id, fecha)) = &serie {
        tx.execute(
            "INSERT OR IGNORE INTO eventos_excepciones (evento_id, fecha) VALUES (?1, ?2)",
            params![serie_id, fecha],
        )?;
    }

    let evento = eventos::obtener(&tx, id)?;
    automaticas::sincronizar_evento(&tx, &evento, ahora)?;
    tx.commit()?;
    Ok(id)
}

/// Google dice que se borró: se va de Nova sin avisarle a Google, con sus
/// repeticiones movidas y las sueltas de solo lectura de esa serie.
pub fn borrar_de_google(conexion: &mut Connection, id_externo: &str) -> Resultado<bool> {
    let tx = conexion.transaction()?;
    let mut borro = false;
    if let Some(v) = vinculo_por_externo(&tx, id_externo)? {
        eventos::borrar_con_repeticiones(&tx, v.id)?;
        borro = true;
    }
    borro |= borrar_sueltas(&tx, id_externo)? > 0;
    tx.commit()?;
    Ok(borro)
}

/// Saca una repetición de su serie: si estaba movida (evento suelto que la
/// reemplaza), se va ese evento; la fecha queda salteada en la serie.
pub fn saltear_de_google(conexion: &mut Connection, serie_externa: &str, fecha: &str, ahora: &Ahora) -> Resultado<()> {
    let tx = conexion.transaction()?;
    if let Some(serie) = vinculo_por_externo(&tx, serie_externa)? {
        tx.execute(
            "INSERT OR IGNORE INTO eventos_excepciones (evento_id, fecha) VALUES (?1, ?2)",
            params![serie.id, fecha],
        )?;
        let movidas: Vec<i64> = tx
            .prepare("SELECT id FROM eventos WHERE serie_id = ?1 AND serie_fecha = ?2")?
            .query_map(params![serie.id, fecha], |f| f.get(0))?
            .collect::<Result<_, _>>()?;
        for id in movidas {
            eventos::borrar_con_repeticiones(&tx, id)?;
        }
        let evento = eventos::obtener(&tx, serie.id)?;
        automaticas::sincronizar_evento(&tx, &evento, ahora)?;
    }
    tx.commit()?;
    Ok(())
}

/// Las repeticiones sueltas (de solo lectura) de una serie de Google. Se
/// borran antes de volver a traerlas, cuando la serie cambia.
pub fn borrar_sueltas(conexion: &Connection, serie_externa: &str) -> Resultado<usize> {
    let ids: Vec<i64> = conexion
        .prepare("SELECT id FROM eventos WHERE solo_lectura = 1 AND serie_externa = ?1")?
        .query_map([serie_externa], |f| f.get(0))?
        .collect::<Result<_, _>>()?;
    for id in &ids {
        automaticas::al_borrar_evento(conexion, *id)?;
        conexion.execute("DELETE FROM eventos WHERE id = ?1", [id])?;
    }
    Ok(ids.len())
}

/// Las series de solo lectura que hay (para renovar sus repeticiones).
pub fn series_sueltas(conexion: &Connection) -> Resultado<Vec<String>> {
    Ok(conexion
        .prepare("SELECT DISTINCT serie_externa FROM eventos WHERE solo_lectura = 1 AND serie_externa IS NOT NULL")?
        .query_map([], |f| f.get(0))?
        .collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::dominio::texto::{Recurrencia, TipoEvento};
    use crate::repos::eventos::tests::datos;
    use crate::repos::responsabilidades;

    fn de_google(id: &str, titulo: &str, tipo: TipoEvento, inicio: &str) -> DeGoogle {
        DeGoogle {
            id_externo: id.into(),
            etag: Some("\"1\"".into()),
            actualizado: Some("2026-11-17T12:00:00.000Z".into()),
            datos: datos(titulo, tipo, inicio),
            excepciones: Vec::new(),
            solo_lectura: false,
            serie_externa: None,
            reemplaza: None,
        }
    }

    #[test]
    fn lo_de_nova_queda_pendiente_y_lo_de_google_no() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let local = eventos::crear(&mut base, datos("Clase", TipoEvento::Clase, "2026-11-18T15:00:00-03:00"), &ahora).unwrap();
        let id = guardar_de_google(&mut base, de_google("g1", "Turno médico", TipoEvento::Otro, "2026-11-19"), &ahora).unwrap();

        let pendientes: Vec<i64> = pendientes(&base).unwrap().into_iter().map(|(e, _)| e.id).collect();
        assert_eq!(pendientes, [local.id]);
        let de_google = eventos::obtener(&base, id).unwrap();
        assert!(de_google.de_google);
        assert!(!local.de_google);
    }

    #[test]
    fn un_examen_de_google_trae_sus_automaticas() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        guardar_de_google(&mut base, de_google("g1", "Parcial de Análisis", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        let lista = responsabilidades::listar(&base).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].fecha_limite.as_deref(), Some("2026-11-19"));

        // Si llega otra vez cambiado, se actualiza (sin duplicar).
        let mut cambiado = de_google("g1", "Parcial de Análisis", TipoEvento::Examen, "2026-11-24");
        cambiado.etag = Some("\"2\"".into());
        guardar_de_google(&mut base, cambiado, &ahora).unwrap();
        assert_eq!(eventos::listar_todos(&base).unwrap().len(), 1);
        let lista = responsabilidades::listar(&base).unwrap();
        assert_eq!(lista[0].fecha_limite.as_deref(), Some("2026-11-23"));
    }

    #[test]
    fn subir_no_pierde_lo_que_cambiaste_en_el_medio() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let e = eventos::crear(&mut base, datos("Clase", TipoEvento::Clase, "2026-11-18"), &ahora).unwrap();
        let leido = vinculo(&base, e.id).unwrap().unwrap().actualizado_en;

        // Mientras se subía, lo editaste.
        eventos::editar(&mut base, e.id, datos("Clase 2", TipoEvento::Clase, "2026-11-18"), &Ahora::fijo_a_las("2026-11-17", "12:05")).unwrap();
        marcar_subido(&base, e.id, "g9", Some("\"1\""), None, &leido).unwrap();
        let v = vinculo(&base, e.id).unwrap().unwrap();
        assert_eq!(v.id_externo.as_deref(), Some("g9"));
        assert!(v.cambiado_local, "tiene que volver a subirse");

        let leido = v.actualizado_en.clone();
        marcar_subido(&base, e.id, "g9", Some("\"2\""), None, &leido).unwrap();
        assert!(!vinculo(&base, e.id).unwrap().unwrap().cambiado_local);
    }

    #[test]
    fn borrar_en_nova_deja_anotado_para_google() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let id = guardar_de_google(&mut base, de_google("g1", "Turno", TipoEvento::Otro, "2026-11-19"), &ahora).unwrap();
        let local = eventos::crear(&mut base, datos("Sin subir", TipoEvento::Otro, "2026-11-19"), &ahora).unwrap();
        eventos::borrar(&mut base, id, &ahora).unwrap();
        eventos::borrar(&mut base, local.id, &ahora).unwrap();
        assert_eq!(borrados(&base).unwrap(), ["g1"]);
        assert!(esta_borrado(&base, "g1").unwrap());
        quitar_borrado(&base, "g1").unwrap();
        assert!(borrados(&base).unwrap().is_empty());
    }

    #[test]
    fn borrar_desde_google_no_deja_nada_para_subir() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        guardar_de_google(&mut base, de_google("g1", "Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        assert!(borrar_de_google(&mut base, "g1").unwrap());
        assert!(eventos::listar_todos(&base).unwrap().is_empty());
        assert!(borrados(&base).unwrap().is_empty());
        // La automática intacta se fue con el evento.
        assert!(responsabilidades::listar(&base).unwrap().is_empty());
        assert!(!borrar_de_google(&mut base, "no-existe").unwrap());
    }

    #[test]
    fn los_de_solo_lectura_no_se_tocan_desde_nova() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let mut suelta = de_google("serie_20261118", "Gimnasio", TipoEvento::Otro, "2026-11-18T08:00:00-03:00");
        suelta.solo_lectura = true;
        suelta.serie_externa = Some("serie".into());
        let id = guardar_de_google(&mut base, suelta, &ahora).unwrap();

        assert!(eventos::editar(&mut base, id, datos("x", TipoEvento::Otro, "2026-11-18"), &ahora).is_err());
        assert!(eventos::borrar(&mut base, id, &ahora).is_err());
        assert!(pendientes(&base).unwrap().is_empty());
        assert_eq!(series_sueltas(&base).unwrap(), ["serie"]);

        assert_eq!(borrar_sueltas(&base, "serie").unwrap(), 1);
        assert!(eventos::listar_todos(&base).unwrap().is_empty());
    }

    #[test]
    fn una_repeticion_movida_reemplaza_su_fecha() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let mut serie = de_google("clase", "Análisis", TipoEvento::Clase, "2026-11-02T15:00:00-03:00");
        serie.datos.recurrencia = Recurrencia::Semanal;
        let serie_id = guardar_de_google(&mut base, serie, &ahora).unwrap();

        let mut movida = de_google("clase_20261123T180000Z", "Análisis", TipoEvento::Clase, "2026-11-24T15:00:00-03:00");
        movida.reemplaza = Some(("clase".into(), "2026-11-23".into()));
        let movida_id = guardar_de_google(&mut base, movida, &ahora).unwrap();

        assert_eq!(eventos::obtener(&base, serie_id).unwrap().excepciones, ["2026-11-23"]);
        let v = vinculo(&base, movida_id).unwrap().unwrap();
        assert_eq!((v.serie_id, v.serie_fecha.as_deref()), (Some(serie_id), Some("2026-11-23")));

        // Cancelarla en Google: se va la movida y la fecha queda salteada.
        saltear_de_google(&mut base, "clase", "2026-11-23", &ahora).unwrap();
        assert!(eventos::obtener(&base, movida_id).is_err());
        assert_eq!(eventos::obtener(&base, serie_id).unwrap().excepciones, ["2026-11-23"]);

        // Borrar la serie en Nova se lleva sus repeticiones movidas.
        let otra = de_google("clase_20261130T180000Z", "Análisis", TipoEvento::Clase, "2026-12-01T15:00:00-03:00");
        let otra = DeGoogle { reemplaza: Some(("clase".into(), "2026-11-30".into())), ..otra };
        let otra_id = guardar_de_google(&mut base, otra, &ahora).unwrap();
        eventos::borrar(&mut base, serie_id, &ahora).unwrap();
        assert!(eventos::obtener(&base, otra_id).is_err());
        assert_eq!(borrados(&base).unwrap(), ["clase"]);
    }

    #[test]
    fn juntar_vincula_y_deja_pendiente() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let e = eventos::crear(&mut base, datos("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        juntar(&base, e.id, "g1", Some("\"1\""), None).unwrap();
        let v = vinculo_por_externo(&base, "g1").unwrap().unwrap();
        assert_eq!(v.id, e.id);
        assert!(v.cambiado_local);
    }
}
