//! La tarea de fondo: descarga los datos de internet cuando vencen y sigue
//! los partidos en vivo. Corre siempre, aunque el panel esté cerrado (así en
//! v0.6 puede avisar "arranca el partido").
//!
//! Cada vuelta solo lee la caché y compara fechas, así que en reposo casi no
//! gasta: duerme un minuto (o 30 s en vivo) y vuelve a mirar. El panel la
//! despierta al abrirse, para que los datos estén al día.
//!
//! Regla de oro con la base: el candado (`db.conexion()`) se toma y se suelta
//! en cada paso, NUNCA se retiene durante un `await` (mientras se espera la
//! red). Rust no deja compilar si lo hacemos: el candado no es `Send`.

use std::time::Duration as DuracionStd;

use chrono::Duration;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::asistente::herramientas::Fuente;
use crate::comandos::EVENTO_DATOS_CAMBIADOS;
use crate::db::Db;
use crate::dominio::futbol as logica;
use crate::dominio::refresco;
use crate::error::Resultado;
use crate::futbol::espn::ClienteEspn;
use crate::futbol::proveedor::{PEDIDOS_CALENDARIO, ProveedorFutbol};
use crate::futbol::vista::{clave_calendario, clave_jugador, clave_tabla, clave_vivo};
use crate::futbol::{EQUIPOS, EstadoPartido, Partido};
use crate::repos::cache;
use crate::repos::{Ahora, preferencias};
use crate::ventana;

/// Evento para la esfera: "¡gol de Boca!".
pub const EVENTO_GOL: &str = "futbol:gol";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvisoGol {
    /// "boca" o "seleccion".
    pub equipo: String,
    pub goles: u32,
}

/// Estado compartido de la tarea (lo guarda Tauri con `app.manage`).
/// `Notify` es un "timbre": `despertar()` corta la espera de la tarea.
#[derive(Default)]
pub struct Fondo {
    timbre: Notify,
}

impl Fondo {
    pub fn despertar(&self) {
        self.timbre.notify_one();
    }
}

/// Toma la base, hace algo y la suelta. Si falla, lo anota en la consola y
/// sigue: un error de la caché no puede frenar la tarea.
fn con_base<T>(db: &Db, paso: impl FnOnce(&Connection) -> Resultado<T>) -> Option<T> {
    let conexion = db.conexion();
    paso(&conexion).map_err(|e| eprintln!("Tarea de fondo: {e}")).ok()
}

fn vencida(db: &Db, clave: &str, ahora: &Ahora) -> bool {
    con_base(db, |c| cache::vencida(c, clave, ahora)).unwrap_or(false)
}

/// ¿Entran `cantidad` pedidos más a ESPN hoy?
fn permitido(db: &Db, cantidad: u32, ahora: &Ahora) -> bool {
    con_base(db, |c| cache::contar_pedidos(c, "espn", cantidad, logica::TOPE_DIARIO, ahora)).unwrap_or(false)
}

/// Guarda el resultado de un pedido: el dato con su vencimiento o el error.
fn anotar<T: Serialize>(db: &Db, clave: &str, resultado: &Resultado<T>, vence: impl FnOnce(&T) -> chrono::DateTime<chrono::FixedOffset>, ahora: &Ahora) {
    con_base(db, |c| match resultado {
        Ok(valor) => cache::guardar(c, clave, valor, vence(valor), ahora),
        Err(e) => cache::anotar_error(c, clave, &e.to_string(), ahora),
    });
}

/// Arranca la tarea. Se llama una vez, en el `setup`.
pub fn arrancar(app: &AppHandle) {
    let app = app.clone();
    // `spawn` la deja corriendo en el motor async de Tauri (tokio), sin
    // bloquear la interfaz. `async move` le pasa `app` a la tarea.
    tauri::async_runtime::spawn(async move {
        let espn = match ClienteEspn::nuevo() {
            Ok(c) => c,
            Err(e) => return eprintln!("No arranca la tarea de fondo: {e}"),
        };
        loop {
            let espera = vuelta(&app, &espn).await;
            let fondo = app.state::<Fondo>();
            // `select!` espera lo primero que pase: que se cumpla el tiempo o
            // que alguien toque el timbre.
            tokio::select! {
                _ = tokio::time::sleep(espera) => {}
                _ = fondo.timbre.notified() => {}
            }
        }
    });
}

/// Una vuelta: refresca lo vencido y devuelve cuánto dormir.
async fn vuelta(app: &AppHandle, espn: &ClienteEspn) -> DuracionStd {
    let db = app.state::<Db>();
    let panel_abierto = ventana::abierto(app);
    let mut cambios: Vec<Fuente> = Vec::new();

    let (hubo, espera) = futbol(app, &db, espn, panel_abierto).await;
    if hubo {
        cambios.push(Fuente::Futbol);
    }

    if !cambios.is_empty() {
        let _ = app.emit(EVENTO_DATOS_CAMBIADOS, &cambios);
    }
    espera.to_std().unwrap_or(DuracionStd::from_secs(60))
}

