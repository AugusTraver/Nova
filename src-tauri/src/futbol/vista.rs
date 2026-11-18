//! Lo que muestran el widget y el asistente, armado desde la caché. Nunca
//! toca la red: la tarea de fondo (`fondo.rs`) es la que descarga.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::{EQUIPOS, EstadoJugador, Partido, Tabla};
use crate::dominio::futbol::{self, ResumenEquipo};
use crate::error::Resultado;
use crate::repos::cache::{self, Cacheado};
use crate::repos::{Ahora, preferencias};

pub fn clave_calendario(equipo_id: &str) -> String {
    format!("futbol:calendario:{equipo_id}")
}

pub fn clave_tabla(liga: &str) -> String {
    format!("futbol:tabla:{liga}")
}

pub fn clave_jugador(jugador_id: &str) -> String {
    format!("futbol:jugador:{jugador_id}")
}

/// La última lectura detallada del partido en vivo de un equipo.
pub fn clave_vivo(equipo_id: &str) -> String {
    format!("futbol:vivo:{equipo_id}")
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VistaFutbol {
    /// Boca primero, después la Selección.
    pub equipos: Vec<ResumenEquipo>,
    /// En el orden de Ajustes; solo los que ya tienen datos.
    pub jugadores: Vec<EstadoJugador>,
}

/// La vista completa. `datos` es `None` si todavía no se descargó nada.
pub fn vista(conexion: &Connection, ahora: &Ahora) -> Resultado<Cacheado<VistaFutbol>> {
    let mut obtenido_en: Option<String> = None;
    let mut error: Option<String> = None;
    let mut hay_algo = false;
    let mut equipos = Vec::new();

    for equipo in EQUIPOS.iter() {
        let calendario: Cacheado<Vec<Partido>> = cache::leer(conexion, &clave_calendario(equipo.id))?;
        let tabla: Option<Tabla> = match equipo.liga_tabla {
            Some(liga) => cache::leer(conexion, &clave_tabla(liga))?.datos,
            None => None,
        };
        let vivo: Option<Partido> = cache::leer(conexion, &clave_vivo(equipo.id))?.datos;
        // El dato más viejo es el que manda en el aviso "datos de hace...".
        if let Some(o) = &calendario.obtenido_en {
            if obtenido_en.as_ref().is_none_or(|actual| o < actual) {
                obtenido_en = Some(o.clone());
            }
        }
        error = error.or(calendario.error);
        hay_algo |= calendario.datos.is_some();
        let partidos = calendario.datos.unwrap_or_default();
        equipos.push(futbol::resumir(equipo, &partidos, tabla.as_ref(), vivo.as_ref(), ahora.momento));
    }

    let mut jugadores = Vec::new();
    for seguido in preferencias::jugadores(conexion)? {
        let estado: Cacheado<EstadoJugador> = cache::leer(conexion, &clave_jugador(&seguido.id))?;
        if let Some(mut j) = estado.datos {
            j.nombre = seguido.nombre.clone(); // el nombre corto que eligió el usuario
            jugadores.push(j);
        }
    }

    Ok(Cacheado {
        datos: hay_algo.then_some(VistaFutbol { equipos, jugadores }),
        obtenido_en,
        error,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::db;
    use crate::dominio::futbol::tests::partido;
    use crate::futbol::{EstadoPartido, JugadorSeguido};

    #[test]
    fn arma_la_vista_desde_la_cache() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo_a_las("2026-11-17", "14:00");
        assert_eq!(vista(&base, &ahora).unwrap().datos, None);

        let luego = ahora.momento + Duration::hours(10);
        let boca = vec![
            partido("1", "2026-11-14T22:00:00Z", EstadoPartido::Terminado, Some((3, 0))),
            partido("2", "2026-11-21T22:30:00Z", EstadoPartido::Programado, None),
        ];
        cache::guardar(&base, &clave_calendario("5"), &boca, luego, &ahora).unwrap();
        cache::anotar_error(&base, &clave_calendario("202"), "Sin conexión con ESPN.", &ahora).unwrap();

        preferencias::guardar(&base, preferencias::JUGADORES, &vec![
            JugadorSeguido { id: "1".into(), nombre: "Messi".into() },
            JugadorSeguido { id: "2".into(), nombre: "Sin datos".into() },
        ])
        .unwrap();
        let messi = EstadoJugador {
            id: "1".into(), nombre: "Lionel Messi".into(), club: Some("Inter Miami".into()), ultimo: None,
            partidos: 30, goles: 25, asistencias: 12,
        };
        cache::guardar(&base, &clave_jugador("1"), &messi, luego, &ahora).unwrap();

        let v = vista(&base, &ahora).unwrap();
        assert_eq!(v.error.as_deref(), Some("Sin conexión con ESPN."));
        assert_eq!(v.obtenido_en.as_deref(), Some("2026-11-17T14:00:00-03:00"));
        let datos = v.datos.unwrap();
        assert_eq!(datos.equipos[0].clave, "boca");
        assert_eq!(datos.equipos[0].ultimo.as_ref().unwrap().id, "1");
        assert_eq!(datos.equipos[0].proximo.as_ref().unwrap().id, "2");
        assert_eq!(datos.equipos[1].proximo, None, "la Selección sin datos queda vacía");
        assert_eq!(datos.jugadores.len(), 1);
        assert_eq!(datos.jugadores[0].nombre, "Messi");
    }
}
