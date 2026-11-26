use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use super::{Ahora, automaticas, fecha, materias, opcional, texto_fecha};
use crate::dominio::momento::Momento;
use crate::dominio::recurrencia::{self, Serie};
use crate::dominio::texto::{Recurrencia, TipoEvento};
use crate::error::{Error, Resultado, invalido};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evento {
    pub id: i64,
    pub titulo: String,
    pub tipo: TipoEvento,
    /// ISO 8601 con zona, o fecha sola si es de todo el día.
    pub inicio: String,
    pub fin: Option<String>,
    pub todo_el_dia: bool,
    pub materia_id: Option<i64>,
    pub persona: Option<String>,
    pub notas: Option<String>,
    pub recurrencia: Recurrencia,
    pub repetir_hasta: Option<String>,
    /// Fechas salteadas de la serie ("2026-11-23").
    pub excepciones: Vec<String>,
    /// Vino de Google Calendar (el widget lo marca apenas).
    pub de_google: bool,
    /// Repetición de una serie de Google que Nova no sabe repetir: se edita
    /// desde Calendar.
    pub solo_lectura: bool,
}

/// Lo que manda la interfaz para crear o editar un evento.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosEvento {
    pub titulo: String,
    pub tipo: TipoEvento,
    pub inicio: String,
    pub fin: Option<String>,
    pub todo_el_dia: bool,
    pub materia_id: Option<i64>,
    pub persona: Option<String>,
    pub notas: Option<String>,
    pub recurrencia: Recurrencia,
    pub repetir_hasta: Option<String>,
}

/// Una repetición concreta de un evento, lista para mostrar.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ocurrencia {
    pub evento: Evento,
    /// Día en que empieza esta repetición.
    pub fecha: String,
    pub inicio: String,
    pub fin: Option<String>,
}

impl Evento {
    /// Arma la serie para calcular repeticiones. Los textos ya se validaron al
    /// guardar, así que acá un error sería un dato corrupto.
    pub fn con_serie<T>(&self, uso: impl FnOnce(&Serie) -> T) -> Resultado<T> {
        let corrupto = |e: String| Error::Sistema(format!("evento {} inválido: {e}", self.id));
        let excepciones: HashSet<NaiveDate> = self.excepciones.iter().filter_map(|f| fecha(f)).collect();
        let serie = Serie {
            inicio: Momento::parsear(&self.inicio).map_err(corrupto)?,
            fin: self.fin.as_deref().map(Momento::parsear).transpose().map_err(corrupto)?,
            recurrencia: self.recurrencia,
            repetir_hasta: self.repetir_hasta.as_deref().and_then(fecha),
            excepciones: &excepciones,
        };
        Ok(uso(&serie))
    }
}

/// Valida y normaliza lo que llega de la interfaz.
pub(crate) fn validar(conexion: &Connection, datos: DatosEvento) -> Resultado<DatosEvento> {
    let titulo = datos.titulo.trim().to_string();
    if titulo.is_empty() {
        return invalido("El evento necesita un título.");
    }
    if titulo.chars().count() > 200 {
        return invalido("El título es muy largo.");
    }

    let inicio = Momento::parsear(&datos.inicio).map_err(Error::Validacion)?;
    if inicio.es_fecha_sola() != datos.todo_el_dia {
        return invalido("Un evento de todo el día va con fecha sola; uno con hora, con fecha y hora.");
    }

    let fin = opcional(datos.fin);
    if let Some(texto) = &fin {
        let fin = Momento::parsear(texto).map_err(Error::Validacion)?;
        let mismo_tipo = fin.es_fecha_sola() == inicio.es_fecha_sola();
        let despues = match (inicio, fin) {
            (Momento::Fecha(a), Momento::Fecha(b)) => b >= a,
            (Momento::FechaHora(a), Momento::FechaHora(b)) => b > a,
            _ => false,
        };
        if !mismo_tipo || !despues {
            return invalido("El fin tiene que ser después del inicio.");
        }
    }

    // `repetir_hasta` solo tiene sentido si se repite.
    let repetir_hasta = match datos.recurrencia {
        Recurrencia::Ninguna => None,
        _ => opcional(datos.repetir_hasta),
    };
    if let Some(texto) = &repetir_hasta {
        let Some(tope) = fecha(texto).filter(|_| texto.len() == 10) else {
            return invalido("La fecha de \"hasta cuándo\" no es válida.");
        };
        if tope < inicio.fecha() {
            return invalido("La serie no puede terminar antes de empezar.");
        }
    }

    if let Some(id) = datos.materia_id
        && !materias::existe(conexion, id)?
    {
        return invalido("Esa materia no existe.");
    }

    Ok(DatosEvento {
        titulo,
        inicio: inicio.a_texto(),
        fin,
        persona: opcional(datos.persona),
        notas: opcional(datos.notas),
        repetir_hasta,
        ..datos
    })
}

