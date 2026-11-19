//! Los avisos del sistema ya mandados (para no repetirlos).

use chrono::{DateTime, Duration};
use rusqlite::{Connection, OptionalExtension, params};

use super::{Ahora, cache, eventos, responsabilidades, texto_fecha};
use crate::dominio::avisos::{EventoProximo, PartidoProximo, Pendientes};
use crate::dominio::momento::Momento;
use crate::dominio::texto::EstadoResponsabilidad;
use crate::error::Resultado;
use crate::futbol::vista::clave_calendario;
use crate::futbol::{EQUIPOS, EstadoPartido, Partido};

/// Lo que puede generar avisos: los eventos con hora de hoy y mañana, lo
/// pendiente que vence hoy y los partidos guardados de los equipos seguidos.
pub fn pendientes(conexion: &Connection, ahora: &Ahora) -> Resultado<Pendientes> {
    let zona = *ahora.momento.offset();
    let eventos = eventos::ocurrencias(conexion, ahora.hoy, ahora.hoy + Duration::days(1))?
        .into_iter()
        .filter(|o| !o.evento.todo_el_dia)
        .filter_map(|o| match Momento::parsear(&o.inicio) {
            Ok(Momento::FechaHora(inicio)) => Some(EventoProximo {
                id: o.evento.id,
                titulo: o.evento.titulo,
                tipo: o.evento.tipo,
                inicio: inicio.with_timezone(&zona),
            }),
            _ => None,
        })
        .collect();

    let hoy = texto_fecha(ahora.hoy);
    let vencen_hoy = responsabilidades::listar(conexion)?
        .into_iter()
        .filter(|r| r.estado == EstadoResponsabilidad::Pendiente && r.fecha_limite.as_deref() == Some(hoy.as_str()))
        .map(|r| r.titulo)
        .collect();

    let mut partidos = Vec::new();
    for equipo in EQUIPOS.iter() {
        let lista: Vec<Partido> =
            cache::leer(conexion, &clave_calendario(equipo.id))?.datos.unwrap_or_default();
        for p in lista {
            if !matches!(p.estado, EstadoPartido::Programado | EstadoPartido::EnJuego) {
                continue;
            }
            let Ok(inicio) = DateTime::parse_from_rfc3339(&p.fecha) else { continue };
            let rival = if p.local.id == equipo.id { p.visitante.nombre } else { p.local.nombre };
            partidos.push(PartidoProximo {
                id: p.id,
                equipo: equipo.nombre.to_string(),
                rival,
                competencia: p.competencia,
                inicio: inicio.with_timezone(&zona),
            });
        }
    }
    Ok(Pendientes { eventos, vencen_hoy, partidos })
}

/// Una semana alcanza: ninguna ventana de aviso dura más.
const DIAS_QUE_SE_GUARDAN: i64 = 7;

pub fn ya_enviado(conexion: &Connection, clave: &str) -> Resultado<bool> {
    let hay: Option<i64> =
        conexion.query_row("SELECT 1 FROM avisos_enviados WHERE clave = ?1", [clave], |f| f.get(0)).optional()?;
    Ok(hay.is_some())
}

pub fn anotar(conexion: &Connection, clave: &str, ahora: &Ahora) -> Resultado<()> {
    conexion.execute(
        "INSERT INTO avisos_enviados (clave, enviado_en) VALUES (?1, ?2) ON CONFLICT (clave) DO NOTHING",
        params![clave, ahora.marca],
    )?;
    Ok(())
}

/// Borra los viejos. Las marcas son ISO con la misma zona, así que se
/// comparan como texto.
pub fn limpiar(conexion: &Connection, ahora: &Ahora) -> Resultado<()> {
    let limite = Ahora::desde(ahora.momento - chrono::Duration::days(DIAS_QUE_SE_GUARDAN)).marca;
    conexion.execute("DELETE FROM avisos_enviados WHERE enviado_en < ?1", [limite])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn junta_lo_que_puede_avisar() {
        use crate::dominio::texto::{OrigenResponsabilidad, TipoEvento};
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-19");
        let ev = |t: &str, i: &str| eventos::tests::datos(t, TipoEvento::Clase, i);
        eventos::crear(&mut base, ev("Análisis II", "2026-11-19T18:00:00-03:00"), &ahora).unwrap();
        eventos::crear(&mut base, ev("Física", "2026-11-20T09:00:00-03:00"), &ahora).unwrap();
        eventos::crear(&mut base, ev("Lejos", "2026-11-25T09:00:00-03:00"), &ahora).unwrap();
        eventos::crear(&mut base, ev("Feriado", "2026-11-19"), &ahora).unwrap();
        let r = |t: &str, f: &str| responsabilidades::tests::datos(t, Some(f));
        let manual = OrigenResponsabilidad::Manual;
        responsabilidades::crear(&base, r("TP", "2026-11-19"), manual, &ahora).unwrap();
        let hecha = responsabilidades::crear(&base, r("Ya está", "2026-11-19"), manual, &ahora).unwrap();
        responsabilidades::marcar(&base, hecha.id, true, &ahora).unwrap();
        responsabilidades::crear(&base, r("Mañana", "2026-11-20"), manual, &ahora).unwrap();

        let p = pendientes(&base, &ahora).unwrap();
        let titulos: Vec<&str> = p.eventos.iter().map(|e| e.titulo.as_str()).collect();
        assert_eq!(titulos, ["Análisis II", "Física"]);
        assert_eq!(p.vencen_hoy, ["TP"]);
        assert!(p.partidos.is_empty());
    }

    #[test]
    fn anota_y_limpia() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-19");
        assert!(!ya_enviado(&base, "vence:2026-11-19").unwrap());
        anotar(&base, "vence:2026-11-19", &ahora).unwrap();
        anotar(&base, "vence:2026-11-19", &ahora).unwrap(); // dos veces no rompe
        assert!(ya_enviado(&base, "vence:2026-11-19").unwrap());

        limpiar(&base, &Ahora::fijo("2026-11-25")).unwrap();
        assert!(ya_enviado(&base, "vence:2026-11-19").unwrap());
        limpiar(&base, &Ahora::fijo("2026-11-27")).unwrap();
        assert!(!ya_enviado(&base, "vence:2026-11-19").unwrap());
    }
}
