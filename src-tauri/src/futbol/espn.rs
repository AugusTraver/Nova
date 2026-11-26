//! `ProveedorFutbol` con la API pública de ESPN (la que usa su sitio).
//!
//! No pide key y tiene la temporada actual de la Liga Profesional, la Copa
//! Argentina, la Conmebol, la Selección y las ligas europeas. No está
//! documentada: si cambia, se rompe solo la lectura de este archivo, y el
//! widget sigue mostrando lo último guardado.
//!
//! Igual que `asistente/openai.rs`: armar las direcciones y leer las
//! respuestas son funciones puras (con tests sobre JSON reales recortados, en
//! `ejemplos/`); la única parte que toca la red es `ClienteEspn::pedir`.

use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDateTime, SecondsFormat, Utc};
use serde_json::Value;

use super::proveedor::ProveedorFutbol;
use super::{
    EquipoBreve, EstadoJugador, EstadoPartido, EventoPartido, FilaTabla, GrupoTabla, JugadorEncontrado, Partido,
    PartidoJugador, Tabla, TipoEventoPartido,
};
use crate::error::{Error, Resultado};

const SITIO: &str = "https://site.api.espn.com/apis/site/v2/sports/soccer";
const TABLAS: &str = "https://site.api.espn.com/apis/v2/sports/soccer";
const COMUN: &str = "https://site.web.api.espn.com/apis/common/v3/sports/soccer";
const BUSQUEDA: &str = "https://site.web.api.espn.com/apis/search/v2";

// ---------- Direcciones ----------

/// "all" = todas las competencias del equipo (liga, copas, Conmebol).
///
/// Va con la temporada (el año) porque, sin ella, ESPN elige la de la
/// próxima competencia que tenga el equipo: en noviembre de 2026 Boca ya
/// tenía la Recopa 2027 y devolvía cero partidos jugados.
pub fn url_jugados(equipo_id: &str, temporada: i32) -> String {
    format!("{SITIO}/all/teams/{equipo_id}/schedule?season={temporada}")
}

pub fn url_proximos(equipo_id: &str) -> String {
    format!("{SITIO}/all/teams/{equipo_id}/schedule?fixture=true")
}

pub fn url_partido(partido_id: &str) -> String {
    format!("{SITIO}/all/summary?event={partido_id}")
}

pub fn url_tabla(liga: &str) -> String {
    format!("{TABLAS}/{liga}/standings")
}

/// `liga`: para pedir una en particular cuando la que elige ESPN viene vacía
/// (ver `otra_liga_del_jugador`).
pub fn url_jugador(jugador_id: &str, liga: Option<&str>) -> String {
    match liga {
        Some(liga) => format!("{COMUN}/athletes/{jugador_id}/gamelog?league={liga}"),
        None => format!("{COMUN}/athletes/{jugador_id}/gamelog"),
    }
}

/// Si el gamelog vino sin partidos, la primera de las otras ligas que ESPN
/// ofrece para ese jugador (la de su club suele ir primero). Pasa cuando ESPN
/// elige una copa que todavía no se jugó, como la Recopa del año que viene.
pub fn otra_liga_del_jugador(json: &Value) -> Option<String> {
    let tiene_partidos = json.get("events").and_then(Value::as_object).is_some_and(|e| !e.is_empty());
    if tiene_partidos {
        return None;
    }
    let filtro = json.get("filters")?.as_array()?.iter().find(|f| texto(f, "name").as_deref() == Some("league"))?;
    let elegida = texto(filtro, "value");
    filtro
        .get("options")?
        .as_array()?
        .iter()
        .filter_map(|o| texto(o, "value"))
        .find(|liga| Some(liga) != elegida.as_ref())
}

/// Si en la temporada de este año todavía no hay nada jugado (enero), se
/// piden también los de la anterior, para mostrar el último resultado.
pub fn falta_la_temporada_anterior(jugados: &[Partido]) -> bool {
    !jugados.iter().any(|p| p.estado != EstadoPartido::Programado)
}

pub fn url_busqueda(texto: &str) -> String {
    // Se codifica a mano lo poco que hace falta para un nombre.
    let texto: String = texto
        .trim()
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            c if c.is_alphanumeric() || c == '-' || c == '.' => c.to_string(),
            _ => String::new(),
        })
        .collect();
    format!("{BUSQUEDA}?query={texto}&limit=8&type=player")
}