const COLUMNAS: &str =
    "id, titulo, tipo, inicio, fin, todo_el_dia, materia_id, persona, notas, recurrencia, repetir_hasta, origen, solo_lectura";

/// El mensaje cuando querés tocar desde Nova algo que solo se cambia en Calendar.
pub const SE_EDITA_EN_GOOGLE: &str = "Ese evento se repite de una forma que Nova no sabe manejar: cambialo desde Google Calendar.";

pub(crate) fn desde_fila(fila: &Row) -> rusqlite::Result<Evento> {
    Ok(Evento {
        id: fila.get("id")?,
        titulo: fila.get("titulo")?,
        tipo: fila.get("tipo")?,
        inicio: fila.get("inicio")?,
        fin: fila.get("fin")?,
        todo_el_dia: fila.get("todo_el_dia")?,
        materia_id: fila.get("materia_id")?,
        persona: fila.get("persona")?,
        notas: fila.get("notas")?,
        recurrencia: fila.get("recurrencia")?,
        repetir_hasta: fila.get("repetir_hasta")?,
        excepciones: Vec::new(),
        de_google: fila.get::<_, String>("origen")? == "google",
        solo_lectura: fila.get("solo_lectura")?,
    })
}

/// Lee eventos con un filtro SQL y les agrega sus excepciones.
pub(crate) fn leer(conexion: &Connection, filtro: &str, parametros: impl rusqlite::Params) -> Resultado<Vec<Evento>> {
    let sql = format!("SELECT {COLUMNAS} FROM eventos {filtro} ORDER BY inicio, id");
    let mut eventos: Vec<Evento> = conexion
        .prepare(&sql)?
        .query_map(parametros, desde_fila)?
        .collect::<Result<_, _>>()?;

    if eventos.iter().any(|e| e.recurrencia != Recurrencia::Ninguna) {
        let mut por_evento: HashMap<i64, Vec<String>> = HashMap::new();
        let mut consulta = conexion.prepare("SELECT evento_id, fecha FROM eventos_excepciones ORDER BY fecha")?;
        for fila in consulta.query_map([], |f| Ok((f.get::<_, i64>(0)?, f.get::<_, String>(1)?)))? {
            let (id, fecha) = fila?;
            por_evento.entry(id).or_default().push(fecha);
        }
        for evento in &mut eventos {
            evento.excepciones = por_evento.remove(&evento.id).unwrap_or_default();
        }
    }
    Ok(eventos)
}

pub fn obtener(conexion: &Connection, id: i64) -> Resultado<Evento> {
    leer(conexion, "WHERE id = ?1", [id])?
        .pop()
        .ok_or(Error::NoEncontrado("ese evento"))
}

pub fn listar_todos(conexion: &Connection) -> Resultado<Vec<Evento>> {
    leer(conexion, "", [])
}

