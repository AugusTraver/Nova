//! Herramientas de consulta (v0.4): fútbol, noticias y el resumen del día.
//! Leen la caché (nunca la red): la tarea de fondo la mantiene al día, así
//! que contestan al instante aunque no haya internet.

use chrono::{DateTime, Timelike};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::clima::{Clima, CLAVE as CLAVE_CLIMA};
use crate::dominio::futbol::{self as logica, ResumenEquipo};
use crate::dominio::lenguaje::normalizar;
use crate::dominio::resumen::{self, DatosResumen, EventoDelDia};
use crate::dominio::texto::{EstadoResponsabilidad, Prioridad};
use crate::futbol::vista::{self, VistaFutbol};
use crate::noticias;
use crate::repos::cache::{self, Cacheado};
use crate::repos::{Ahora, eventos, preferencias, responsabilidades, texto_fecha};

type Paso<T> = Result<T, String>;

fn error(e: crate::error::Error) -> String {
    e.to_string()
}

/// Si el dato es viejo porque falló la descarga, se le avisa al modelo
/// para que lo aclare ("según lo último que tengo, de hace 3 horas").
fn aviso<T>(c: &Cacheado<T>) -> Option<String> {
    c.error.as_ref().map(|e| format!("{e} Los datos son de {}.", c.obtenido_en.as_deref().unwrap_or("antes")))
}

fn futbol(conexion: &Connection, ahora: &Ahora) -> Paso<(Cacheado<VistaFutbol>, VistaFutbol)> {
    let c = vista::vista(conexion, ahora).map_err(error)?;
    let datos = c.datos.clone().ok_or("Todavía no tengo datos de fútbol (¿hay internet?). Decile que pruebe en un rato.")?;
    Ok((c, datos))
}

fn equipo<'a>(v: &'a VistaFutbol, clave: &str) -> Paso<&'a ResumenEquipo> {
    v.equipos.iter().find(|e| e.clave == clave).ok_or_else(|| format!("No sigo a «{clave}»."))
}

#[derive(Deserialize)]
pub struct ArgsPartidos {
    equipo: Option<String>,
}

pub fn consultar_partidos(conexion: &Connection, a: ArgsPartidos, ahora: &Ahora) -> Paso<Value> {
    let clave = match a.equipo.as_deref().map(normalizar).as_deref() {
        None | Some("") | Some("boca") | Some("boca juniors") => "boca",
        Some("seleccion") | Some("argentina") | Some("seleccion argentina") => "seleccion",
        Some(otro) => return Err(format!("Solo sigo a Boca y a la Selección, no a «{otro}».")),
    };
    let (c, v) = futbol(conexion, ahora)?;
    let e = equipo(&v, clave)?;
    Ok(json!({
        "equipo": e.nombre,
        "en_vivo": e.en_vivo.as_ref().map(|p| logica::describir_en_vivo(p, &e.id)),
        "proximo": e.proximo.as_ref().map(|p| logica::describir_proximo(p, &e.id, ahora.momento)),
        "ultimo": e.ultimo.as_ref().map(|p| logica::describir_ultimo(p, &e.id, ahora.momento)),
        "aviso": aviso(&c),
    }))
}

pub fn consultar_tabla(conexion: &Connection, ahora: &Ahora) -> Paso<Value> {
    let (c, v) = futbol(conexion, ahora)?;
    let e = equipo(&v, "boca")?;
    let p = e.posicion.as_ref().ok_or("No tengo la tabla del torneo todavía.")?;
    Ok(json!({
        "torneo": p.torneo,
        "zona": p.zona,
        "puesto": p.fila.puesto,
        "equipos_en_zona": p.equipos_en_zona,
        "puntos": p.fila.puntos,
        "jugados": p.fila.jugados,
        "ganados": p.fila.ganados,
        "empatados": p.fila.empatados,
        "perdidos": p.fila.perdidos,
        "diferencia_de_gol": p.fila.diferencia,
        "aviso": aviso(&c),
    }))
}

#[derive(Deserialize)]
pub struct ArgsJugador {
    nombre: String,
}

