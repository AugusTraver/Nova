//! Lógica del widget de fútbol, sin red: qué partido es el próximo, cuál fue
//! el último, cuándo está en vivo, cada cuánto consultarlo, si Boca hizo un
//! gol y cuándo vence la caché.

use chrono::{DateTime, Datelike, Duration, FixedOffset, Timelike};
use serde::{Deserialize, Serialize};

use super::fechas_relativas::{nombre_dia_con_tilde, nombre_mes};
use super::refresco;
use crate::futbol::{EquipoSeguido, EstadoPartido, FilaTabla, Partido, Tabla, TipoEventoPartido};

/// El modo en vivo arranca un rato antes, por si el partido empieza antes.
pub const ANTES_DEL_ARRANQUE: i64 = 5; // minutos
/// Si después de esto sigue sin terminar (sin datos), se deja de consultar.
pub const DURACION_MAXIMA: i64 = 3 * 60 + 30; // minutos (con alargue y penales)
/// Cuándo se refresca el calendario después del arranque de un partido.
pub const REFRESCO_DESPUES_DEL_ARRANQUE: i64 = 2 * 60 + 30; // minutos

/// Tope propio de pedidos por día a ESPN. La cuenta de un día normal es de
/// unos 13 y un partido en vivo suma hasta 240 (SPEC 4.4).
pub const TOPE_DIARIO: u32 = 500;

fn fecha(partido: &Partido) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(&partido.fecha).ok()
}

/// El próximo partido que todavía no se jugó.
pub fn proximo(partidos: &[Partido], ahora: DateTime<FixedOffset>) -> Option<&Partido> {
    partidos
        .iter()
        .filter(|p| p.estado == EstadoPartido::Programado)
        // Uno "programado" que arrancó hace horas es un dato viejo, no el próximo.
        .filter(|p| fecha(p).is_some_and(|f| f + Duration::minutes(DURACION_MAXIMA) > ahora))
        .min_by(|a, b| a.fecha.cmp(&b.fecha))
}

/// El último partido terminado.
pub fn ultimo(partidos: &[Partido]) -> Option<&Partido> {
    partidos.iter().filter(|p| p.estado == EstadoPartido::Terminado).max_by(|a, b| a.fecha.cmp(&b.fecha))
}

/// ¿Este partido está (o debería estar) en juego ahora?
pub fn en_ventana(partido: &Partido, ahora: DateTime<FixedOffset>) -> bool {
    if partido.estado.en_curso() {
        return true;
    }
    if partido.estado != EstadoPartido::Programado {
        return false;
    }
    fecha(partido).is_some_and(|f| {
        ahora >= f - Duration::minutes(ANTES_DEL_ARRANQUE) && ahora < f + Duration::minutes(DURACION_MAXIMA)
    })
}

/// El partido que hay que seguir en vivo, si hay uno.
pub fn en_vivo(partidos: &[Partido], ahora: DateTime<FixedOffset>) -> Option<&Partido> {
    partidos.iter().find(|p| en_ventana(p, ahora))
}

/// Cada cuánto se consulta un partido en vivo: más seguido con el panel
/// abierto; en el entretiempo o esperando el arranque, menos.
pub fn intervalo_en_vivo(estado: EstadoPartido, panel_abierto: bool) -> Duration {
    match (estado, panel_abierto) {
        (EstadoPartido::Entretiempo, _) => Duration::seconds(120),
        (EstadoPartido::EnJuego, true) => Duration::seconds(30),
        _ => Duration::seconds(60),
    }
}

/// Cuándo vence el calendario de un equipo: a la mañana, o después del
/// próximo partido si es antes (para tener el resultado sin esperar).
pub fn vence_calendario(partidos: &[Partido], ahora: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let despues = proximo(partidos, ahora)
        .and_then(fecha)
        .map(|f| f + Duration::minutes(REFRESCO_DESPUES_DEL_ARRANQUE));
    refresco::lo_antes(ahora, despues)
}

/// Cuántos goles nuevos hizo `equipo_id` entre dos lecturas del mismo partido.
pub fn goles_nuevos(anterior: Option<&Partido>, nuevo: &Partido, equipo_id: &str) -> u32 {
    match anterior {
        Some(a) if a.id == nuevo.id && a.estado != EstadoPartido::Programado => {
            nuevo.goles_de(equipo_id).saturating_sub(a.goles_de(equipo_id))
        }
        // Sin lectura anterior no se festeja nada (puede ser un gol viejo).
        _ => 0,
    }
}