/// Fútbol: calendarios, tabla, jugadores y en vivo. Devuelve si cambió algo
/// y cuánto conviene dormir (menos si hay un partido en juego).
async fn futbol(app: &AppHandle, db: &Db, espn: &ClienteEspn, panel_abierto: bool) -> (bool, Duration) {
    let mut cambio = false;
    let mut espera = Duration::seconds(60);

    for equipo in EQUIPOS.iter() {
        let ahora = Ahora::del_sistema();
        let clave = clave_calendario(equipo.id);
        if vencida(db, &clave, &ahora) && permitido(db, PEDIDOS_CALENDARIO, &ahora) {
            let resultado = espn.calendario(equipo.id).await;
            anotar(db, &clave, &resultado, |p| logica::vence_calendario(p, ahora.momento), &ahora);
            cambio = true;
        }
    }

    // La tabla vence a la mañana o después del próximo partido de Boca.
    for equipo in EQUIPOS.iter() {
        let Some(liga) = equipo.liga_tabla else { continue };
        let ahora = Ahora::del_sistema();
        let clave = clave_tabla(liga);
        if vencida(db, &clave, &ahora) && permitido(db, 1, &ahora) {
            let partidos = partidos_guardados(db, equipo.id);
            let resultado = espn.tabla(liga).await;
            anotar(db, &clave, &resultado, |_| logica::vence_calendario(&partidos, ahora.momento), &ahora);
            cambio = true;
        }
    }

    // Jugadores: una vez por día, a la mañana (después de la fecha).
    let jugadores = con_base(db, preferencias::jugadores).unwrap_or_default();
    for jugador in &jugadores {
        let ahora = Ahora::del_sistema();
        let clave = clave_jugador(&jugador.id);
        if vencida(db, &clave, &ahora) && permitido(db, 1, &ahora) {
            let resultado = espn.jugador(&jugador.id, &jugador.nombre).await;
            anotar(db, &clave, &resultado, |_| refresco::proxima_manana(ahora.momento), &ahora);
            cambio = true;
        }
    }

    // En vivo: solo si hay un partido de Boca o la Selección en su horario.
    for equipo in EQUIPOS.iter() {
        let ahora = Ahora::del_sistema();
        let partidos = partidos_guardados(db, equipo.id);
        let Some(partido) = logica::en_vivo(&partidos, ahora.momento) else { continue };
        let clave = clave_vivo(equipo.id);
        let anterior: Option<Partido> = con_base(db, |c| cache::leer(c, &clave)).and_then(|l| l.datos);
        // Ya terminó según la última lectura: no hace falta seguir consultando
        // (el calendario se refresca solo y lo saca de la ventana).
        let ya_termino = anterior.as_ref().is_some_and(|a| {
            a.id == partido.id && matches!(a.estado, EstadoPartido::Terminado | EstadoPartido::Suspendido)
        });
        if ya_termino {
            continue;
        }
        if vencida(db, &clave, &ahora) && permitido(db, 1, &ahora) {
            match espn.partido(&partido.id).await {
                Ok(nuevo) => {
                    let intervalo = logica::intervalo_en_vivo(nuevo.estado, panel_abierto);
                    let goles = logica::goles_nuevos(anterior.as_ref(), &nuevo, equipo.id);
                    if goles > 0 {
                        let _ = app.emit(EVENTO_GOL, AvisoGol { equipo: equipo.clave.into(), goles });
                    }
                    if logica::termino(anterior.as_ref(), &nuevo) {
                        // Terminó: el resultado y la tabla se piden ya.
                        con_base(db, |c| {
                            cache::vencer(c, &clave_calendario(equipo.id), &ahora)?;
                            match equipo.liga_tabla {
                                Some(liga) => cache::vencer(c, &clave_tabla(liga), &ahora),
                                None => Ok(()),
                            }
                        });
                    }
                    con_base(db, |c| cache::guardar(c, &clave, &nuevo, ahora.momento + intervalo, &ahora));
                    espera = espera.min(intervalo);
                }
                Err(e) => {
                    // En vivo no se espera 5 minutos: se reintenta en uno.
                    let reintento = Duration::seconds(60);
                    con_base(db, |c| {
                        cache::anotar_error(c, &clave, &e.to_string(), &ahora)?;
                        cache::cambiar_vencimiento(c, &clave, ahora.momento + reintento)
                    });
                }
            }
            cambio = true;
        } else {
            let estado = anterior.map(|a| a.estado).unwrap_or(partido.estado);
            espera = espera.min(logica::intervalo_en_vivo(estado, panel_abierto));
        }
    }

    (cambio, espera)
}

fn partidos_guardados(db: &Db, equipo_id: &str) -> Vec<Partido> {
    con_base(db, |c| cache::leer::<Vec<Partido>>(c, &clave_calendario(equipo_id)))
        .and_then(|l| l.datos)
        .unwrap_or_default()
}