// ---------- Lectura ----------

/// Atajo para leer un texto de un JSON: `texto(v, "id")`.
fn texto(valor: &Value, clave: &str) -> Option<String> {
    match valor.get(clave)? {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Los números vienen de muchas formas: 3, 3.0, "3" o `{ "value": 3 }`.
fn numero(valor: Option<&Value>) -> Option<f64> {
    match valor? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Object(o) => numero(o.get("value")),
        _ => None,
    }
}

fn entero(valor: Option<&Value>) -> Option<u32> {
    numero(valor).filter(|n| *n >= 0.0).map(|n| n.round() as u32)
}

/// "2026-11-21T22:30Z" (sin segundos) o un RFC 3339 completo → RFC 3339 UTC.
pub fn normalizar_fecha(texto: &str) -> Option<String> {
    let fecha = DateTime::parse_from_rfc3339(texto)
        .map(|f| f.with_timezone(&Utc))
        .or_else(|_| NaiveDateTime::parse_from_str(texto, "%Y-%m-%dT%H:%MZ").map(|f| f.and_utc()))
        .ok()?;
    Some(fecha.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// El nombre en castellano de una competencia.
pub fn competencia(slug: Option<&str>, nombre: Option<&str>) -> String {
    let por_slug = match slug.unwrap_or("") {
        "arg.1" => Some("Liga Profesional"),
        "arg.copa" => Some("Copa Argentina"),
        "arg.supercopa" | "arg.trofeo" => Some("Trofeo de Campeones"),
        "conmebol.libertadores" => Some("Libertadores"),
        "conmebol.sudamericana" => Some("Sudamericana"),
        "conmebol.recopa" => Some("Recopa"),
        "conmebol.america" => Some("Copa América"),
        "fifa.world" => Some("Mundial"),
        "fifa.cwc" => Some("Mundial de Clubes"),
        "fifa.friendly" | "club.friendly" => Some("Amistoso"),
        s if s.starts_with("fifa.worldq") => Some("Eliminatorias"),
        _ => None,
    };
    if let Some(n) = por_slug {
        return n.to_string();
    }
    let nombre = nombre.unwrap_or("").trim();
    let por_nombre = [
        ("Liga Profesional", "Liga Profesional"),
        ("Copa Argentina", "Copa Argentina"),
        ("Libertadores", "Libertadores"),
        ("Sudamericana", "Sudamericana"),
        ("Friendly", "Amistoso"),
        ("World Cup Qualifying", "Eliminatorias"),
        ("WCQ", "Eliminatorias"),
        ("World Cup", "Mundial"),
        ("Copa América", "Copa América"),
        ("Champions League", "Champions"),
        ("Brazilian Serie A", "Brasileirão"),
    ];
    por_nombre
        .iter()
        .find(|(clave, _)| nombre.contains(clave))
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| nombre.to_string())
}

/// La fase de una copa ("Clausura - Round of 16" → "octavos de final").
pub fn fase(nombre: &str) -> Option<String> {
    let fases = [
        ("Round of 32", "dieciseisavos de final"),
        ("Round of 16", "octavos de final"),
        ("Quarterfinal", "cuartos de final"),
        ("Semifinal", "semifinal"),
        ("Final", "final"),
    ];
    fases.iter().find(|(clave, _)| nombre.contains(clave)).map(|(_, f)| f.to_string())
}

fn equipo(valor: &Value) -> Option<EquipoBreve> {
    let id = texto(valor, "id")?;
    let nombre = texto(valor, "displayName").or_else(|| texto(valor, "name"))?;
    let escudo = valor
        .get("logos")
        .and_then(|l| l.get(0))
        .and_then(|l| texto(l, "href"))
        .or_else(|| texto(valor, "logo"));
    Some(EquipoBreve { abreviatura: texto(valor, "abbreviation").unwrap_or_else(|| nombre.clone()), id, nombre, escudo })
}

/// El estado y el minuto, desde `status`.
fn estado(status: &Value) -> (EstadoPartido, Option<String>) {
    let tipo = status.get("type").unwrap_or(&Value::Null);
    let nombre = texto(tipo, "name").unwrap_or_default();
    let fase = texto(tipo, "state").unwrap_or_default();
    let estado = match (fase.as_str(), nombre.as_str()) {
        (_, "STATUS_HALFTIME") => EstadoPartido::Entretiempo,
        (_, "STATUS_POSTPONED" | "STATUS_CANCELED" | "STATUS_SUSPENDED" | "STATUS_ABANDONED" | "STATUS_FORFEIT") => {
            EstadoPartido::Suspendido
        }
        ("in", _) => EstadoPartido::EnJuego,
        ("post", _) => EstadoPartido::Terminado,
        _ => EstadoPartido::Programado,
    };
    let minuto = match estado {
        EstadoPartido::EnJuego => texto(status, "displayClock"),
        EstadoPartido::Entretiempo => Some("Entretiempo".into()),
        _ => None,
    };
    (estado, minuto)
}

/// Arma un partido desde una "competition" de ESPN. `liga` y `fase_texto`
/// vienen del evento que la contiene.
fn partido_de(id: String, liga: Option<&Value>, fase_texto: Option<String>, comp: &Value) -> Option<Partido> {
    let competidores = comp.get("competitors")?.as_array()?;
    let lado = |cual: &str| competidores.iter().find(|c| texto(c, "homeAway").as_deref() == Some(cual));
    let (local, visitante) = (lado("home")?, lado("away")?);
    let (estado, minuto) = estado(comp.get("status").unwrap_or(&Value::Null));
    let hay_marcador = estado != EstadoPartido::Programado;
    let goles = |c: &Value| if hay_marcador { entero(c.get("score")) } else { None };
    let liga = liga.unwrap_or(&Value::Null);
    let slug = texto(liga, "slug");
    Some(Partido {
        id,
        competencia: competencia(slug.as_deref(), texto(liga, "name").as_deref()),
        liga: slug.unwrap_or_else(|| "all".into()),
        fase: fase_texto.as_deref().and_then(fase),
        fecha: normalizar_fecha(&texto(comp, "date")?)?,
        goles_local: goles(local),
        goles_visitante: goles(visitante),
        penales_local: entero(local.get("shootoutScore")),
        penales_visitante: entero(visitante.get("shootoutScore")),
        local: equipo(local.get("team")?)?,
        visitante: equipo(visitante.get("team")?)?,
        estado,
        minuto,
        eventos: Vec::new(),
    })
}

/// Un evento de la lista de partidos de un equipo.
fn partido_de_evento(evento: &Value) -> Option<Partido> {
    let comp = evento.get("competitions")?.get(0)?;
    let fase_texto = evento.get("seasonType").and_then(|s| texto(s, "name"));
    partido_de(texto(evento, "id")?, evento.get("league"), fase_texto, comp)
}

/// `/teams/{id}/schedule`: los partidos que se pudieron leer, por fecha.
pub fn leer_calendario(json: &Value) -> Vec<Partido> {
    let mut partidos: Vec<Partido> = json
        .get("events")
        .and_then(Value::as_array)
        .map(|eventos| eventos.iter().filter_map(partido_de_evento).collect())
        .unwrap_or_default();
    partidos.sort_by(|a, b| a.fecha.cmp(&b.fecha));
    partidos
}

/// Junta jugados y próximos sin repetir (un partido puede estar en las dos).
pub fn unir_calendarios(jugados: Vec<Partido>, proximos: Vec<Partido>) -> Vec<Partido> {
    let mut todos = jugados;
    for p in proximos {
        if !todos.iter().any(|t| t.id == p.id) {
            todos.push(p);
        }
    }
    todos.sort_by(|a, b| a.fecha.cmp(&b.fecha));
    todos
}

/// `/summary?event=`: el partido con sus goles y expulsiones.
pub fn leer_partido(json: &Value) -> Option<Partido> {
    let cabecera = json.get("header")?;
    let comp = cabecera.get("competitions")?.get(0)?;
    let mut partido = partido_de(texto(cabecera, "id")?, cabecera.get("league"), None, comp)?;
    partido.eventos = json
        .get("keyEvents")
        .and_then(Value::as_array)
        .map(|eventos| eventos.iter().filter_map(evento_clave).collect())
        .unwrap_or_default();
    Some(partido)
}

fn evento_clave(evento: &Value) -> Option<EventoPartido> {
    let tipo = texto(evento.get("type")?, "type").unwrap_or_default();
    let es_gol = evento.get("scoringPlay").and_then(Value::as_bool).unwrap_or(false);
    let tipo = if es_gol && tipo.contains("own-goal") {
        TipoEventoPartido::GolEnContra
    } else if es_gol {
        TipoEventoPartido::Gol
    } else if tipo.contains("red-card") {
        TipoEventoPartido::Expulsion
    } else {
        return None;
    };
    Some(EventoPartido {
        minuto: evento.get("clock").and_then(|c| texto(c, "displayValue")).unwrap_or_default(),
        tipo,
        equipo_id: texto(evento.get("team")?, "id")?,
        jugador: evento
            .get("participants")
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("athlete"))
            .and_then(|a| texto(a, "displayName")),
    })
}

/// "Group A" → "Zona A".
fn nombre_grupo(nombre: &str) -> String {
    nombre.replace("Group", "Zona")
}

/// `/standings`: una tabla por zona.
pub fn leer_tabla(json: &Value) -> Option<Tabla> {
    let fila = |entrada: &Value| -> Option<FilaTabla> {
        let stats = entrada.get("stats")?.as_array()?;
        let stat = |nombre: &str| numero(stats.iter().find(|s| texto(s, "name").as_deref() == Some(nombre)));
        let n = |nombre: &str| stat(nombre).map(|v| v.max(0.0).round() as u32).unwrap_or(0);
        Some(FilaTabla {
            puesto: n("rank"),
            equipo: equipo(entrada.get("team")?)?,
            puntos: n("points"),
            jugados: n("gamesPlayed"),
            ganados: n("wins"),
            empatados: n("ties"),
            perdidos: n("losses"),
            diferencia: stat("pointDifferential").map(|v| v.round() as i32).unwrap_or(0),
        })
    };
    let grupo = |valor: &Value, nombre: String| -> Option<GrupoTabla> {
        let entradas = valor.get("standings")?.get("entries")?.as_array()?;
        let mut filas: Vec<FilaTabla> = entradas.iter().filter_map(fila).collect();
        filas.sort_by_key(|f| f.puesto);
        Some(GrupoTabla { nombre, filas })
    };
    // Con zonas vienen en `children`; una tabla única viene en la raíz.
    let grupos: Vec<GrupoTabla> = match json.get("children").and_then(Value::as_array) {
        Some(hijos) => hijos
            .iter()
            .filter_map(|h| grupo(h, nombre_grupo(&texto(h, "name").unwrap_or_default())))
            .collect(),
        None => grupo(json, String::new()).into_iter().collect(),
    };
    if grupos.is_empty() {
        return None;
    }
    Some(Tabla { torneo: competencia(None, texto(json, "name").as_deref()), grupos })
}

/// `/athletes/{id}/gamelog`: club, último partido y totales de la temporada.
pub fn leer_jugador(json: &Value, id: &str, nombre: &str) -> Option<EstadoJugador> {
    let eventos = json.get("events")?.as_object()?;
    let nombres: Vec<String> = json
        .get("names")
        .and_then(Value::as_array)
        .map(|n| n.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let columna = |nombre: &str| nombres.iter().position(|n| n == nombre);
    let (col_goles, col_asist) = (columna("totalGoals"), columna("goalAssists"));

    // Las estadísticas de cada partido están en otra parte, por id de evento.
    let mut por_evento: Vec<(String, u32, u32)> = Vec::new();
    for tipo in json.get("seasonTypes").and_then(Value::as_array).into_iter().flatten() {
        for categoria in tipo.get("categories").and_then(Value::as_array).into_iter().flatten() {
            for ev in categoria.get("events").and_then(Value::as_array).into_iter().flatten() {
                let Some(ev_id) = texto(ev, "eventId") else { continue };
                let stats = ev.get("stats").and_then(Value::as_array);
                let valor = |col: Option<usize>| entero(col.and_then(|c| stats?.get(c))).unwrap_or(0);
                if !por_evento.iter().any(|(e, ..)| *e == ev_id) {
                    por_evento.push((ev_id, valor(col_goles), valor(col_asist)));
                }
            }
        }
    }

    let ultimo = eventos
        .values()
        .filter_map(|e| Some((normalizar_fecha(&texto(e, "gameDate")?)?, e)))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(fecha, e)| {
            let equipo_id = e.get("team").and_then(|t| texto(t, "id")).unwrap_or_default();
            let (local, visita) = (texto(e, "homeTeamScore").unwrap_or_default(), texto(e, "awayTeamScore").unwrap_or_default());
            let resultado = if texto(e, "awayTeamId").as_deref() == Some(equipo_id.as_str()) {
                format!("{visita}-{local}")
            } else {
                format!("{local}-{visita}")
            };
            let id_evento = texto(e, "id").unwrap_or_default();
            let (goles, asistencias) = por_evento
                .iter()
                .find(|(ev, ..)| *ev == id_evento)
                .map(|(_, g, a)| (*g, *a))
                .unwrap_or((0, 0));
            PartidoJugador {
                fecha,
                rival: e.get("opponent").and_then(|o| texto(o, "displayName")).unwrap_or_default(),
                resultado,
                desenlace: texto(e, "gameResult").and_then(|r| match r.as_str() {
                    "W" => Some("G".to_string()),
                    "D" => Some("E".to_string()),
                    "L" => Some("P".to_string()),
                    _ => None,
                }),
                competencia: texto(e, "leagueName").map(|n| competencia(None, Some(&n))),
                goles,
                asistencias,
            }
        });

    // El club: el equipo elegido en el filtro "team" del gamelog.
    let club = json.get("filters").and_then(Value::as_array).and_then(|filtros| {
        let filtro = filtros.iter().find(|f| texto(f, "name").as_deref() == Some("team"))?;
        let elegido = texto(filtro, "value")?;
        let opciones = filtro.get("options")?.as_array()?;
        opciones.iter().find(|o| texto(o, "value").as_deref() == Some(elegido.as_str())).and_then(|o| texto(o, "displayValue"))
    });

    Some(EstadoJugador {
        id: id.to_string(),
        nombre: nombre.to_string(),
        club,
        ultimo,
        partidos: por_evento.len() as u32,
        goles: por_evento.iter().map(|(_, g, _)| g).sum(),
        asistencias: por_evento.iter().map(|(_, _, a)| a).sum(),
    })
}

/// La búsqueda de ESPN mezcla deportes: solo quedan los de fútbol
/// (`uid` "s:600~a:{id}").
pub fn leer_busqueda(json: &Value) -> Vec<JugadorEncontrado> {
    json.get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|r| texto(r, "type").as_deref() == Some("player"))
        .flat_map(|r| r.get("contents").and_then(Value::as_array).cloned().unwrap_or_default())
        .filter_map(|c| {
            let id = texto(&c, "uid")?.strip_prefix("s:600~a:")?.to_string();
            Some(JugadorEncontrado { id, nombre: texto(&c, "displayName")?, club: texto(&c, "subtitle") })
        })
        .collect()
}