/// ¿Terminó entre dos lecturas? (para refrescar la tabla y el calendario).
pub fn termino(anterior: Option<&Partido>, nuevo: &Partido) -> bool {
    nuevo.estado == EstadoPartido::Terminado && anterior.is_none_or(|a| a.estado != EstadoPartido::Terminado)
}

/// La posición de un equipo en la tabla (con el nombre de la zona).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Posicion {
    pub torneo: String,
    pub zona: String,
    pub fila: FilaTabla,
    pub equipos_en_zona: u32,
}

pub fn posicion(tabla: &Tabla, equipo_id: &str) -> Option<Posicion> {
    tabla.grupos.iter().find_map(|g| {
        let fila = g.filas.iter().find(|f| f.equipo.id == equipo_id)?;
        Some(Posicion {
            torneo: tabla.torneo.clone(),
            zona: g.nombre.clone(),
            fila: fila.clone(),
            equipos_en_zona: g.filas.len() as u32,
        })
    })
}

/// Lo que ve el widget de un equipo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenEquipo {
    /// "boca" o "seleccion".
    pub clave: String,
    pub id: String,
    pub nombre: String,
    pub proximo: Option<Partido>,
    pub ultimo: Option<Partido>,
    pub en_vivo: Option<Partido>,
    pub posicion: Option<Posicion>,
}

/// Arma el resumen. `vivo` es la última lectura detallada del partido en
/// vivo: si es el mismo, pisa al del calendario (que tiene menos datos).
pub fn resumir(
    equipo: &EquipoSeguido,
    partidos: &[Partido],
    tabla: Option<&Tabla>,
    vivo: Option<&Partido>,
    ahora: DateTime<FixedOffset>,
) -> ResumenEquipo {
    let del_calendario = en_vivo(partidos, ahora);
    let en_vivo = match (del_calendario, vivo) {
        (Some(c), Some(v)) if c.id == v.id => {
            // Ya terminó según la lectura en vivo: queda como último.
            if v.estado.en_curso() || v.estado == EstadoPartido::Programado { Some(v.clone()) } else { None }
        }
        (Some(c), _) => Some(c.clone()),
        (None, Some(v)) if v.estado.en_curso() && v.juega(equipo.id) => Some(v.clone()),
        _ => None,
    };
    let terminado_en_vivo = vivo.filter(|v| v.estado == EstadoPartido::Terminado && v.juega(equipo.id));
    let ultimo = match (ultimo(partidos), terminado_en_vivo) {
        (Some(u), Some(v)) if v.fecha >= u.fecha => Some(v.clone()),
        (None, Some(v)) => Some(v.clone()),
        (u, _) => u.cloned(),
    };
    let en_vivo_id = en_vivo.as_ref().map(|p| p.id.clone());
    // El que se está jugando (o ya terminó en vivo) no es "el próximo".
    let resto: Vec<Partido> = partidos
        .iter()
        .filter(|p| Some(&p.id) != en_vivo_id.as_ref() && Some(&p.id) != ultimo.as_ref().map(|u| &u.id))
        .cloned()
        .collect();
    ResumenEquipo {
        clave: equipo.clave.into(),
        id: equipo.id.into(),
        nombre: equipo.nombre.into(),
        proximo: proximo(&resto, ahora).cloned(),
        ultimo,
        en_vivo,
        posicion: tabla.and_then(|t| posicion(t, equipo.id)),
    }
}

// ---------- Textos (para el asistente y el resumen del día) ----------

/// "hoy a las 19:30", "mañana a las 21:00", "el sábado 21 a las 19:30" o
/// "el sábado 5 de diciembre a las 19:30", en la zona de `ahora`.
pub fn cuando(fecha_iso: &str, ahora: DateTime<FixedOffset>) -> String {
    let Ok(f) = DateTime::parse_from_rfc3339(fecha_iso) else { return fecha_iso.to_string() };
    let f = f.with_timezone(&ahora.timezone());
    let hora = format!("{:02}:{:02}", f.hour(), f.minute());
    let dias = (f.date_naive() - ahora.date_naive()).num_days();
    let dia = nombre_dia_con_tilde(f.weekday());
    match dias {
        0 => format!("hoy a las {hora}"),
        1 => format!("mañana a las {hora}"),
        -1 => format!("ayer a las {hora}"),
        2..=6 => format!("el {dia} {} a las {hora}", f.day()),
        _ => format!("el {dia} {} de {} a las {hora}", f.day(), nombre_mes(f.month())),
    }
}