pub fn consultar_jugador(conexion: &Connection, a: ArgsJugador, ahora: &Ahora) -> Paso<Value> {
    let (c, v) = futbol(conexion, ahora)?;
    let buscado = normalizar(&a.nombre);
    let coincide = |nombre: &str| {
        let n = normalizar(nombre);
        // "Julián" encuentra "Julián Álvarez"; "Barco" encuentra "Valentín Barco".
        n == buscado || n.split_whitespace().any(|p| p == buscado) || n.contains(&buscado)
    };
    let encontrados: Vec<_> = v.jugadores.iter().filter(|j| coincide(&j.nombre)).collect();
    let j = match encontrados.as_slice() {
        [uno] => *uno,
        [] => {
            let seguidos = preferencias::jugadores(conexion).map_err(error)?;
            let nombres: Vec<String> = seguidos.into_iter().map(|s| s.nombre).collect();
            return Err(format!(
                "No sigo a «{}». Sigo a: {}. Se agregan en Ajustes.",
                a.nombre.trim(),
                nombres.join(", ")
            ));
        }
        varios => {
            let nombres: Vec<&str> = varios.iter().map(|j| j.nombre.as_str()).collect();
            return Err(format!("«{}» puede ser {}. Preguntale cuál.", a.nombre.trim(), nombres.join(" o ")));
        }
    };
    Ok(json!({
        "nombre": j.nombre,
        "club": j.club,
        "ultimo_partido": j.ultimo.as_ref().map(|u| json!({
            "cuando": logica::cuando(&u.fecha, ahora.momento),
            "rival": u.rival,
            "resultado": u.resultado,
            "competencia": u.competencia,
            "goles": u.goles,
            "asistencias": u.asistencias,
        })),
        "temporada": { "partidos": j.partidos, "goles": j.goles, "asistencias": j.asistencias },
        "aviso": aviso(&c),
    }))
}

pub fn consultar_noticias(conexion: &Connection) -> Paso<Value> {
    let c = noticias::vista(conexion).map_err(error)?;
    let lista = c.datos.clone().ok_or("Todavía no tengo noticias (¿hay internet?).")?;
    let lista: Vec<Value> = lista
        .iter()
        .map(|n| json!({ "titulo": n.titulo, "resumen": n.resumen, "fuente": n.fuente }))
        .collect();
    Ok(json!({ "noticias": lista, "aviso": aviso(&c) }))
}

/// "Boca juega hoy a las 21:30 contra Racing (de local, la Liga Profesional)".
/// Solo lo de hoy: partido en vivo, partido más tarde o resultado de las
/// últimas horas.
fn futbol_de_hoy(v: &VistaFutbol, ahora: &Ahora) -> Vec<String> {
    let mut lineas = Vec::new();
    for e in &v.equipos {
        let reciente = |iso: &str| {
            DateTime::parse_from_rfc3339(iso).is_ok_and(|f| (ahora.momento - f).num_hours() < 16 && f <= ahora.momento)
        };
        let es_hoy = |iso: &str| {
            DateTime::parse_from_rfc3339(iso)
                .is_ok_and(|f| f.with_timezone(&ahora.momento.timezone()).date_naive() == ahora.hoy)
        };
        if let Some(p) = &e.en_vivo {
            lineas.push(format!("{} está jugando, {}", e.nombre, logica::describir_en_vivo(p, &e.id)));
        } else if let Some(p) = e.proximo.as_ref().filter(|p| es_hoy(&p.fecha)) {
            lineas.push(format!("{} juega {}", e.nombre, logica::describir_proximo(p, &e.id, ahora.momento)));
        } else if let Some(p) = e.ultimo.as_ref().filter(|p| reciente(&p.fecha)) {
            lineas.push(format!("{} {}", e.nombre, logica::describir_ultimo(p, &e.id, ahora.momento)));
        }
    }
    lineas
}

fn describir_clima(c: &Clima) -> String {
    let mut texto = format!("{:.0} grados y {}", c.temperatura, c.descripcion);
    if let Some(max) = c.maxima {
        texto.push_str(&format!(", máxima de {max:.0}"));
    }
    if let Some(lluvia) = c.lluvia.filter(|l| *l >= 50) {
        texto.push_str(&format!(", con {lluvia}% de chances de lluvia"));
    }
    texto
}

