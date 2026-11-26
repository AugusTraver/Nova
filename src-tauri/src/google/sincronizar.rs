//! Una vuelta de sincronización con Google Calendar: primero se trae lo que
//! cambió allá, después se sube lo que cambió acá.
//!
//! - La primera vez (o si Google pide empezar de nuevo) se trae todo lo que
//!   no terminó hace más de 30 días y se juntan los eventos que ya estaban
//!   de los dos lados (mismo título y mismo inicio).
//! - Después, cada vuelta pide con `syncToken` solo lo que cambió.
//! - Lo que cambiaste en Nova (`cambiado_local`) y lo que borraste
//!   (`google_borrados`) se sube al final.
//!
//! El candado de la base se toma y se suelta en cada paso, nunca mientras se
//! espera a Google (regla de `fondo.rs`).

use std::collections::{HashMap, HashSet};

use chrono::{Duration, FixedOffset, NaiveDate, SecondsFormat};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::calendario::{ApiCalendar, Falla, Lista};
use crate::db::Db;
use crate::dominio::google::{
    self as logica, Accion, EnNova, EventoGoogle, Llegada, MateriaConocida, ParaSubir, Traducido, ZONA_POR_DEFECTO,
};
use crate::error::Resultado;
use crate::repos::eventos::{self, DatosEvento, Evento};
use crate::repos::google::{self as repo, DeGoogle, Vinculo};
use crate::repos::{Ahora, cache, materias};

/// Clave en `cache_externa`: el estado de la sincronización y su último error.
pub const CLAVE: &str = "google:calendar";