/// Todas las repeticiones que tocan `[desde, hasta]`, ordenadas por inicio.
pub fn ocurrencias(conexion: &Connection, desde: NaiveDate, hasta: NaiveDate) -> Resultado<Vec<Ocurrencia>> {
    // Prefiltro en SQL: las series se calculan siempre; los eventos comunes,
    // solo si su rango de días toca el pedido. `substr(..., 1, 10)` toma la
    // fecha de un texto ISO, con o sin hora.
    let candidatos = leer(
        conexion,
        "WHERE recurrencia != 'ninguna'
            OR (substr(inicio, 1, 10) <= ?2 AND substr(coalesce(fin, inicio), 1, 10) >= ?1)",
        params![texto_fecha(desde), texto_fecha(hasta)],
    )?;

    let mut lista = Vec::new();
    for evento in candidatos {
        let repeticiones = evento.con_serie(|serie| recurrencia::repeticiones(serie, desde, hasta))?;
        for r in repeticiones {
            lista.push(Ocurrencia {
                fecha: texto_fecha(r.fecha()),
                inicio: r.inicio.a_texto(),
                fin: r.fin.map(|f| f.a_texto()),
                evento: evento.clone(),
            });
        }
    }
    // Las de todo el día van primero en su día; después, por hora.
    lista.sort_by(|a, b| {
        (&a.fecha, !a.evento.todo_el_dia, &a.inicio).cmp(&(&b.fecha, !b.evento.todo_el_dia, &b.inicio))
    });
    Ok(lista)
}

/// Crea el evento y sus responsabilidades automáticas, todo o nada.
pub fn crear(conexion: &mut Connection, datos: DatosEvento, ahora: &Ahora) -> Resultado<Evento> {
    // Una transacción agrupa varias escrituras: si alguna falla, se deshacen
    // todas al salir de la función sin llamar a `commit`.
    let tx = conexion.transaction()?;
    let d = validar(&tx, datos)?;
    tx.execute(
        "INSERT INTO eventos (titulo, tipo, inicio, fin, todo_el_dia, materia_id, persona, notas,
                              recurrencia, repetir_hasta, creado_en, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            d.titulo, d.tipo, d.inicio, d.fin, d.todo_el_dia, d.materia_id, d.persona, d.notas,
            d.recurrencia, d.repetir_hasta, ahora.marca
        ],
    )?;
    let evento = obtener(&tx, tx.last_insert_rowid())?;
    automaticas::sincronizar_evento(&tx, &evento, ahora)?;
    tx.commit()?;
    Ok(evento)
}

/// Los de solo lectura no se tocan desde Nova (ni a mano ni con el asistente).
fn editable(conexion: &Connection, id: i64) -> Resultado<()> {
    if obtener(conexion, id)?.solo_lectura {
        return invalido(SE_EDITA_EN_GOOGLE);
    }
    Ok(())
}

/// Cambiar un evento desde Nova lo marca para subirlo a Google
/// (`cambiado_local = 1`). Lo que llega de Google entra por `repos::google`,
/// que no lo marca: si no, iría y volvería para siempre.
pub fn editar(conexion: &mut Connection, id: i64, datos: DatosEvento, ahora: &Ahora) -> Resultado<Evento> {
    let tx = conexion.transaction()?;
    editable(&tx, id)?;
    let d = validar(&tx, datos)?;
    let cambiadas = tx.execute(
        "UPDATE eventos SET titulo = ?1, tipo = ?2, inicio = ?3, fin = ?4, todo_el_dia = ?5,
                materia_id = ?6, persona = ?7, notas = ?8, recurrencia = ?9, repetir_hasta = ?10,
                actualizado_en = ?11, cambiado_local = 1
         WHERE id = ?12",
        params![
            d.titulo, d.tipo, d.inicio, d.fin, d.todo_el_dia, d.materia_id, d.persona, d.notas,
            d.recurrencia, d.repetir_hasta, ahora.marca, id
        ],
    )?;
    if cambiadas == 0 {
        return Err(Error::NoEncontrado("ese evento"));
    }
    let evento = obtener(&tx, id)?;
    automaticas::sincronizar_evento(&tx, &evento, ahora)?;
    tx.commit()?;
    Ok(evento)
}