/// Junta los datos del resumen desde la base y la caché.
pub fn datos_resumen(conexion: &Connection, ahora: &Ahora) -> Paso<DatosResumen> {
    let eventos_hoy = eventos::ocurrencias(conexion, ahora.hoy, ahora.hoy).map_err(error)?;
    let eventos: Vec<EventoDelDia> = eventos_hoy
        .iter()
        .map(|o| EventoDelDia {
            titulo: o.evento.titulo.clone(),
            hora: (!o.evento.todo_el_dia).then(|| o.inicio.get(11..16).unwrap_or("").to_string()).filter(|h| !h.is_empty()),
        })
        .collect();

    let hoy = texto_fecha(ahora.hoy);
    let pendientes: Vec<_> = responsabilidades::listar(conexion)
        .map_err(error)?
        .into_iter()
        .filter(|r| r.estado == EstadoResponsabilidad::Pendiente)
        .collect();
    let mut para_hoy: Vec<_> = pendientes.iter().filter(|r| r.fecha_limite.as_deref() == Some(hoy.as_str())).collect();
    let rango = |p: Prioridad| match p {
        Prioridad::Alta => 0,
        Prioridad::Media => 1,
        Prioridad::Baja => 2,
    };
    para_hoy.sort_by_key(|r| rango(r.prioridad));
    let vencidas = pendientes.iter().filter(|r| r.fecha_limite.as_deref().is_some_and(|f| f < hoy.as_str())).count();

    let futbol = vista::vista(conexion, ahora)
        .ok()
        .and_then(|c| c.datos)
        .map(|v| futbol_de_hoy(&v, ahora))
        .unwrap_or_default();
    let noticia = noticias::vista(conexion)
        .ok()
        .and_then(|c| c.datos)
        .and_then(|l| l.into_iter().next())
        .map(|n| n.titulo);
    let clima = if preferencias::clima(conexion).map(|a| a.mostrar).unwrap_or(false) {
        cache::leer::<Clima>(conexion, CLAVE_CLIMA).ok().and_then(|c| c.datos).map(|c| describir_clima(&c))
    } else {
        None
    };

    Ok(DatosResumen {
        saludo: resumen::saludo(ahora.momento.hour()).to_string(),
        eventos,
        para_hoy: para_hoy.iter().map(|r| r.titulo.clone()).collect(),
        vencidas,
        futbol,
        noticia,
        clima,
    })
}

pub fn resumen_del_dia(conexion: &Connection, ahora: &Ahora) -> Paso<Value> {
    let d = datos_resumen(conexion, ahora)?;
    Ok(json!({ "texto": resumen::texto(&d), "datos": d }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::dominio::futbol::tests::partido;
    use crate::futbol::{BOCA, EstadoPartido, Partido};
    use crate::repos::responsabilidades::DatosResponsabilidad;

    fn ahora() -> Ahora {
        Ahora::fijo_a_las("2026-11-17", "14:00")
    }

    fn con_futbol() -> Connection {
        let base = db::en_memoria();
        let a = ahora();
        let partidos: Vec<Partido> = vec![
            partido("1", "2026-11-14T22:00:00Z", EstadoPartido::Terminado, Some((3, 0))),
            partido("2", "2026-11-17T23:30:00Z", EstadoPartido::Programado, None),
        ];
        let vence = a.momento + chrono::Duration::hours(5);
        cache::guardar(&base, &vista::clave_calendario(BOCA.id), &partidos, vence, &a).unwrap();
        base
    }

    #[test]
    fn partidos_de_boca() {
        let base = con_futbol();
        let v = consultar_partidos(&base, ArgsPartidos { equipo: None }, &ahora()).unwrap();
        assert_eq!(v["equipo"], "Boca");
        assert!(v["proximo"].as_str().unwrap().starts_with("hoy a las 20:30"));
        assert!(v["ultimo"].as_str().unwrap().starts_with("ganó 3-0"));
        assert!(v["aviso"].is_null());
        assert!(consultar_partidos(&base, ArgsPartidos { equipo: Some("River".into()) }, &ahora()).is_err());
    }

    #[test]
    fn sin_datos_es_un_error_explicado() {
        let base = db::en_memoria();
        assert!(consultar_partidos(&base, ArgsPartidos { equipo: None }, &ahora()).unwrap_err().contains("Todavía"));
        assert!(consultar_noticias(&base).is_err());
    }

    #[test]
    fn jugador_que_no_sigo() {
        let base = con_futbol();
        let e = consultar_jugador(&base, ArgsJugador { nombre: "Riquelme".into() }, &ahora()).unwrap_err();
        assert!(e.contains("Messi"));
    }

    #[test]
    fn resumen_con_partido_y_pendientes() {
        let base = con_futbol();
        let a = ahora();
        responsabilidades::crear(
            &base,
            DatosResponsabilidad {
                titulo: "Llamar al dentista".into(),
                descripcion: None,
                fecha_limite: Some("2026-11-17".into()),
                prioridad: Prioridad::Media,
                evento_id: None,
                materia_id: None,
            },
            crate::dominio::texto::OrigenResponsabilidad::Manual,
            &a,
        )
        .unwrap();
        let v = resumen_del_dia(&base, &a).unwrap();
        let texto = v["texto"].as_str().unwrap();
        assert!(texto.starts_with("Buenas tardes. Hoy no tenés nada en la agenda. Para hoy: Llamar al dentista."), "{texto}");
        assert!(texto.contains("Boca juega hoy a las 20:30"), "{texto}");
    }
}