/// Cuántos días para atrás se trae y se sube la primera vez.
pub const DIAS_PARA_ATRAS: i64 = 30;
/// Hasta cuándo se guardan las repeticiones sueltas de una serie rara.
pub const DIAS_DE_SUELTAS: i64 = 90;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoCalendar {
    /// Para pedir solo lo que cambió. Sin él, la próxima vuelta trae todo.
    pub sync_token: Option<String>,
    /// La zona del calendario principal.
    pub zona: Option<String>,
    /// El día en que se renovaron por última vez las repeticiones sueltas.
    pub sueltas_del: Option<String>,
    /// Lo que hizo la primera vez (se muestra en Ajustes).
    pub primera_vez: Option<Resumen>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resumen {
    pub traidos: u32,
    pub subidos: u32,
    pub juntados: u32,
    pub borrados: u32,
}

impl Resumen {
    /// ¿Cambió algo en Nova? (para avisarle al widget).
    pub fn cambio_algo_aca(&self) -> bool {
        self.traidos > 0 || self.juntados > 0 || self.borrados > 0
    }
}

fn leer_estado(db: &Db) -> Resultado<EstadoCalendar> {
    Ok(cache::leer::<EstadoCalendar>(&db.conexion(), CLAVE)?.datos.unwrap_or_default())
}

fn guardar_estado(db: &Db, estado: &EstadoCalendar, ahora: &Ahora, proxima: Duration) -> Resultado<()> {
    cache::guardar(&db.conexion(), CLAVE, estado, ahora.momento + proxima, ahora)
}

/// Olvida el `syncToken` (al desconectar): la próxima conexión trae todo y
/// junta por id, sin duplicar.
pub fn olvidar(conexion: &Connection) -> Resultado<()> {
    cache::borrar(conexion, CLAVE)
}

fn rfc3339(fecha: NaiveDate, zona: FixedOffset) -> String {
    fecha
        .and_hms_opt(0, 0, 0)
        .and_then(|n| n.and_local_timezone(zona).single())
        .map(|f| f.to_rfc3339_opts(SecondsFormat::Secs, false))
        .unwrap_or_default()
}

fn materias_conocidas(conexion: &Connection) -> Resultado<Vec<MateriaConocida>> {
    Ok(materias::listar(conexion)?.into_iter().map(|m| MateriaConocida { id: m.id, nombre: m.nombre }).collect())
}

fn datos_de(t: Traducido) -> (DatosEvento, Vec<String>) {
    (
        DatosEvento {
            titulo: t.titulo,
            tipo: t.tipo,
            inicio: t.inicio,
            fin: t.fin,
            todo_el_dia: t.todo_el_dia,
            materia_id: t.materia_id,
            persona: t.persona,
            notas: t.notas,
            recurrencia: t.recurrencia,
            repetir_hasta: t.repetir_hasta,
        },
        t.excepciones,
    )
}

fn de_google(ev: &EventoGoogle, t: Traducido) -> DeGoogle {
    let (datos, excepciones) = datos_de(t);
    DeGoogle {
        id_externo: ev.id.clone(),
        etag: ev.etag.clone(),
        actualizado: ev.updated.clone(),
        datos,
        excepciones,
        solo_lectura: false,
        serie_externa: None,
        reemplaza: None,
    }
}

/// Una vuelta completa. Devuelve qué hizo; los errores de Google se
/// devuelven como `Falla` para que la tarea de fondo los anote.
pub async fn sincronizar(db: &Db, api: &impl ApiCalendar, ahora: &Ahora) -> Result<Resumen, Falla> {
    let mut estado = leer_estado(db).map_err(base)?;
    let zona_local = *ahora.momento.offset();
    let desde = ahora.hoy - Duration::days(DIAS_PARA_ATRAS);

    let mut resumen = Resumen::default();
    let mut raras: HashSet<String> = HashSet::new();

    let primera = estado.sync_token.is_none();
    let lista = match estado.sync_token.as_deref() {
        Some(token) => match api.listar(Some(token), None).await {
            Err(Falla::SyncVencido) => None,
            otro => Some(otro?),
        },
        None => None,
    };
    let (lista, completa) = match lista {
        Some(l) => (l, false),
        None => (api.listar(None, Some(&rfc3339(desde, zona_local))).await?, true),
    };
    if let Some(zona) = &lista.zona {
        estado.zona = Some(zona.clone());
    }

    {
        let mut conexion = db.conexion();
        // Los que se juntan no se procesan: queda lo de Nova y se sube.
        let juntados = if completa { juntar_iguales(&mut conexion, &lista, zona_local, desde).map_err(base)? } else { HashSet::new() };
        resumen.juntados = juntados.len() as u32;
        let resto: Vec<EventoGoogle> = lista.eventos.iter().filter(|e| !juntados.contains(&e.id)).cloned().collect();
        procesar(&mut conexion, &resto, zona_local, ahora, &mut resumen, &mut raras).map_err(base)?;
        if completa {
            resumen.borrados += quitar_desaparecidos(&mut conexion, &lista, desde).map_err(base)?;
        }
    }

    // Las series raras: las que cambiaron ahora y, una vez por día, todas
    // (para que la ventana de 90 días avance).
    let hoy = ahora.hoy.format("%Y-%m-%d").to_string();
    if estado.sueltas_del.as_deref() != Some(&hoy) {
        raras.extend(repo::series_sueltas(&db.conexion()).map_err(base)?);
        estado.sueltas_del = Some(hoy);
    }
    for serie in raras {
        resumen.traidos += traer_sueltas(db, api, &serie, zona_local, ahora).await?;
    }

    resumen.subidos = subir(db, api, estado.zona.as_deref().unwrap_or(ZONA_POR_DEFECTO), zona_local, desde, ahora, &mut resumen).await?;

    estado.sync_token = lista.sync_token.or(estado.sync_token);
    if primera {
        estado.primera_vez = Some(resumen.clone());
    }
    guardar_estado(db, &estado, ahora, Duration::minutes(5)).map_err(base)?;
    Ok(resumen)
}

fn base(e: crate::error::Error) -> Falla {
    Falla::Otra(e.to_string())
}

/// La primera vez: si un evento de Google y uno de Nova sin subir son el
/// mismo, se vinculan en vez de duplicarse.
fn juntar_iguales(conexion: &mut Connection, lista: &Lista, zona: FixedOffset, desde: NaiveDate) -> Resultado<HashSet<String>> {
    let mut sueltos: HashMap<String, i64> = HashMap::new();
    for (evento, vinculo) in repo::todos_con_vinculo(conexion)? {
        if vinculo.id_externo.is_none()
            && logica::vigente(&evento.inicio, evento.fin.as_deref(), evento.recurrencia, evento.repetir_hasta.as_deref(), desde)
            && let Some(clave) = logica::clave_para_juntar(&evento.titulo, &evento.inicio)
        {
            sueltos.entry(clave).or_insert(evento.id);
        }
    }
    let mut juntados = HashSet::new();
    for ev in &lista.eventos {
        if ev.cancelado() || ev.recurring_event_id.is_some() || repo::vinculo_por_externo(conexion, &ev.id)?.is_some() {
            continue;
        }
        let Some(t) = logica::traducir_datos(ev, zona, &[]) else { continue };
        if let Some(clave) = logica::clave_para_juntar(&t.titulo, &t.inicio)
            && let Some(id) = sueltos.remove(&clave)
        {
            repo::juntar(conexion, id, &ev.id, ev.etag.as_deref(), ev.updated.as_deref())?;
            juntados.insert(ev.id.clone());
        }
    }
    Ok(juntados)
}

/// Lo que llegó de Google, evento por evento. Primero las series y los
/// sueltos; después las repeticiones movidas o borradas (necesitan su serie).
fn procesar(
    conexion: &mut Connection,
    eventos: &[EventoGoogle],
    zona: FixedOffset,
    ahora: &Ahora,
    resumen: &mut Resumen,
    raras: &mut HashSet<String>,
) -> Resultado<()> {
    let materias = materias_conocidas(conexion)?;
    let (repeticiones, resto): (Vec<&EventoGoogle>, Vec<&EventoGoogle>) =
        eventos.iter().partition(|e| e.recurring_event_id.is_some());
    let sueltas: HashSet<String> = repo::series_sueltas(conexion)?.into_iter().collect();

    for ev in resto.into_iter().chain(repeticiones) {
        let vinculo = repo::vinculo_por_externo(conexion, &ev.id)?;
        // Lo que subimos nosotros vuelve en la próxima lista: si el etag es el
        // mismo, ya está.
        if let Some(v) = &vinculo
            && !v.cambiado_local
            && v.etag.is_some()
            && v.etag == ev.etag
        {
            continue;
        }
        let borrado_en: Option<String> = if repo::esta_borrado(conexion, &ev.id)? {
            conexion.query_row("SELECT borrado_en FROM google_borrados WHERE id_externo = ?1", [&ev.id], |f| f.get(0))?
        } else {
            None
        };
        let en_nova = match (&borrado_en, &vinculo) {
            (Some(b), _) => EnNova::Borrado(b),
            (None, Some(v)) if v.cambiado_local => EnNova::Cambiado(&v.actualizado_en),
            (None, Some(_)) => EnNova::Igual,
            (None, None) => EnNova::NoEsta,
        };

        let llegada = logica::traducir(ev, zona, &materias);
        // Las repeticiones de una serie rara se renuevan trayendo la serie.
        if let Llegada::Movida { serie, .. } | Llegada::Cancelada { serie, .. } = &llegada
            && (sueltas.contains(serie) || raras.contains(serie))
        {
            raras.insert(serie.clone());
            continue;
        }

        // Una repetición borrada se decide por su serie, no por su propio id
        // (que Nova casi nunca tiene).
        if let Llegada::Cancelada { serie, fecha } = &llegada {
            if repo::vinculo_por_externo(conexion, serie)?.is_some() {
                repo::saltear_de_google(conexion, serie, fecha, ahora)?;
                resumen.borrados += 1;
            }
            continue;
        }

        let accion = logica::decidir(ev.cancelado(), en_nova, ev.updated.as_deref());
        match (llegada, accion) {
            (_, Accion::Nada | Accion::Ignorar) => {}
            (_, Accion::OlvidarBorrado) => repo::quitar_borrado(conexion, &ev.id)?,
            (_, Accion::Borrar) => {
                if repo::borrar_de_google(conexion, &ev.id)? {
                    resumen.borrados += 1;
                }
            }
            (_, Accion::Recrear) => {
                if let Some(v) = &vinculo {
                    repo::marcar_pendiente(conexion, v.id, true)?;
                }
            }
            (Llegada::SerieRara, _) => {
                // Si antes era una serie común, se va: vuelven sus repeticiones sueltas.
                if vinculo.is_some() {
                    repo::borrar_de_google(conexion, &ev.id)?;
                }
                repo::quitar_borrado(conexion, &ev.id)?;
                raras.insert(ev.id.clone());
            }
            (Llegada::Evento(t), accion) => {
                if accion == Accion::Restaurar {
                    repo::quitar_borrado(conexion, &ev.id)?;
                }
                // Si era una serie rara y ahora Nova la sabe repetir.
                repo::borrar_sueltas(conexion, &ev.id)?;
                if guardar(conexion, de_google(ev, t), ahora) {
                    resumen.traidos += 1;
                }
            }
            (Llegada::Movida { serie, fecha, evento }, accion) => {
                if accion == Accion::Restaurar {
                    repo::quitar_borrado(conexion, &ev.id)?;
                }
                let nuevo = DeGoogle { reemplaza: Some((serie, fecha)), ..de_google(ev, evento) };
                if guardar(conexion, nuevo, ahora) {
                    resumen.traidos += 1;
                }
            }
            (Llegada::Borrado | Llegada::Ignorar | Llegada::Cancelada { .. }, _) => {}
        }
    }
    Ok(())
}

/// Guarda uno que llegó de Google. Si Nova no lo acepta (un dato que no
/// valida), se anota en la consola y se sigue con el resto.
fn guardar(conexion: &mut Connection, nuevo: DeGoogle, ahora: &Ahora) -> bool {
    let id = nuevo.id_externo.clone();
    match repo::guardar_de_google(conexion, nuevo, ahora) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Google Calendar: no guardé el evento {id}: {e}");
            false
        }
    }
}