/// "Atlético-MG (de visitante, final de la Sudamericana)".
fn rival_y_competencia(p: &Partido, equipo_id: &str) -> String {
    let condicion = if p.local.id == equipo_id { "de local" } else { "de visitante" };
    let competencia = match &p.fase {
        Some(fase) => format!("{fase} de {}", con_articulo(&p.competencia)),
        None => con_articulo(&p.competencia),
    };
    format!("{} ({condicion}, {competencia})", p.rival_de(equipo_id).nombre)
}

fn con_articulo(competencia: &str) -> String {
    match competencia {
        "Libertadores" | "Sudamericana" | "Liga Profesional" | "Copa Argentina" | "Copa América" | "Recopa" => {
            format!("la {competencia}")
        }
        "Amistoso" => "amistoso".into(),
        "Mundial" | "Mundial de Clubes" | "Trofeo de Campeones" => format!("el {competencia}"),
        "Eliminatorias" => "las Eliminatorias".into(),
        otra => otra.into(),
    }
}

/// "3-0" desde el lado del equipo, con penales si hubo.
pub fn marcador(p: &Partido, equipo_id: &str) -> String {
    let (propios, rival) = (p.goles_de(equipo_id), p.rival_de(equipo_id));
    let goles_rival = p.goles_de(&rival.id);
    let mut texto = format!("{propios}-{goles_rival}");
    if let (Some(pl), Some(pv)) = (p.penales_local, p.penales_visitante) {
        let (pp, pr) = if p.local.id == equipo_id { (pl, pv) } else { (pv, pl) };
        texto.push_str(&format!(" ({pp}-{pr} en penales)"));
    }
    texto
}

pub fn describir_proximo(p: &Partido, equipo_id: &str, ahora: DateTime<FixedOffset>) -> String {
    format!("{} contra {}", cuando(&p.fecha, ahora), rival_y_competencia(p, equipo_id))
}

pub fn describir_ultimo(p: &Partido, equipo_id: &str, ahora: DateTime<FixedOffset>) -> String {
    let (propios, contra) = (p.goles_de(equipo_id), p.goles_de(&p.rival_de(equipo_id).id));
    let verbo = match (propios.cmp(&contra), p.penales_local.is_some()) {
        (std::cmp::Ordering::Equal, false) => "empató",
        (std::cmp::Ordering::Greater, _) => "ganó",
        (std::cmp::Ordering::Less, _) => "perdió",
        (std::cmp::Ordering::Equal, true) => {
            let (pl, pv) = (p.penales_local.unwrap_or(0), p.penales_visitante.unwrap_or(0));
            let gano = if p.local.id == equipo_id { pl > pv } else { pv > pl };
            if gano { "ganó por penales" } else { "perdió por penales" }
        }
    };
    let goleadores: Vec<String> = p
        .eventos
        .iter()
        .filter(|e| e.equipo_id == equipo_id && e.tipo != TipoEventoPartido::Expulsion)
        .map(|e| match (&e.jugador, e.tipo) {
            (Some(j), TipoEventoPartido::GolEnContra) => format!("{j} en contra {}", e.minuto),
            (Some(j), _) => format!("{j} {}", e.minuto),
            (None, _) => e.minuto.clone(),
        })
        .collect();
    let mut texto =
        format!("{verbo} {} contra {}, {}", marcador(p, equipo_id), rival_y_competencia(p, equipo_id), cuando(&p.fecha, ahora));
    if !goleadores.is_empty() {
        texto.push_str(&format!(". Goles: {}", goleadores.join(", ")));
    }
    texto
}

