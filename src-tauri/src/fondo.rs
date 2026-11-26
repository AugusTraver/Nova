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
use crate::clima::{self as servicio_clima, ClienteClima};
use crate::comandos::EVENTO_DATOS_CAMBIADOS;
use crate::db::Db;
use crate::dominio::futbol as logica;
use crate::dominio::noticias as eleccion;
use crate::dominio::refresco;
use crate::error::{Error, Resultado};
use crate::futbol::espn::ClienteEspn;
use crate::futbol::proveedor::{PEDIDOS_CALENDARIO, PEDIDOS_JUGADOR, ProveedorFutbol};
use crate::futbol::vista::{clave_calendario, clave_jugador, clave_tabla, clave_vivo};
use crate::futbol::{EQUIPOS, EstadoPartido, Partido};
use crate::llavero;
use crate::noticias::rss::ClienteNoticias;
use crate::noticias::{self as servicio_noticias, Elegidas, Titular};
use crate::dominio::avisos as logica_avisos;
use crate::repos::{avisos as repo_avisos, cache, temporizadores as repo_temporizadores};
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
        let clientes = match Clientes::nuevos() {
            Ok(c) => c,
            Err(e) => return eprintln!("No arranca la tarea de fondo: {e}"),
        };
        loop {
            let espera = vuelta(&app, &clientes).await;
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

/// Los clientes HTTP se arman una vez y se reusan (cada uno guarda sus
/// conexiones abiertas).
struct Clientes {
    espn: ClienteEspn,
    noticias: ClienteNoticias,
    clima: ClienteClima,
}

impl Clientes {
    fn nuevos() -> Resultado<Self> {
        Ok(Clientes { espn: ClienteEspn::nuevo()?, noticias: ClienteNoticias::nuevo()?, clima: ClienteClima::nuevo()? })
    }
}

/// Una vuelta: refresca lo vencido y devuelve cuánto dormir.
async fn vuelta(app: &AppHandle, clientes: &Clientes) -> DuracionStd {
    let db = app.state::<Db>();
    let panel_abierto = ventana::abierto(app);
    let mut cambios: Vec<Fuente> = Vec::new();

    let (hubo, espera) = futbol(app, &db, &clientes.espn, panel_abierto).await;
    if hubo {
        cambios.push(Fuente::Futbol);
    }
    if noticias(&db, &clientes.noticias).await {
        cambios.push(Fuente::Noticias);
    }
    if clima(&db, &clientes.clima).await {
        cambios.push(Fuente::Clima);
    }

    if !cambios.is_empty() {
        let _ = app.emit(EVENTO_DATOS_CAMBIADOS, &cambios);
    }
    avisos(app, &db);
    temporizadores(app, &db);
    // Cada minuto como mucho, así los avisos llegan a horario; y si hay un
    // temporizador antes, justo para él.
    let mut espera = espera.min(Duration::seconds(60));
    if let Some(proximo) = con_base(&db, repo_temporizadores::proximo).flatten() {
        let falta = proximo - Ahora::del_sistema().momento;
        espera = espera.min(falta.max(Duration::seconds(1)));
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
        if vencida(db, &clave, &ahora) && permitido(db, PEDIDOS_JUGADOR, &ahora) {
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

/// Noticias: baja los feeds cada media hora y, si hay titulares nuevos,
/// vuelve a elegir (con IA como mucho cada 4 horas; si no hay IA, por
/// palabras clave). Devuelve si cambió algo.
async fn noticias(db: &Db, cliente: &ClienteNoticias) -> bool {
    let ahora = Ahora::del_sistema();
    let ajustes = con_base(db, preferencias::noticias).unwrap_or_default();
    let mut cambio = false;

    if vencida(db, servicio_noticias::CLAVE_TITULARES, &ahora) {
        let mut todos: Vec<Titular> = Vec::new();
        for feed in &ajustes.feeds {
            match cliente.bajar(feed).await {
                Ok(titulares) => todos.extend(titulares),
                // Un feed caído no frena a los demás.
                Err(e) => eprintln!("Noticias: {e}"),
            }
        }
        let resultado: Resultado<Vec<Titular>> = if todos.is_empty() && !ajustes.feeds.is_empty() {
            Err(Error::Sistema("Sin conexión con las fuentes de noticias.".into()))
        } else {
            Ok(todos)
        };
        let despues = Ahora::del_sistema();
        anotar(
            db,
            servicio_noticias::CLAVE_TITULARES,
            &resultado,
            |_| despues.momento + Duration::minutes(eleccion::MINUTOS_ENTRE_FEEDS),
            &despues,
        );
        cambio = true;
    }

    let titulares: Vec<Titular> = con_base(db, |c| cache::leer(c, servicio_noticias::CLAVE_TITULARES))
        .and_then(|l| l.datos)
        .unwrap_or_default();
    if titulares.is_empty() {
        return cambio;
    }
    let previas: Option<Elegidas> =
        con_base(db, |c| cache::leer(c, servicio_noticias::CLAVE_ELEGIDAS)).and_then(|l| l.datos);
    if previas.is_some() && !vencida(db, servicio_noticias::CLAVE_ELEGIDAS, &ahora) {
        return cambio;
    }
    let candidatos = eleccion::candidatos(&titulares, ahora.momento);
    let novedades = previas.as_ref().is_none_or(|p| eleccion::hay_novedades(&p.candidatos, &candidatos));
    if !novedades {
        // Nada nuevo: lo elegido sigue valiendo un rato más.
        let siguiente = ahora.momento + Duration::minutes(eleccion::MINUTOS_ENTRE_FEEDS);
        con_base(db, |c| cache::cambiar_vencimiento(c, servicio_noticias::CLAVE_ELEGIDAS, siguiente));
        return cambio;
    }

    let con_ia = match cliente_ia(db) {
        Ok(ia) => match ia.texto(&eleccion::armar_pedido(&candidatos, &ajustes.intereses)).await {
            Ok(texto) => eleccion::leer_respuesta(&texto, &candidatos),
            Err(e) => {
                eprintln!("Noticias sin IA: {e}");
                None
            }
        },
        Err(_) => None, // sin key o sin proveedor: orden por palabras clave
    };
    let ahora = Ahora::del_sistema();
    let (lista, usada_ia, minutos) = match con_ia {
        Some(lista) => (lista, true, eleccion::MINUTOS_ENTRE_IA),
        None => (
            eleccion::ordenar_sin_ia(&candidatos, &eleccion::intereses(&ajustes.intereses), ahora.momento),
            false,
            eleccion::MINUTOS_ENTRE_FEEDS,
        ),
    };
    let elegidas = Elegidas { noticias: lista, con_ia: usada_ia, candidatos: candidatos.iter().map(|t| t.id.clone()).collect() };
    con_base(db, |c| {
        cache::guardar(c, servicio_noticias::CLAVE_ELEGIDAS, &elegidas, ahora.momento + Duration::minutes(minutos), &ahora)
    });
    true
}

/// El mismo proveedor de IA que usa el asistente (con su key del llavero).
/// El llavero se lee sin tener tomada la base.
fn cliente_ia(db: &Db) -> Resultado<crate::asistente::openai::ClienteCompatible> {
    let ajustes = con_base(db, preferencias::ajustes_ia).unwrap_or_default();
    let key = match ajustes.proveedor.cuenta_llavero() {
        Some(cuenta) => llavero::leer(cuenta)?,
        None => None,
    };
    ajustes.cliente(key)
}

/// Clima: cada media hora, si está a la vista.
async fn clima(db: &Db, cliente: &ClienteClima) -> bool {
    let ahora = Ahora::del_sistema();
    let ajustes = con_base(db, preferencias::clima).unwrap_or_default();
    if !ajustes.mostrar || !vencida(db, servicio_clima::CLAVE, &ahora) {
        return false;
    }
    let resultado = cliente.pronostico(&ajustes).await;
    anotar(db, servicio_clima::CLAVE, &resultado, |_| ahora.momento + Duration::minutes(30), &ahora);
    true
}

/// Temporizadores vencidos (v0.7.1): los pediste vos, así que avisan aunque
/// sea "no molestar", y se dicen en voz alta si los avisos van en voz.
fn temporizadores(app: &AppHandle, db: &Db) {
    let ahora = Ahora::del_sistema();
    let Some(vencidos) = con_base(db, |c| repo_temporizadores::tomar_vencidos(c, &ahora)) else { return };
    if vencidos.is_empty() {
        return;
    }
    for t in &vencidos {
        let aviso = logica_avisos::Aviso { clave: format!("temporizador:{}", t.id), titulo: "Nova te avisa".into(), cuerpo: t.texto.clone() };
        crate::notificaciones::mostrar(app, &aviso);
    }
    let Some(ajustes) = con_base(db, preferencias::avisos) else { return };
    if ajustes.en_voz {
        let voz = app.state::<crate::voz::Voz>();
        let Some(ajustes_voz) = con_base(db, preferencias::voz) else { return };
        let texto = vencidos.iter().map(|t| format!("Te aviso: {}.", t.texto)).collect::<Vec<_>>().join(" ");
        voz.mandar(app, &ajustes_voz, crate::voz::motor::Orden::Hablar(texto));
    }
}

/// Avisos del sistema: lo que toca ahora y no se mandó (v0.6). Cada aviso se
/// anota antes de mostrarse, así un error no lo repite cada minuto.
fn avisos(app: &AppHandle, db: &Db) {
    let ahora = Ahora::del_sistema();
    let Some((ajustes, nuevos)) = con_base(db, |c| {
        let ajustes = preferencias::avisos(c)?;
        let pendientes = repo_avisos::pendientes(c, &ahora)?;
        let nuevos = logica_avisos::que_avisar(&pendientes, &ajustes, ahora.momento, |clave| {
            // Si no se puede leer, se toma como mandado: mejor uno de menos que repetidos.
            repo_avisos::ya_enviado(c, clave).unwrap_or(true)
        });
        for aviso in &nuevos {
            repo_avisos::anotar(c, &aviso.clave, &ahora)?;
        }
        repo_avisos::limpiar(c, &ahora)?;
        Ok((ajustes, nuevos))
    }) else {
        return;
    };
    for aviso in &nuevos {
        crate::notificaciones::mostrar(app, aviso);
    }
    if ajustes.en_voz && !nuevos.is_empty() {
        let voz = app.state::<crate::voz::Voz>();
        let Some(ajustes_voz) = con_base(db, preferencias::voz) else { return };
        let texto = nuevos.iter().map(logica_avisos::para_decir).collect::<Vec<_>>().join(" ");
        voz.mandar(app, &ajustes_voz, crate::voz::motor::Orden::Hablar(texto));
    }
}