/// Con la lista completa: lo que Nova tenía vinculado y Google ya no tiene
/// se borró allá mientras no se sincronizaba.
fn quitar_desaparecidos(conexion: &mut Connection, lista: &Lista, desde: NaiveDate) -> Resultado<u32> {
    let en_google: HashSet<&str> = lista.eventos.iter().filter(|e| !e.cancelado()).map(|e| e.id.as_str()).collect();
    let margen = desde + Duration::days(1);
    let mut borrados = 0;
    for (evento, v) in repo::todos_con_vinculo(conexion)? {
        let Some(id) = &v.id_externo else { continue };
        if evento.solo_lectura
            || v.cambiado_local
            || en_google.contains(id.as_str())
            || !logica::vigente(&evento.inicio, evento.fin.as_deref(), evento.recurrencia, evento.repetir_hasta.as_deref(), margen)
        {
            continue;
        }
        if repo::borrar_de_google(conexion, id)? {
            borrados += 1;
        }
    }
    Ok(borrados)
}

/// Trae de nuevo las repeticiones de una serie que Nova no sabe repetir.
async fn traer_sueltas(db: &Db, api: &impl ApiCalendar, serie: &str, zona: FixedOffset, ahora: &Ahora) -> Result<u32, Falla> {
    let desde = rfc3339(ahora.hoy - Duration::days(1), zona);
    let hasta = rfc3339(ahora.hoy + Duration::days(DIAS_DE_SUELTAS), zona);
    let repeticiones = match api.repeticiones(serie, &desde, &hasta).await {
        Err(Falla::NoExiste | Falla::SyncVencido) => Vec::new(),
        otro => otro?,
    };
    let mut conexion = db.conexion();
    let materias = materias_conocidas(&conexion).map_err(base)?;
    repo::borrar_sueltas(&conexion, serie).map_err(base)?;
    let mut traidas = 0;
    for ev in repeticiones.iter().filter(|e| !e.cancelado()) {
        let Some(t) = logica::traducir_datos(ev, zona, &materias) else { continue };
        let nuevo = DeGoogle { solo_lectura: true, serie_externa: Some(serie.to_string()), ..de_google(ev, t) };
        if guardar(&mut conexion, nuevo, ahora) {
            traidas += 1;
        }
    }
    Ok(traidas)
}