// ---------- Red ----------

pub struct ClienteEspn {
    http: reqwest::Client,
}

impl ClienteEspn {
    pub fn nuevo() -> Resultado<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(8))
            .user_agent("Nova/0.4 (app de escritorio personal)")
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteEspn { http })
    }

    async fn pedir(&self, url: &str) -> Resultado<Value> {
        let sin_conexion = |_| Error::Sistema("Sin conexión con ESPN.".into());
        let respuesta = self.http.get(url).send().await.map_err(sin_conexion)?;
        let estado = respuesta.status().as_u16();
        if estado >= 400 {
            return Err(Error::Sistema(format!("ESPN respondió con un error ({estado}).")));
        }
        respuesta.json::<Value>().await.map_err(|_| Error::Sistema("ESPN respondió algo que no entiendo.".into()))
    }
}

fn no_entiendo() -> Error {
    Error::Sistema("ESPN respondió algo que no entiendo.".into())
}

impl ProveedorFutbol for ClienteEspn {
    async fn calendario(&self, equipo_id: &str) -> Resultado<Vec<Partido>> {
        let anio = chrono::Local::now().year();
        let mut jugados = leer_calendario(&self.pedir(&url_jugados(equipo_id, anio)).await?);
        if falta_la_temporada_anterior(&jugados) {
            let anteriores = leer_calendario(&self.pedir(&url_jugados(equipo_id, anio - 1)).await?);
            jugados = unir_calendarios(anteriores, jugados);
        }
        let proximos = leer_calendario(&self.pedir(&url_proximos(equipo_id)).await?);
        Ok(unir_calendarios(jugados, proximos))
    }