/// Si el evento ya estaba en Google, queda anotado en `google_borrados` para
/// borrarlo allá cuando haya internet.
pub fn borrar(conexion: &mut Connection, id: i64, ahora: &Ahora) -> Resultado<()> {
    let tx = conexion.transaction()?;
    editable(&tx, id)?;
    let id_externo: Option<String> =
        tx.query_row("SELECT id_externo FROM eventos WHERE id = ?1", [id], |f| f.get(0))?;
    if let Some(id_externo) = id_externo {
        tx.execute(
            "INSERT OR REPLACE INTO google_borrados (id_externo, borrado_en) VALUES (?1, ?2)",
            params![id_externo, ahora.marca],
        )?;
    }
    borrar_con_repeticiones(&tx, id)?;
    tx.commit()?;
    Ok(())
}

/// Borra el evento y sus repeticiones movidas (`serie_id`), con sus
/// automáticas. En Google, borrar la serie ya se lleva sus repeticiones.
pub(crate) fn borrar_con_repeticiones(conexion: &Connection, id: i64) -> Resultado<()> {
    let hijos: Vec<i64> = conexion
        .prepare("SELECT id FROM eventos WHERE serie_id = ?1")?
        .query_map([id], |f| f.get(0))?
        .collect::<Result<_, _>>()?;
    for hijo in hijos.into_iter().chain([id]) {
        automaticas::al_borrar_evento(conexion, hijo)?;
        conexion.execute("DELETE FROM eventos WHERE id = ?1", [hijo])?;
    }
    Ok(())
}