/// El cuerpo para Google de un evento de Nova.
fn armar_cuerpo(conexion: &Connection, evento: &Evento, vinculo: &Vinculo, zona_calendario: &str) -> Resultado<serde_json::Value> {
    let materia = match evento.materia_id {
        Some(id) => Some(materias::obtener(conexion, id)?.nombre),
        None => None,
    };
    let movidas = repo::fechas_movidas(conexion, evento.id)?;
    let excepciones = evento
        .excepciones
        .iter()
        .filter(|f| !movidas.contains(f))
        .filter_map(|f| NaiveDate::parse_from_str(f, "%Y-%m-%d").ok())
        .collect();
    let p = ParaSubir {
        titulo: &evento.titulo,
        tipo: evento.tipo,
        inicio: &evento.inicio,
        fin: evento.fin.as_deref(),
        materia: materia.as_deref(),
        persona: evento.persona.as_deref(),
        notas: evento.notas.as_deref(),
        recurrencia: evento.recurrencia,
        repetir_hasta: evento.repetir_hasta.as_deref(),
        excepciones,
        es_repeticion: vinculo.serie_id.is_some(),
    };
    logica::cuerpo(&p, zona_calendario).map_err(crate::error::Error::Sistema)
}

/// Sube lo borrado y lo cambiado en Nova. Sin internet corta y lo deja para
/// la próxima vuelta.
async fn subir(
    db: &Db,
    api: &impl ApiCalendar,
    zona_calendario: &str,
    zona: FixedOffset,
    desde: NaiveDate,
    ahora: &Ahora,
    resumen: &mut Resumen,
) -> Result<u32, Falla> {
    // Primero se copia la lista: si el `for` usara `db.conexion()` directo,
    // el candado quedaría tomado durante todo el ciclo (y adentro se pide otra vez).
    let borrados = repo::borrados(&db.conexion()).map_err(base)?;
    for id in borrados {
        api.borrar(&id).await?;
        repo::quitar_borrado(&db.conexion(), &id).map_err(base)?;
    }

    let pendientes = repo::pendientes(&db.conexion()).map_err(base)?;
    let mut subidos = 0;
    for (evento, vinculo) in pendientes {
        // Lo viejo que nunca estuvo en Google se queda solo en Nova.
        if vinculo.id_externo.is_none()
            && !logica::vigente(&evento.inicio, evento.fin.as_deref(), evento.recurrencia, evento.repetir_hasta.as_deref(), desde)
        {
            continue;
        }
        let cuerpo = armar_cuerpo(&db.conexion(), &evento, &vinculo, zona_calendario).map_err(base)?;
        let resultado = match &vinculo.id_externo {
            None => api.crear(&cuerpo).await,
            Some(id) => match api.modificar(id, &cuerpo, vinculo.etag.as_deref()).await {
                // Cambió en Google desde la última vez: gana el cambio más nuevo.
                Err(Falla::Conflicto) => {
                    let actual = api.obtener(id).await?;
                    match logica::quien_gana(&vinculo.actualizado_en, actual.updated.as_deref()) {
                        logica::Ganador::Nova => api.modificar(id, &cuerpo, None).await,
                        logica::Ganador::Google => {
                            // Se marca como sin cambios para que entre lo de Google.
                            let mut conexion = db.conexion();
                            conexion
                                .execute("UPDATE eventos SET cambiado_local = 0 WHERE id = ?1", [evento.id])
                                .map_err(|e| base(e.into()))?;
                            procesar(&mut conexion, &[actual], zona, ahora, resumen, &mut HashSet::new()).map_err(base)?;
                            continue;
                        }
                    }
                }
                // Lo borraron en Google pero acá lo editaste: gana la edición.
                Err(Falla::NoExiste) => {
                    let sin_serie = if vinculo.serie_id.is_some() {
                        let mut c = cuerpo.clone();
                        c["recurrence"] = serde_json::json!(Vec::<String>::new());
                        c
                    } else {
                        cuerpo.clone()
                    };
                    api.crear(&sin_serie).await
                }
                otro => otro,
            },
        };
        let creado = resultado?;
        repo::marcar_subido(&db.conexion(), evento.id, &creado.id, creado.etag.as_deref(), creado.updated.as_deref(), &vinculo.actualizado_en)
            .map_err(base)?;
        subidos += 1;
    }
    Ok(subidos)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::Value;

    use super::*;
    use crate::dominio::momento::Momento;
    use crate::dominio::texto::{Recurrencia, TipoEvento};
    use crate::repos::eventos::tests::datos;
    use crate::repos::responsabilidades;

    /// Un Google de mentira: guarda los eventos en memoria, cuenta versiones
    /// (etag) y devuelve en cada lista lo que cambió desde la anterior.
    #[derive(Default)]
    struct FalsoGoogle {
        estado: Mutex<Falso>,
    }

    #[derive(Default)]
    struct Falso {
        eventos: HashMap<String, EventoGoogle>,
        /// Ids cambiados desde la última lista (incluye los borrados).
        cambiados: Vec<String>,
        version: u32,
        sin_red: bool,
        token_vencido: bool,
        reloj: u32,
    }

    impl Falso {
        fn marcar(&mut self, ev: &mut EventoGoogle) {
            self.version += 1;
            self.reloj += 1;
            ev.etag = Some(format!("\"{}\"", self.version));
            // Google pone su propia hora: la hacemos avanzar desde el mediodía de UTC.
            ev.updated = Some(format!("2026-11-17T15:{:02}:00.000Z", self.reloj.min(59)));
            self.cambiados.push(ev.id.clone());
        }
    }

    impl FalsoGoogle {
        /// Como si lo cargaras desde el celular.
        fn cargar(&self, id: &str, titulo: &str, inicio: &str) {
            let mut f = self.estado.lock().unwrap();
            let mut ev = EventoGoogle {
                id: id.into(),
                status: Some("confirmed".into()),
                summary: Some(titulo.into()),
                start: Some(cuando(inicio)),
                end: Some(cuando(&fin_de(inicio))),
                ..Default::default()
            };
            f.marcar(&mut ev);
            f.eventos.insert(id.into(), ev);
        }

        fn cambiar(&self, id: &str, cambio: impl FnOnce(&mut EventoGoogle)) {
            let mut f = self.estado.lock().unwrap();
            let mut ev = f.eventos.get(id).cloned().unwrap();
            cambio(&mut ev);
            f.marcar(&mut ev);
            f.eventos.insert(id.into(), ev);
        }

        fn borrar_en_google(&self, id: &str) {
            self.cambiar(id, |e| e.status = Some("cancelled".into()));
        }

        fn titulos(&self) -> Vec<String> {
            let f = self.estado.lock().unwrap();
            let mut t: Vec<String> = f.eventos.values().filter(|e| !e.cancelado()).filter_map(|e| e.summary.clone()).collect();
            t.sort();
            t
        }

        fn evento(&self, id: &str) -> EventoGoogle {
            self.estado.lock().unwrap().eventos[id].clone()
        }

        fn sin_red(&self, valor: bool) {
            self.estado.lock().unwrap().sin_red = valor;
        }

        fn red(&self) -> Result<(), Falla> {
            if self.estado.lock().unwrap().sin_red { Err(Falla::SinRed("sin internet".into())) } else { Ok(()) }
        }
    }

    fn cuando(t: &str) -> crate::dominio::google::CuandoGoogle {
        if t.len() == 10 {
            crate::dominio::google::CuandoGoogle { date: Some(t.into()), ..Default::default() }
        } else {
            crate::dominio::google::CuandoGoogle { date_time: Some(t.into()), ..Default::default() }
        }
    }

    fn fin_de(inicio: &str) -> String {
        match Momento::parsear(inicio).unwrap() {
            Momento::Fecha(f) => (f + Duration::days(1)).format("%Y-%m-%d").to_string(),
            Momento::FechaHora(fh) => (fh + Duration::hours(2)).to_rfc3339_opts(SecondsFormat::Secs, false),
        }
    }

    impl ApiCalendar for FalsoGoogle {
        async fn listar(&self, sync_token: Option<&str>, _desde: Option<&str>) -> Result<Lista, Falla> {
            self.red()?;
            let mut f = self.estado.lock().unwrap();
            if sync_token.is_some() && f.token_vencido {
                f.token_vencido = false;
                return Err(Falla::SyncVencido);
            }
            let eventos: Vec<EventoGoogle> = match sync_token {
                Some(_) => f.cambiados.iter().filter_map(|id| f.eventos.get(id).cloned()).collect(),
                None => f.eventos.values().filter(|e| !e.cancelado()).cloned().collect(),
            };
            f.cambiados.clear();
            Ok(Lista { eventos, sync_token: Some(format!("token-{}", f.version)), zona: Some(ZONA_POR_DEFECTO.into()) })
        }

        async fn repeticiones(&self, _serie: &str, _desde: &str, _hasta: &str) -> Result<Vec<EventoGoogle>, Falla> {
            Ok(Vec::new())
        }

        async fn obtener(&self, id: &str) -> Result<EventoGoogle, Falla> {
            self.red()?;
            self.estado.lock().unwrap().eventos.get(id).cloned().ok_or(Falla::NoExiste)
        }

        async fn crear(&self, cuerpo: &Value) -> Result<EventoGoogle, Falla> {
            self.red()?;
            let mut f = self.estado.lock().unwrap();
            let mut ev: EventoGoogle = serde_json::from_value(cuerpo.clone()).unwrap();
            ev.id = format!("nova{}", f.eventos.len() + 1);
            ev.status = Some("confirmed".into());
            f.marcar(&mut ev);
            f.eventos.insert(ev.id.clone(), ev.clone());
            Ok(ev)
        }

        async fn modificar(&self, id: &str, cuerpo: &Value, etag: Option<&str>) -> Result<EventoGoogle, Falla> {
            self.red()?;
            let mut f = self.estado.lock().unwrap();
            let actual = f.eventos.get(id).cloned().ok_or(Falla::NoExiste)?;
            if actual.cancelado() {
                return Err(Falla::NoExiste);
            }
            if etag.is_some() && etag != actual.etag.as_deref() {
                return Err(Falla::Conflicto);
            }
            let mut valor = serde_json::to_value(&actual).unwrap();
            for (k, v) in cuerpo.as_object().unwrap() {
                valor[k] = v.clone();
            }
            let mut ev: EventoGoogle = serde_json::from_value(valor).unwrap();
            f.marcar(&mut ev);
            f.eventos.insert(id.into(), ev.clone());
            Ok(ev)
        }

        async fn borrar(&self, id: &str) -> Result<(), Falla> {
            self.red()?;
            let mut f = self.estado.lock().unwrap();
            if let Some(mut ev) = f.eventos.get(id).cloned() {
                ev.status = Some("cancelled".into());
                f.marcar(&mut ev);
                f.eventos.insert(id.into(), ev);
            }
            Ok(())
        }
    }

    fn base() -> Db {
        Db::de_conexion(crate::db::en_memoria())
    }

    fn ahora() -> Ahora {
        Ahora::fijo("2026-11-17")
    }

    fn titulos_en_nova(db: &Db) -> Vec<String> {
        let mut t: Vec<String> = eventos::listar_todos(&db.conexion()).unwrap().into_iter().map(|e| e.titulo).collect();
        t.sort();
        t
    }

    #[tokio::test]
    async fn la_primera_vez_sube_trae_y_junta() {
        let db = base();
        let google = FalsoGoogle::default();
        eventos::crear(&mut db.conexion(), datos("Clase de Física", TipoEvento::Clase, "2026-11-18T15:00:00-03:00"), &ahora()).unwrap();
        eventos::crear(&mut db.conexion(), datos("Parcial de Análisis", TipoEvento::Examen, "2026-11-20T09:00:00-03:00"), &ahora()).unwrap();
        eventos::crear(&mut db.conexion(), datos("Viejo", TipoEvento::Otro, "2026-01-10"), &ahora()).unwrap();
        google.cargar("g1", "Dentista", "2026-11-19T10:00:00-03:00");
        // El mismo parcial, cargado también en el celular.
        google.cargar("g2", "parcial de analisis", "2026-11-20T12:00:00Z");

        let r = sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!((r.traidos, r.subidos, r.juntados), (1, 2, 1));
        assert_eq!(google.titulos(), ["Clase de Física", "Dentista", "Parcial de Análisis"]);
        assert_eq!(titulos_en_nova(&db), ["Clase de Física", "Dentista", "Parcial de Análisis", "Viejo"]);
        // El tipo de Nova llegó a Google.
        assert_eq!(google.evento("g2").extended_properties.unwrap().private["nova_tipo"], "examen");

        // Una segunda vuelta sin cambios no hace nada.
        let r = sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(r, Resumen::default());
    }

    #[tokio::test]
    async fn lo_del_celular_llega_con_sus_automaticas() {
        let db = base();
        let google = FalsoGoogle::default();
        sincronizar(&db, &google, &ahora()).await.unwrap();

        google.cargar("g1", "Parcial de Física", "2026-11-24");
        let r = sincronizar(&db, &google, &ahora()).await.unwrap();
        assert!(r.cambio_algo_aca());
        let pendientes = responsabilidades::listar(&db.conexion()).unwrap();
        assert_eq!(pendientes.len(), 1);
        assert_eq!(pendientes[0].fecha_limite.as_deref(), Some("2026-11-23"));
        // Las automáticas no se suben a Google.
        assert_eq!(google.titulos(), ["Parcial de Física"]);
        assert!(repo::pendientes(&db.conexion()).unwrap().is_empty());
    }

    #[tokio::test]
    async fn editar_y_borrar_de_cada_lado() {
        let db = base();
        let google = FalsoGoogle::default();
        let e = eventos::crear(&mut db.conexion(), datos("Clase", TipoEvento::Clase, "2026-11-18T15:00:00-03:00"), &ahora()).unwrap();
        google.cargar("g1", "Turno", "2026-11-19");
        sincronizar(&db, &google, &ahora()).await.unwrap();

        // Nova → Google
        let mut cambio = datos("Clase de consulta", TipoEvento::Clase, "2026-11-18T16:00:00-03:00");
        cambio.notas = Some("Aula 3".into());
        eventos::editar(&mut db.conexion(), e.id, cambio, &Ahora::fijo_a_las("2026-11-17", "12:10")).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();
        let id_google = repo::vinculo(&db.conexion(), e.id).unwrap().unwrap().id_externo.unwrap();
        let en_google = google.evento(&id_google);
        assert_eq!(en_google.summary.as_deref(), Some("Clase de consulta"));
        assert_eq!(en_google.description.as_deref(), Some("Aula 3"));

        // Google → Nova
        google.cambiar("g1", |ev| ev.summary = Some("Turno médico".into()));
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(titulos_en_nova(&db), ["Clase de consulta", "Turno médico"]);

        // Borrar en Google
        google.borrar_en_google("g1");
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(titulos_en_nova(&db), ["Clase de consulta"]);

        // Borrar en Nova
        eventos::borrar(&mut db.conexion(), e.id, &ahora()).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert!(google.titulos().is_empty());
        assert!(repo::borrados(&db.conexion()).unwrap().is_empty());
    }

    #[tokio::test]
    async fn sin_internet_queda_pendiente_y_sube_al_volver() {
        let db = base();
        let google = FalsoGoogle::default();
        sincronizar(&db, &google, &ahora()).await.unwrap();

        google.sin_red(true);
        eventos::crear(&mut db.conexion(), datos("Entrega TP", TipoEvento::Entrega, "2026-11-25"), &ahora()).unwrap();
        assert!(matches!(sincronizar(&db, &google, &ahora()).await, Err(Falla::SinRed(_))));
        assert_eq!(repo::pendientes(&db.conexion()).unwrap().len(), 1);

        google.sin_red(false);
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(google.titulos(), ["Entrega TP"]);
        assert!(repo::pendientes(&db.conexion()).unwrap().is_empty());
    }

    #[tokio::test]
    async fn si_cambio_de_los_dos_lados_gana_el_mas_nuevo() {
        let db = base();
        let google = FalsoGoogle::default();
        google.cargar("g1", "Turno", "2026-11-19");
        sincronizar(&db, &google, &ahora()).await.unwrap();
        let id = repo::vinculo_por_externo(&db.conexion(), "g1").unwrap().unwrap().id;

        // En Google a las 12:02 (15:02 UTC); en Nova a las 12:30: gana Nova.
        google.cambiar("g1", |ev| ev.summary = Some("Turno (Google)".into()));
        eventos::editar(&mut db.conexion(), id, datos("Turno (Nova)", TipoEvento::Otro, "2026-11-19"), &Ahora::fijo_a_las("2026-11-17", "12:30")).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(google.titulos(), ["Turno (Nova)"]);
        assert_eq!(titulos_en_nova(&db), ["Turno (Nova)"]);

        // Ahora en Nova a las 11:00 (antes que lo de Google): gana Google.
        google.cambiar("g1", |ev| ev.summary = Some("Turno (Google otra vez)".into()));
        eventos::editar(&mut db.conexion(), id, datos("Turno viejo", TipoEvento::Otro, "2026-11-19"), &Ahora::fijo_a_las("2026-11-17", "11:00")).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(google.titulos(), ["Turno (Google otra vez)"]);
        assert_eq!(titulos_en_nova(&db), ["Turno (Google otra vez)"]);
    }

    #[tokio::test]
    async fn borrado_en_google_pero_editado_en_nova_vuelve() {
        let db = base();
        let google = FalsoGoogle::default();
        google.cargar("g1", "Turno", "2026-11-19");
        sincronizar(&db, &google, &ahora()).await.unwrap();
        let id = repo::vinculo_por_externo(&db.conexion(), "g1").unwrap().unwrap().id;

        google.borrar_en_google("g1");
        eventos::editar(&mut db.conexion(), id, datos("Turno cambiado", TipoEvento::Otro, "2026-11-19"), &ahora()).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(google.titulos(), ["Turno cambiado"]);
        assert_eq!(titulos_en_nova(&db), ["Turno cambiado"]);
    }

    #[tokio::test]
    async fn si_google_pide_empezar_de_nuevo_no_duplica() {
        let db = base();
        let google = FalsoGoogle::default();
        google.cargar("g1", "Turno", "2026-11-19");
        eventos::crear(&mut db.conexion(), datos("Clase", TipoEvento::Clase, "2026-11-18"), &ahora()).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();

        google.estado.lock().unwrap().token_vencido = true;
        // Mientras tanto lo borraron en Google sin que nos enteremos.
        {
            let mut f = google.estado.lock().unwrap();
            f.eventos.remove("g1");
            f.cambiados.clear();
        }
        sincronizar(&db, &google, &ahora()).await.unwrap();
        assert_eq!(titulos_en_nova(&db), ["Clase"]);
        assert_eq!(google.titulos(), ["Clase"]);
    }

    #[tokio::test]
    async fn una_serie_ida_y_vuelta_con_fechas_salteadas() {
        let db = base();
        let google = FalsoGoogle::default();
        let mut clase = datos("Análisis", TipoEvento::Clase, "2026-11-02T15:00:00-03:00");
        clase.recurrencia = Recurrencia::Semanal;
        clase.repetir_hasta = Some("2026-12-07".into());
        let clase = eventos::crear(&mut db.conexion(), clase, &ahora()).unwrap();
        eventos::cambiar_excepcion(&mut db.conexion(), clase.id, "2026-11-23", true, &ahora()).unwrap();
        sincronizar(&db, &google, &ahora()).await.unwrap();

        let id_google = repo::vinculo(&db.conexion(), clase.id).unwrap().unwrap().id_externo.unwrap();
        let en_google = google.evento(&id_google);
        assert_eq!(
            en_google.recurrence.unwrap(),
            ["RRULE:FREQ=WEEKLY;UNTIL=20261208T025959Z", "EXDATE:20261123T180000Z"]
        );

        // En Google cancelan el 30: llega como fecha salteada.
        let cancelada = EventoGoogle {
            id: format!("{id_google}_20261130T180000Z"),
            status: Some("cancelled".into()),
            recurring_event_id: Some(id_google.clone()),
            original_start_time: Some(cuando("2026-11-30T15:00:00-03:00")),
            ..Default::default()
        };
        {
            let mut f = google.estado.lock().unwrap();
            let mut ev = cancelada;
            f.marcar(&mut ev);
            f.eventos.insert(ev.id.clone(), ev);
        }
        sincronizar(&db, &google, &ahora()).await.unwrap();
        let clase = eventos::obtener(&db.conexion(), clase.id).unwrap();
        assert_eq!(clase.excepciones, ["2026-11-23", "2026-11-30"]);
    }

    #[test]
    fn el_estado_se_guarda_en_la_cache() {
        let db = base();
        let estado = EstadoCalendar { sync_token: Some("t".into()), ..Default::default() };
        guardar_estado(&db, &estado, &ahora(), Duration::minutes(5)).unwrap();
        assert_eq!(leer_estado(&db).unwrap(), estado);
        olvidar(&db.conexion()).unwrap();
        assert_eq!(leer_estado(&db).unwrap(), EstadoCalendar::default());
    }
}