    async fn partido(&self, partido_id: &str) -> Resultado<Partido> {
        leer_partido(&self.pedir(&url_partido(partido_id)).await?).ok_or_else(no_entiendo)
    }

    async fn tabla(&self, liga: &str) -> Resultado<Tabla> {
        leer_tabla(&self.pedir(&url_tabla(liga)).await?).ok_or_else(no_entiendo)
    }

    async fn jugador(&self, jugador_id: &str, nombre: &str) -> Resultado<EstadoJugador> {
        let mut json = self.pedir(&url_jugador(jugador_id, None)).await?;
        if let Some(liga) = otra_liga_del_jugador(&json) {
            json = self.pedir(&url_jugador(jugador_id, Some(&liga))).await?;
        }
        leer_jugador(&json, jugador_id, nombre).ok_or_else(no_entiendo)
    }

    async fn buscar_jugadores(&self, texto: &str) -> Resultado<Vec<JugadorEncontrado>> {
        Ok(leer_busqueda(&self.pedir(&url_busqueda(texto)).await?))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn ejemplo(texto: &str) -> Value {
        serde_json::from_str(texto).unwrap()
    }

    #[test]
    fn las_direcciones() {
        assert_eq!(url_jugados("5", 2026), "https://site.api.espn.com/apis/site/v2/sports/soccer/all/teams/5/schedule?season=2026");
        assert!(url_jugador("400359", None).ends_with("/athletes/400359/gamelog"));
        assert!(url_jugador("400359", Some("arg.1")).ends_with("/athletes/400359/gamelog?league=arg.1"));
        assert!(url_proximos("202").ends_with("/all/teams/202/schedule?fixture=true"));
        assert!(url_partido("401926286").ends_with("/all/summary?event=401926286"));
        assert_eq!(url_tabla("arg.1"), "https://site.api.espn.com/apis/v2/sports/soccer/arg.1/standings");
        assert!(url_busqueda(" Tomás Aranda&x ").ends_with("?query=Tomás%20Arandax&limit=8&type=player"));
    }

    #[test]
    fn lee_los_proximos_de_boca() {
        let partidos = leer_calendario(&ejemplo(include_str!("ejemplos/fixture_boca.json")));
        assert_eq!(partidos.len(), 3);
        let p = &partidos[0];
        assert_eq!(p.competencia, "Sudamericana");
        assert_eq!(p.fase.as_deref(), Some("final"));
        assert_eq!(p.fecha, "2026-11-21T22:30:00Z");
        assert_eq!(p.estado, EstadoPartido::Programado);
        assert_eq!((p.local.nombre.as_str(), p.visitante.nombre.as_str()), ("Atlético-MG", "Boca Juniors"));
        assert_eq!((p.goles_local, p.goles_visitante), (None, None));
        assert_eq!(partidos[1].competencia, "Liga Profesional");
        assert_eq!(partidos[2].competencia, "Copa Argentina");
    }

    #[test]
    fn lee_los_jugados_con_marcador() {
        let partidos = leer_calendario(&ejemplo(include_str!("ejemplos/calendario_boca.json")));
        let ultimo = partidos.last().unwrap();
        assert_eq!(ultimo.estado, EstadoPartido::Terminado);
        assert_eq!(ultimo.competencia, "Copa Argentina");
        assert_eq!(ultimo.local.nombre, "Banfield");
        assert_eq!((ultimo.goles_local, ultimo.goles_visitante), (Some(0), Some(3)));
        assert_eq!(ultimo.goles_de("5"), 3);
        assert_eq!(ultimo.rival_de("5").nombre, "Banfield");
        // Ordenados por fecha.
        assert!(partidos.windows(2).all(|par| par[0].fecha <= par[1].fecha));
    }

    #[test]
    fn lee_el_partido_con_goles_y_sin_amarillas() {
        let p = leer_partido(&ejemplo(include_str!("ejemplos/resumen_partido.json"))).unwrap();
        assert_eq!(p.id, "401926286");
        assert_eq!(p.liga, "arg.copa");
        assert_eq!((p.goles_local, p.goles_visitante), (Some(0), Some(3)));
        let goles: Vec<_> = p.eventos.iter().filter(|e| e.tipo == TipoEventoPartido::Gol).collect();
        assert_eq!(goles.len(), 3);
        assert_eq!(goles[0].minuto, "45'+1'");
        assert_eq!(goles[0].jugador.as_deref(), Some("Santiago Ascacibar"));
        assert!(goles.iter().all(|g| g.equipo_id == "5"));
        assert_eq!(p.eventos.len(), 3, "las amarillas no se guardan");
    }

    #[test]
    fn el_estado_en_vivo_entretiempo_y_suspendido() {
        let comp = |nombre: &str, estado: &str| {
            json!({
                "date": "2026-11-21T22:30Z",
                "status": { "displayClock": "34'", "type": { "name": nombre, "state": estado } },
                "competitors": [
                    { "homeAway": "home", "score": "1", "team": { "id": "5", "displayName": "Boca Juniors" } },
                    { "homeAway": "away", "score": "0", "team": { "id": "7", "displayName": "Otro" } }
                ]
            })
        };
        let vivo = partido_de("1".into(), None, None, &comp("STATUS_FIRST_HALF", "in")).unwrap();
        assert_eq!((vivo.estado, vivo.minuto.as_deref()), (EstadoPartido::EnJuego, Some("34'")));
        assert_eq!(vivo.goles_local, Some(1));
        assert_eq!(vivo.local.abreviatura, "Boca Juniors", "sin abreviatura usa el nombre");
        let et = partido_de("1".into(), None, None, &comp("STATUS_HALFTIME", "in")).unwrap();
        assert_eq!((et.estado, et.minuto.as_deref()), (EstadoPartido::Entretiempo, Some("Entretiempo")));
        let susp = partido_de("1".into(), None, None, &comp("STATUS_POSTPONED", "post")).unwrap();
        assert_eq!(susp.estado, EstadoPartido::Suspendido);
        // Un partido programado no tiene marcador aunque ESPN mande "0".
        let pre = partido_de("1".into(), None, None, &comp("STATUS_SCHEDULED", "pre")).unwrap();
        assert_eq!(pre.goles_local, None);
    }

    #[test]
    fn un_gol_en_contra_y_una_roja() {
        let gol = json!({ "type": { "type": "own-goal" }, "scoringPlay": true, "clock": { "displayValue": "12'" },
                          "team": { "id": "5" }, "participants": [{ "athlete": { "displayName": "X" } }] });
        assert_eq!(evento_clave(&gol).unwrap().tipo, TipoEventoPartido::GolEnContra);
        let roja = json!({ "type": { "type": "red-card" }, "scoringPlay": false, "clock": { "displayValue": "80'" }, "team": { "id": "7" } });
        let roja = evento_clave(&roja).unwrap();
        assert_eq!((roja.tipo, roja.jugador), (TipoEventoPartido::Expulsion, None));
    }

    #[test]
    fn lee_la_tabla_por_zonas() {
        let tabla = leer_tabla(&ejemplo(include_str!("ejemplos/tabla_lpf.json"))).unwrap();
        assert_eq!(tabla.torneo, "Liga Profesional");
        assert_eq!(tabla.grupos[0].nombre, "Zona A");
        let boca = tabla.grupos[0].filas.iter().find(|f| f.equipo.id == "5").unwrap();
        assert_eq!((boca.puesto, boca.puntos, boca.jugados, boca.diferencia), (2, 33, 16, 10));
        assert_eq!((boca.ganados, boca.empatados, boca.perdidos), (9, 6, 1));
        assert!(tabla.grupos[0].filas.windows(2).all(|par| par[0].puesto <= par[1].puesto));
    }

    #[test]
    fn lee_un_jugador() {
        let j = leer_jugador(&ejemplo(include_str!("ejemplos/jugador_partidos.json")), "400359", "Tomás Aranda").unwrap();
        assert_eq!(j.club.as_deref(), Some("Boca Juniors"));
        assert_eq!(j.partidos, 3);
        let u = j.ultimo.unwrap();
        assert_eq!(u.rival, "Defensa y Justicia");
        assert_eq!(u.resultado, "1-0");
        assert_eq!(u.desenlace.as_deref(), Some("G"));
        assert_eq!(u.competencia.as_deref(), Some("Liga Profesional"));
        assert_eq!(u.fecha, "2026-11-07T20:30:00Z");
    }

    #[test]
    fn el_resultado_va_desde_el_lado_del_jugador_y_suma_goles() {
        let json = json!({
            "names": ["totalGoals", "goalAssists"],
            "events": { "9": { "id": "9", "gameDate": "2026-11-10T20:00:00.000+00:00", "homeTeamId": "1", "awayTeamId": "2",
                               "homeTeamScore": "0", "awayTeamScore": "2", "gameResult": "W", "team": { "id": "2" },
                               "opponent": { "displayName": "Rival" } } },
            "seasonTypes": [{ "categories": [{ "events": [{ "eventId": "9", "stats": ["2", "1"] }, { "eventId": "8", "stats": ["1", "0"] }] }] }]
        });
        let j = leer_jugador(&json, "1", "Messi").unwrap();
        let u = j.ultimo.unwrap();
        assert_eq!((u.resultado.as_str(), u.goles, u.asistencias), ("2-0", 2, 1));
        assert_eq!((j.partidos, j.goles, j.asistencias), (2, 3, 1));
        assert_eq!(j.club, None);
    }

    #[test]
    fn la_busqueda_deja_solo_futbolistas() {
        let r = leer_busqueda(&ejemplo(include_str!("ejemplos/busqueda_jugador.json")));
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], JugadorEncontrado { id: "153115".into(), nombre: "Leandro Paredes".into(), club: Some("Boca Juniors".into()) });
    }

    #[test]
    fn si_espn_elige_una_liga_sin_partidos_se_pide_otra() {
        // Lo que devolvía en noviembre de 2026 para un jugador de Boca.
        let vacio = json!({ "filters": [
            { "name": "season", "value": "2026" },
            { "name": "league", "value": "conmebol.recopa", "options": [
                { "value": "arg.1" }, { "value": "conmebol.sudamericana" }, { "value": "conmebol.recopa" }
            ] }
        ] });
        assert_eq!(otra_liga_del_jugador(&vacio).as_deref(), Some("arg.1"));
        // Si ya elegía la liga y vino vacía, prueba la siguiente.
        let mut en_liga = vacio.clone();
        en_liga["filters"][1]["value"] = json!("arg.1");
        assert_eq!(otra_liga_del_jugador(&en_liga).as_deref(), Some("conmebol.sudamericana"));
        // Con partidos, no se pide nada más.
        assert_eq!(otra_liga_del_jugador(&ejemplo(include_str!("ejemplos/jugador_partidos.json"))), None);
    }

    #[test]
    fn en_enero_se_pide_la_temporada_anterior() {
        let proximos = leer_calendario(&ejemplo(include_str!("ejemplos/fixture_boca.json")));
        assert!(!proximos.is_empty());
        assert!(falta_la_temporada_anterior(&proximos));
        assert!(falta_la_temporada_anterior(&[]));
        let jugados = leer_calendario(&ejemplo(include_str!("ejemplos/calendario_boca.json")));
        assert!(!falta_la_temporada_anterior(&jugados));
    }

    /// Usa internet (ESPN, gratis): `cargo test -- --ignored espn_de_verdad --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn espn_de_verdad() {
        let espn = ClienteEspn::nuevo().unwrap();
        let partidos = espn.calendario("5").await.unwrap();
        let jugados = partidos.iter().filter(|p| p.estado == EstadoPartido::Terminado).count();
        println!("Boca: {} partidos, {jugados} jugados", partidos.len());
        for p in partidos.iter().rev().take(4) {
            println!("  {} {} {} - {}", p.fecha, p.competencia, p.local.nombre, p.visitante.nombre);
        }
        assert!(jugados > 0, "sin partidos jugados: ESPN volvió a cambiar la temporada");
        let aranda = espn.jugador("400359", "Tomás Aranda").await.unwrap();
        println!("Aranda: {:?}", aranda.ultimo);
    }

    #[test]
    fn competencias_y_fases_en_castellano() {
        assert_eq!(competencia(Some("fifa.worldq.conmebol"), None), "Eliminatorias");
        assert_eq!(competencia(None, Some("Argentine Liga Profesional de Fútbol")), "Liga Profesional");
        assert_eq!(competencia(None, Some("Spanish LALIGA")), "Spanish LALIGA");
        assert_eq!(fase("Clausura - Round of 16").as_deref(), Some("octavos de final"));
        assert_eq!(fase("Semifinals").as_deref(), Some("semifinal"));
        assert_eq!(fase("Torneo Clausura"), None);
        assert_eq!(normalizar_fecha("2026-11-07T20:30:00.000+00:00").as_deref(), Some("2026-11-07T20:30:00Z"));
        assert_eq!(normalizar_fecha("ayer"), None);
    }
}