/// Saltea (o vuelve a poner) una fecha de un evento recurrente.
pub fn cambiar_excepcion(
    conexion: &mut Connection,
    id: i64,
    dia: &str,
    saltear: bool,
    ahora: &Ahora,
) -> Resultado<Evento> {
    let Some(dia) = fecha(dia).filter(|_| dia.len() == 10) else {
        return invalido("Fecha inválida.");
    };
    let tx = conexion.transaction()?;
    let evento = obtener(&tx, id)?;
    if evento.solo_lectura {
        return invalido(SE_EDITA_EN_GOOGLE);
    }
    if evento.recurrencia == Recurrencia::Ninguna {
        return invalido("Solo se pueden saltear fechas de un evento que se repite.");
    }
    if saltear {
        tx.execute(
            "INSERT OR IGNORE INTO eventos_excepciones (evento_id, fecha) VALUES (?1, ?2)",
            params![id, texto_fecha(dia)],
        )?;
    } else {
        tx.execute(
            "DELETE FROM eventos_excepciones WHERE evento_id = ?1 AND fecha = ?2",
            params![id, texto_fecha(dia)],
        )?;
    }
    tx.execute(
        "UPDATE eventos SET cambiado_local = 1, actualizado_en = ?1 WHERE id = ?2",
        params![ahora.marca, id],
    )?;
    let evento = obtener(&tx, id)?;
    automaticas::sincronizar_evento(&tx, &evento, ahora)?;
    tx.commit()?;
    Ok(evento)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db;

    pub fn datos(titulo: &str, tipo: TipoEvento, inicio: &str) -> DatosEvento {
        DatosEvento {
            titulo: titulo.into(),
            tipo,
            inicio: inicio.into(),
            fin: None,
            todo_el_dia: inicio.len() == 10,
            materia_id: None,
            persona: None,
            notas: None,
            recurrencia: Recurrencia::Ninguna,
            repetir_hasta: None,
        }
    }

    fn d(texto: &str) -> NaiveDate {
        fecha(texto).unwrap()
    }

    #[test]
    fn crea_edita_y_borra() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let clase = crear(&mut base, datos(" Clase de Física ", TipoEvento::Clase, "2026-11-18T15:00:00-03:00"), &ahora).unwrap();
        assert_eq!(clase.titulo, "Clase de Física");
        assert!(!clase.todo_el_dia);

        let mut cambios = datos("Clase de Física I", TipoEvento::Clase, "2026-11-18T16:00:00-03:00");
        cambios.fin = Some("2026-11-18T18:00:00-03:00".into());
        let editada = editar(&mut base, clase.id, cambios, &ahora).unwrap();
        assert_eq!(editada.fin.as_deref(), Some("2026-11-18T18:00:00-03:00"));

        borrar(&mut base, clase.id, &ahora).unwrap();
        assert!(matches!(obtener(&base, clase.id), Err(Error::NoEncontrado(_))));
    }

    #[test]
    fn valida_fechas_y_titulo() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let mal = |d: DatosEvento, base: &mut Connection| matches!(crear(base, d, &ahora), Err(Error::Validacion(_)));

        assert!(mal(datos(" ", TipoEvento::Otro, "2026-11-18"), &mut base));
        assert!(mal(datos("x", TipoEvento::Otro, "mañana"), &mut base));

        // Dice "todo el día" pero trae hora
        let mut d1 = datos("x", TipoEvento::Otro, "2026-11-18T10:00:00-03:00");
        d1.todo_el_dia = true;
        assert!(mal(d1, &mut base));

        // Termina antes de empezar
        let mut d2 = datos("x", TipoEvento::Otro, "2026-11-18T10:00:00-03:00");
        d2.fin = Some("2026-11-18T09:00:00-03:00".into());
        assert!(mal(d2, &mut base));

        // Materia que no existe
        let mut d3 = datos("x", TipoEvento::Examen, "2026-11-18");
        d3.materia_id = Some(42);
        assert!(mal(d3, &mut base));

        // Serie que termina antes de empezar
        let mut d4 = datos("x", TipoEvento::Clase, "2026-11-18T10:00:00-03:00");
        d4.recurrencia = Recurrencia::Semanal;
        d4.repetir_hasta = Some("2026-11-01".into());
        assert!(mal(d4, &mut base));
    }

    #[test]
    fn ocurrencias_mezcla_comunes_y_series() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let mut clase = datos("Análisis", TipoEvento::Clase, "2026-08-03T15:00:00-03:00");
        clase.recurrencia = Recurrencia::Semanal;
        let clase = crear(&mut base, clase, &ahora).unwrap();
        crear(&mut base, datos("Parcial", TipoEvento::Examen, "2026-11-19T09:00:00-03:00"), &ahora).unwrap();
        crear(&mut base, datos("Viejo", TipoEvento::Otro, "2026-10-01"), &ahora).unwrap();
        let mut cumple = datos("Cumple de Juan", TipoEvento::Cumpleanos, "2000-11-23");
        cumple.recurrencia = Recurrencia::Anual;
        crear(&mut base, cumple, &ahora).unwrap();

        let lista = ocurrencias(&base, d("2026-11-16"), d("2026-11-23")).unwrap();
        let resumen: Vec<_> = lista.iter().map(|o| format!("{} {}", o.fecha, o.evento.titulo)).collect();
        assert_eq!(
            resumen,
            ["2026-11-16 Análisis", "2026-11-19 Parcial", "2026-11-23 Cumple de Juan", "2026-11-23 Análisis"]
        );

        // Saltear el lunes 23 y volver a ponerlo
        cambiar_excepcion(&mut base, clase.id, "2026-11-23", true, &ahora).unwrap();
        let lista = ocurrencias(&base, d("2026-11-23"), d("2026-11-23")).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].evento.titulo, "Cumple de Juan");
        let clase = cambiar_excepcion(&mut base, clase.id, "2026-11-23", false, &ahora).unwrap();
        assert!(clase.excepciones.is_empty());
    }

    #[test]
    fn no_se_saltean_fechas_de_eventos_comunes() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let parcial = crear(&mut base, datos("Parcial", TipoEvento::Examen, "2026-11-19"), &ahora).unwrap();
        assert!(cambiar_excepcion(&mut base, parcial.id, "2026-11-19", true, &ahora).is_err());
    }
}