pub fn describir_en_vivo(p: &Partido, equipo_id: &str) -> String {
    let minuto = p.minuto.clone().unwrap_or_else(|| "por empezar".into());
    format!("en vivo ({minuto}): {} contra {}", marcador(p, equipo_id), rival_y_competencia(p, equipo_id))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::futbol::{BOCA, EquipoBreve, EventoPartido, GrupoTabla};

    fn t(texto: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(texto).unwrap()
    }

    fn equipo(id: &str, nombre: &str) -> EquipoBreve {
        EquipoBreve { id: id.into(), nombre: nombre.into(), abreviatura: nombre.into(), escudo: None }
    }

    /// Un partido de Boca (local) contra Racing.
    pub fn partido(id: &str, fecha: &str, estado: EstadoPartido, goles: Option<(u32, u32)>) -> Partido {
        Partido {
            id: id.into(),
            liga: "arg.1".into(),
            competencia: "Liga Profesional".into(),
            fase: None,
            fecha: fecha.into(),
            local: equipo("5", "Boca Juniors"),
            visitante: equipo("15", "Racing Club"),
            estado,
            minuto: None,
            goles_local: goles.map(|g| g.0),
            goles_visitante: goles.map(|g| g.1),
            penales_local: None,
            penales_visitante: None,
            eventos: vec![],
        }
    }

    fn calendario() -> Vec<Partido> {
        vec![
            partido("1", "2026-11-10T23:00:00Z", EstadoPartido::Terminado, Some((2, 1))),
            partido("2", "2026-11-14T22:00:00Z", EstadoPartido::Terminado, Some((0, 0))),
            partido("3", "2026-11-21T22:30:00Z", EstadoPartido::Programado, None),
            partido("4", "2026-11-26T00:00:00Z", EstadoPartido::Programado, None),
        ]
    }

    #[test]
    fn proximo_y_ultimo() {
        let ahora = t("2026-11-17T14:00:00-03:00");
        let p = calendario();
        assert_eq!(proximo(&p, ahora).unwrap().id, "3");
        assert_eq!(ultimo(&p).unwrap().id, "2");
        // Un programado que arrancó hace más de 3 h y media ya no es el próximo.
        assert_eq!(proximo(&p, t("2026-11-22T00:00:00-03:00")).unwrap().id, "4");
    }

    #[test]
    fn la_ventana_en_vivo() {
        let p = partido("3", "2026-11-21T22:30:00Z", EstadoPartido::Programado, None); // 19:30 en Argentina
        assert!(!en_ventana(&p, t("2026-11-21T19:24:00-03:00")));
        assert!(en_ventana(&p, t("2026-11-21T19:25:00-03:00")));
        assert!(en_ventana(&p, t("2026-11-21T22:59:00-03:00")));
        assert!(!en_ventana(&p, t("2026-11-21T23:00:00-03:00")));
        // En juego cuenta siempre; terminado, nunca.
        let mut vivo = p.clone();
        vivo.estado = EstadoPartido::EnJuego;
        assert!(en_ventana(&vivo, t("2026-11-22T03:00:00-03:00")));
        vivo.estado = EstadoPartido::Terminado;
        assert!(!en_ventana(&vivo, t("2026-11-21T20:00:00-03:00")));
    }

    #[test]
    fn el_intervalo_cuida_el_limite() {
        assert_eq!(intervalo_en_vivo(EstadoPartido::EnJuego, true), Duration::seconds(30));
        assert_eq!(intervalo_en_vivo(EstadoPartido::EnJuego, false), Duration::seconds(60));
        assert_eq!(intervalo_en_vivo(EstadoPartido::Entretiempo, true), Duration::seconds(120));
        assert_eq!(intervalo_en_vivo(EstadoPartido::Programado, true), Duration::seconds(60));
        // Peor caso: 3 h 35 min consultando cada 30 s = 430 pedidos, debajo del tope.
        let peor = (DURACION_MAXIMA + ANTES_DEL_ARRANQUE) * 60 / 30;
        assert!(peor as u32 + 20 < TOPE_DIARIO);
    }

    #[test]
    fn el_calendario_vence_despues_del_proximo_partido_o_a_la_manana() {
        let p = calendario();
        // El sábado a las 14: vence después del partido de las 19:30.
        assert_eq!(vence_calendario(&p, t("2026-11-21T14:00:00-03:00")), t("2026-11-21T22:00:00-03:00"));
        // Un martes: vence a la mañana siguiente.
        assert_eq!(vence_calendario(&p, t("2026-11-17T14:00:00-03:00")), t("2026-11-18T06:00:00-03:00"));
    }

    #[test]
    fn detecta_goles_nuevos_y_el_final() {
        let mut antes = partido("3", "2026-11-21T22:30:00Z", EstadoPartido::EnJuego, Some((0, 0)));
        let mut despues = antes.clone();
        despues.goles_local = Some(1);
        assert_eq!(goles_nuevos(Some(&antes), &despues, "5"), 1);
        assert_eq!(goles_nuevos(Some(&antes), &despues, "15"), 0);
        // Sin lectura anterior, o de otro partido, no se festeja.
        assert_eq!(goles_nuevos(None, &despues, "5"), 0);
        antes.id = "otro".into();
        assert_eq!(goles_nuevos(Some(&antes), &despues, "5"), 0);
        // Un gol anulado no da negativo.
        antes.id = "3".into();
        antes.goles_local = Some(2);
        assert_eq!(goles_nuevos(Some(&antes), &despues, "5"), 0);

        assert!(!termino(Some(&antes), &despues));
        despues.estado = EstadoPartido::Terminado;
        assert!(termino(Some(&antes), &despues));
        assert!(!termino(Some(&despues), &despues));
    }

    #[test]
    fn el_resumen_usa_la_lectura_en_vivo() {
        let ahora = t("2026-11-21T20:10:00-03:00");
        let p = calendario();
        let mut vivo = partido("3", "2026-11-21T22:30:00Z", EstadoPartido::EnJuego, Some((1, 0)));
        vivo.minuto = Some("38'".into());
        let tabla = Tabla {
            torneo: "Liga Profesional".into(),
            grupos: vec![GrupoTabla {
                nombre: "Zona A".into(),
                filas: vec![FilaTabla {
                    puesto: 2, equipo: equipo("5", "Boca Juniors"), puntos: 33, jugados: 16, ganados: 9,
                    empatados: 6, perdidos: 1, diferencia: 10,
                }],
            }],
        };
        let r = resumir(&BOCA, &p, Some(&tabla), Some(&vivo), ahora);
        assert_eq!(r.en_vivo.as_ref().unwrap().minuto.as_deref(), Some("38'"));
        assert_eq!(r.proximo.unwrap().id, "4", "el que se juega no figura como próximo");
        assert_eq!(r.ultimo.unwrap().id, "2");
        assert_eq!(r.posicion.unwrap().fila.puesto, 2);

        // Terminó: deja de estar en vivo y pasa a ser el último.
        vivo.estado = EstadoPartido::Terminado;
        let r = resumir(&BOCA, &p, None, Some(&vivo), t("2026-11-21T21:30:00-03:00"));
        assert_eq!(r.en_vivo, None);
        assert_eq!(r.ultimo.unwrap().goles_local, Some(1));
    }

    #[test]
    fn los_textos_para_el_asistente() {
        let ahora = t("2026-11-17T14:00:00-03:00");
        let mut p = partido("3", "2026-11-21T22:30:00Z", EstadoPartido::Programado, None);
        p.competencia = "Sudamericana".into();
        p.fase = Some("final".into());
        std::mem::swap(&mut p.local, &mut p.visitante);
        p.local = equipo("7632", "Atlético-MG");
        assert_eq!(
            describir_proximo(&p, "5", ahora),
            "el sábado 21 a las 19:30 contra Atlético-MG (de visitante, final de la Sudamericana)"
        );
        assert_eq!(cuando("2026-11-17T23:00:00Z", ahora), "hoy a las 20:00");
        assert_eq!(cuando("2026-11-18T23:00:00Z", ahora), "mañana a las 20:00");
        assert_eq!(cuando("2026-12-05T23:00:00Z", ahora), "el sábado 5 de diciembre a las 20:00");

        let mut u = partido("2", "2026-11-14T22:00:00Z", EstadoPartido::Terminado, Some((3, 0)));
        u.eventos = vec![
            EventoPartido { minuto: "45'+1'".into(), tipo: TipoEventoPartido::Gol, equipo_id: "5".into(), jugador: Some("Ascacibar".into()) },
            EventoPartido { minuto: "80'".into(), tipo: TipoEventoPartido::Expulsion, equipo_id: "15".into(), jugador: Some("X".into()) },
        ];
        assert_eq!(
            describir_ultimo(&u, "5", ahora),
            "ganó 3-0 contra Racing Club (de local, la Liga Profesional), el sábado 14 de noviembre a las 19:00. Goles: Ascacibar 45'+1'"
        );
        u.goles_local = Some(1);
        u.goles_visitante = Some(1);
        u.penales_local = Some(4);
        u.penales_visitante = Some(5);
        assert!(describir_ultimo(&u, "5", ahora).starts_with("perdió por penales 1-1 (4-5 en penales)"));

        let mut v = partido("3", "2026-11-21T22:30:00Z", EstadoPartido::EnJuego, Some((1, 0)));
        v.minuto = Some("38'".into());
        assert_eq!(describir_en_vivo(&v, "5"), "en vivo (38'): 1-0 contra Racing Club (de local, la Liga Profesional)");
    }
}
