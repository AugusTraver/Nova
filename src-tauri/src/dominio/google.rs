//! Google Calendar ↔ Nova, sin red ni base (v0.7.2): cómo se traduce un
//! evento en cada sentido, la recurrencia (RRULE), qué tipo tiene un evento
//! que llega sin tipo, quién gana si cambió de los dos lados y cuándo dos
//! eventos son el mismo.
//!
//! Lo que Nova sabe repetir: semanal (el mismo día de la semana del inicio)
//! y anual, con fecha de fin opcional y fechas salteadas. Todo lo demás
//! (diario, mensual, cada 2 semanas, varios días) es una "serie rara": la
//! tarea de fondo guarda sus repeticiones sueltas, de solo lectura.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, SecondsFormat, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::lenguaje::normalizar;
use super::momento::Momento;
use super::texto::{Recurrencia, TipoEvento};

/// Zona por defecto si Google no dijo la del calendario.
pub const ZONA_POR_DEFECTO: &str = "America/Argentina/Buenos_Aires";

// Claves de `extendedProperties.private`: datos de Nova que Google guarda
// sin mostrarlos, para que no se pierdan al ir y volver.
const PROP_TIPO: &str = "nova_tipo";
const PROP_MATERIA: &str = "nova_materia";
const PROP_PERSONA: &str = "nova_persona";
const PROP_SIN_FIN: &str = "nova_sin_fin";

/// Un evento como lo manda la API de Calendar (solo los campos que usamos).
/// `Option` en todo: los borrados llegan casi vacíos.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventoGoogle {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub start: Option<CuandoGoogle>,
    #[serde(default)]
    pub end: Option<CuandoGoogle>,
    #[serde(default)]
    pub recurrence: Option<Vec<String>>,
    #[serde(default)]
    pub recurring_event_id: Option<String>,
    #[serde(default)]
    pub original_start_time: Option<CuandoGoogle>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub extended_properties: Option<Extendidas>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuandoGoogle {
    /// Todo el día: "2026-11-20".
    #[serde(default)]
    pub date: Option<String>,
    /// Con hora: "2026-11-20T09:00:00-03:00".
    #[serde(default)]
    pub date_time: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Extendidas {
    #[serde(default)]
    pub private: HashMap<String, String>,
}

impl EventoGoogle {
    pub fn cancelado(&self) -> bool {
        self.status.as_deref() == Some("cancelled")
    }

    fn propiedad(&self, clave: &str) -> Option<&str> {
        self.extended_properties
            .as_ref()
            .and_then(|e| e.private.get(clave))
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }
}

/// Lo que se guarda en Nova (los mismos campos que `repos::eventos::DatosEvento`).
#[derive(Debug, Clone, PartialEq)]
pub struct Traducido {
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
    pub excepciones: Vec<String>,
}

/// Qué hacer con algo que llegó de Google.
#[derive(Debug, Clone, PartialEq)]
pub enum Llegada {
    /// Un evento suelto o una serie que Nova sabe repetir.
    Evento(Traducido),
    /// Una repetición de una serie, cambiada en Google (otra hora, otro día,
    /// otro título). `fecha` es el día en que caía originalmente.
    Movida { serie: String, fecha: String, evento: Traducido },
    /// Una repetición borrada de una serie.
    Cancelada { serie: String, fecha: String },
    /// Se borró el evento (o la serie entera).
    Borrado,
    /// Una serie que Nova no sabe repetir: hay que traer sus repeticiones.
    SerieRara,
    /// Algo que no va a la agenda (el "lugar de trabajo" de Google) o que vino roto.
    Ignorar,
}

/// Una materia para reconocerla en los títulos.
#[derive(Debug, Clone)]
pub struct MateriaConocida {
    pub id: i64,
    pub nombre: String,
}

/// Traduce un evento de Google. `zona` es el huso de la compu: las horas se
/// guardan en ese huso, como las que cargás en Nova.
pub fn traducir(ev: &EventoGoogle, zona: FixedOffset, materias: &[MateriaConocida]) -> Llegada {
    let fecha_original = ev.original_start_time.as_ref().and_then(|c| dia_de(c, zona));
    if ev.cancelado() {
        return match (&ev.recurring_event_id, fecha_original) {
            (Some(serie), Some(fecha)) => Llegada::Cancelada { serie: serie.clone(), fecha: texto_dia(fecha) },
            _ => Llegada::Borrado,
        };
    }
    if ev.event_type.as_deref() == Some("workingLocation") {
        return Llegada::Ignorar;
    }
    let Some(evento) = traducir_datos(ev, zona, materias) else {
        return Llegada::Ignorar;
    };

    if let (Some(serie), Some(fecha)) = (&ev.recurring_event_id, fecha_original) {
        return Llegada::Movida { serie: serie.clone(), fecha: texto_dia(fecha), evento };
    }
    let lineas = ev.recurrence.as_deref().unwrap_or_default();
    if lineas.is_empty() {
        return Llegada::Evento(evento);
    }
    let Ok(inicio) = Momento::parsear(&evento.inicio) else {
        return Llegada::Ignorar;
    };
    match leer_recurrencia(lineas, &inicio, zona) {
        Some(r) => Llegada::Evento(Traducido {
            recurrencia: r.recurrencia,
            repetir_hasta: r.hasta.map(texto_dia),
            excepciones: r.excepciones.into_iter().map(texto_dia).collect(),
            ..evento
        }),
        None => Llegada::SerieRara,
    }
}

/// Los datos de un evento, sin mirar la recurrencia (sirve también para las
/// repeticiones sueltas de una serie rara).
pub fn traducir_datos(ev: &EventoGoogle, zona: FixedOffset, materias: &[MateriaConocida]) -> Option<Traducido> {
    let inicio = momento_de(ev.start.as_ref()?, zona)?;
    let fin = ev.end.as_ref().and_then(|c| momento_de(c, zona));
    let todo_el_dia = inicio.es_fecha_sola();

    let fin = match (inicio, fin) {
        // En Google el fin de un evento de todo el día es el día siguiente
        // (no incluido); en Nova, el último día incluido.
        (Momento::Fecha(a), Some(Momento::Fecha(b))) => {
            let ultimo = b - Duration::days(1);
            (ultimo > a).then(|| Momento::Fecha(ultimo))
        }
        (Momento::FechaHora(a), Some(Momento::FechaHora(b))) if b > a => {
            (ev.propiedad(PROP_SIN_FIN).is_none()).then_some(Momento::FechaHora(b))
        }
        _ => None,
    };

    let titulo: String = ev.summary.as_deref().map(str::trim).filter(|t| !t.is_empty()).unwrap_or("(Sin título)").chars().take(200).collect();
    let tipo = ev
        .propiedad(PROP_TIPO)
        .and_then(TipoEvento::desde_texto)
        .unwrap_or_else(|| if ev.event_type.as_deref() == Some("birthday") { TipoEvento::Cumpleanos } else { adivinar_tipo(&titulo) });
    let materia_id = match ev.propiedad(PROP_MATERIA) {
        Some(nombre) => materias.iter().find(|m| normalizar(&m.nombre) == normalizar(nombre)).map(|m| m.id),
        None if matches!(tipo, TipoEvento::Clase | TipoEvento::Examen | TipoEvento::Entrega) => materia_de(&titulo, materias),
        None => None,
    };
    let persona = match ev.propiedad(PROP_PERSONA) {
        Some(p) => Some(p.to_string()),
        None if tipo == TipoEvento::Cumpleanos => persona_de(&titulo),
        None => None,
    };
    let notas = ev.description.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(String::from);

    Some(Traducido {
        titulo,
        tipo,
        inicio: inicio.a_texto(),
        fin: fin.map(|f| f.a_texto()),
        todo_el_dia,
        materia_id,
        persona,
        notas,
        recurrencia: Recurrencia::Ninguna,
        repetir_hasta: None,
        excepciones: Vec::new(),
    })
}

/// Fecha sola o fecha con hora pasada al huso de la compu.
fn momento_de(cuando: &CuandoGoogle, zona: FixedOffset) -> Option<Momento> {
    if let Some(fecha) = &cuando.date {
        return NaiveDate::parse_from_str(fecha, "%Y-%m-%d").ok().map(Momento::Fecha);
    }
    let fh = DateTime::parse_from_rfc3339(cuando.date_time.as_deref()?).ok()?;
    Some(Momento::FechaHora(fh.with_timezone(&zona)))
}

fn dia_de(cuando: &CuandoGoogle, zona: FixedOffset) -> Option<NaiveDate> {
    momento_de(cuando, zona).map(|m| m.fecha())
}

fn texto_dia(fecha: NaiveDate) -> String {
    fecha.format("%Y-%m-%d").to_string()
}

// ---------- Recurrencia ----------

#[derive(Debug, Clone, PartialEq)]
pub struct RecurrenciaLeida {
    pub recurrencia: Recurrencia,
    pub hasta: Option<NaiveDate>,
    pub excepciones: Vec<NaiveDate>,
}

/// Lee las líneas `RRULE`/`EXDATE` de Google. `None` si Nova no la sabe
/// repetir.
pub fn leer_recurrencia(lineas: &[String], inicio: &Momento, zona: FixedOffset) -> Option<RecurrenciaLeida> {
    let mut regla = None;
    let mut excepciones = Vec::new();
    for linea in lineas {
        let (nombre, valor) = linea.split_once(':')?;
        let nombre = nombre.split(';').next().unwrap_or_default().to_ascii_uppercase();
        match nombre.as_str() {
            "RRULE" if regla.is_none() => regla = Some(valor),
            // `EXDATE;TZID=...:20261123T150000,20261130T150000`
            "EXDATE" => {
                for parte in linea.rsplit_once(':')?.1.split(',') {
                    excepciones.push(dia_ical(parte.trim(), zona)?);
                }
            }
            // RDATE, EXRULE o dos RRULE: Nova no las sabe repetir.
            _ => return None,
        }
    }

    let mut partes = HashMap::new();
    for par in regla?.split(';') {
        let (k, v) = par.split_once('=')?;
        partes.insert(k.to_ascii_uppercase(), v.to_ascii_uppercase());
    }
    let primera = inicio.fecha();
    let recurrencia = match partes.get("FREQ")?.as_str() {
        "WEEKLY" => Recurrencia::Semanal,
        "YEARLY" => Recurrencia::Anual,
        _ => return None,
    };
    for (clave, valor) in &partes {
        let encaja = match clave.as_str() {
            "FREQ" | "UNTIL" | "COUNT" | "WKST" => true,
            "INTERVAL" => valor == "1",
            "BYDAY" => recurrencia == Recurrencia::Semanal && valor == codigo_dia(primera.weekday()),
            "BYMONTH" => recurrencia == Recurrencia::Anual && valor.parse() == Ok(primera.month()),
            "BYMONTHDAY" => recurrencia == Recurrencia::Anual && valor.parse() == Ok(primera.day()),
            _ => false,
        };
        if !encaja {
            return None;
        }
    }

    let hasta = match (partes.get("UNTIL"), partes.get("COUNT")) {
        (Some(until), _) => Some(dia_ical(until, zona)?),
        (None, Some(cuenta)) => {
            let n: i64 = cuenta.parse().ok().filter(|n| *n >= 1)?;
            Some(match recurrencia {
                Recurrencia::Semanal => primera + Duration::weeks(n - 1),
                _ => primera.with_year(primera.year() + i32::try_from(n - 1).ok()?).unwrap_or(primera + Duration::days(365 * (n - 1))),
            })
        }
        (None, None) => None,
    };
    excepciones.sort();
    excepciones.dedup();
    Some(RecurrenciaLeida { recurrencia, hasta, excepciones })
}

/// "20261123", "20261123T150000" (hora local del calendario) o
/// "20261123T180000Z" (UTC, se pasa al huso de la compu).
fn dia_ical(texto: &str, zona: FixedOffset) -> Option<NaiveDate> {
    if let Some(utc) = texto.strip_suffix('Z') {
        let fh = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some(fh.and_utc().with_timezone(&zona).date_naive());
    }
    NaiveDate::parse_from_str(texto.get(..8)?, "%Y%m%d").ok()
}

fn codigo_dia(dia: Weekday) -> &'static str {
    match dia {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}

/// Las líneas de recurrencia para Google. Las fechas salteadas van como
/// `EXDATE`, en UTC para no depender de la zona del calendario.
pub fn escribir_recurrencia(
    recurrencia: Recurrencia,
    inicio: &Momento,
    hasta: Option<NaiveDate>,
    excepciones: &[NaiveDate],
) -> Vec<String> {
    let frecuencia = match recurrencia {
        Recurrencia::Ninguna => return Vec::new(),
        Recurrencia::Semanal => "WEEKLY",
        Recurrencia::Anual => "YEARLY",
    };
    let mut regla = format!("RRULE:FREQ={frecuencia}");
    if let Some(hasta) = hasta {
        match inicio {
            Momento::Fecha(_) => regla.push_str(&format!(";UNTIL={}", hasta.format("%Y%m%d"))),
            // Con hora, UNTIL va en UTC: el final de ese día en el huso del evento.
            Momento::FechaHora(fh) => {
                let fin_del_dia = hasta.and_hms_opt(23, 59, 59).and_then(|n| n.and_local_timezone(*fh.offset()).single());
                if let Some(f) = fin_del_dia {
                    regla.push_str(&format!(";UNTIL={}", f.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")));
                }
            }
        }
    }
    let mut lineas = vec![regla];
    if !excepciones.is_empty() {
        let fechas: Vec<String> = excepciones
            .iter()
            .filter_map(|dia| match inicio {
                Momento::Fecha(_) => Some(dia.format("%Y%m%d").to_string()),
                Momento::FechaHora(fh) => {
                    let corrido = *fh + Duration::days((*dia - fh.date_naive()).num_days());
                    Some(corrido.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string())
                }
            })
            .collect();
        let prefijo = if inicio.es_fecha_sola() { "EXDATE;VALUE=DATE:" } else { "EXDATE:" };
        lineas.push(format!("{prefijo}{}", fechas.join(",")));
    }
    lineas
}

// ---------- Hacia Google ----------

/// Un evento de Nova listo para subir.
#[derive(Debug, Clone)]
pub struct ParaSubir<'a> {
    pub titulo: &'a str,
    pub tipo: TipoEvento,
    pub inicio: &'a str,
    pub fin: Option<&'a str>,
    pub materia: Option<&'a str>,
    pub persona: Option<&'a str>,
    pub notas: Option<&'a str>,
    pub recurrencia: Recurrencia,
    pub repetir_hasta: Option<&'a str>,
    /// Las salteadas, sin las que reemplaza una repetición movida (si no,
    /// Google borraría también la movida).
    pub excepciones: Vec<NaiveDate>,
    /// Es una repetición movida: no lleva recurrencia propia.
    pub es_repeticion: bool,
}

/// El cuerpo para `events.insert` o `events.patch`.
pub fn cuerpo(p: &ParaSubir, zona_calendario: &str) -> Result<Value, String> {
    let inicio = Momento::parsear(p.inicio)?;
    let fin = p.fin.map(Momento::parsear).transpose()?;
    let (start, end, sin_fin) = match (inicio, fin) {
        (Momento::Fecha(a), fin) => {
            let ultimo = match fin {
                Some(Momento::Fecha(b)) if b > a => b,
                _ => a,
            };
            (json!({ "date": texto_dia(a) }), json!({ "date": texto_dia(ultimo + Duration::days(1)) }), false)
        }
        (Momento::FechaHora(a), fin) => {
            let (b, sin_fin) = match fin {
                Some(Momento::FechaHora(b)) if b > a => (b, false),
                // Google pide un fin: una hora, y se anota que en Nova no tenía.
                _ => (a + Duration::hours(1), true),
            };
            let con_hora = |m: DateTime<FixedOffset>| {
                json!({ "dateTime": m.to_rfc3339_opts(SecondsFormat::Secs, false), "timeZone": zona_calendario })
            };
            (con_hora(a), con_hora(b), sin_fin)
        }
    };

    let mut cuerpo = json!({
        "summary": p.titulo,
        "description": p.notas.unwrap_or(""),
        "start": start,
        "end": end,
        "extendedProperties": { "private": {
            PROP_TIPO: p.tipo.como_texto(),
            PROP_MATERIA: p.materia.unwrap_or(""),
            PROP_PERSONA: p.persona.unwrap_or(""),
            PROP_SIN_FIN: if sin_fin { "1" } else { "" },
        }},
    });
    if !p.es_repeticion {
        let hasta = p.repetir_hasta.and_then(|h| NaiveDate::parse_from_str(h, "%Y-%m-%d").ok());
        cuerpo["recurrence"] = json!(escribir_recurrencia(p.recurrencia, &inicio, hasta, &p.excepciones));
    }
    Ok(cuerpo)
}

// ---------- Adivinar lo que Google no tiene ----------

fn palabras(texto: &str) -> Vec<String> {
    normalizar(texto)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

/// El tipo de un evento que se creó en Calendar, por su título.
pub fn adivinar_tipo(titulo: &str) -> TipoEvento {
    let p = palabras(titulo);
    let hay = |lista: &[&str]| p.iter().any(|w| lista.contains(&w.as_str()));
    let empieza = |prefijos: &[&str]| p.iter().any(|w| prefijos.iter().any(|x| w.starts_with(x)));

    if empieza(&["cumple"]) {
        TipoEvento::Cumpleanos
    } else if hay(&["partido", "vs", "boca", "seleccion"]) {
        TipoEvento::Partido
    } else if hay(&["entrega", "entregar", "tp", "tps", "tpo", "informe", "monografia"])
        || p.iter().any(|w| w.starts_with("tp") && w[2..].chars().all(|c| c.is_ascii_digit()))
        || normalizar(titulo).contains("trabajo practico")
    {
        TipoEvento::Entrega
    } else if hay(&["parcial", "parcialito", "examen", "final", "recuperatorio", "coloquio", "evaluacion", "quiz"]) {
        TipoEvento::Examen
    } else if hay(&["clase", "clases", "teorica", "teorico", "practica", "practico", "cursada", "consulta"]) {
        TipoEvento::Clase
    } else {
        TipoEvento::Otro
    }
}

/// "Cumple de Juan" → "Juan". Respeta cómo está escrito el nombre.
pub fn persona_de(titulo: &str) -> Option<String> {
    let palabras: Vec<&str> = titulo.split_whitespace().collect();
    let pos = palabras.iter().position(|w| normalizar(w).starts_with("cumple"))?;
    let mut resto = &palabras[pos + 1..];
    if resto.first().is_some_and(|w| matches!(normalizar(w).as_str(), "de" | "del")) {
        resto = &resto[1..];
    }
    let persona = resto.join(" ").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    (!persona.is_empty()).then_some(persona)
}

/// La materia cuyo nombre aparece en el título (la más larga, si hay dos).
pub fn materia_de(titulo: &str, materias: &[MateriaConocida]) -> Option<i64> {
    let titulo = format!(" {} ", palabras(titulo).join(" "));
    materias
        .iter()
        .filter(|m| {
            let nombre = palabras(&m.nombre).join(" ");
            !nombre.is_empty() && titulo.contains(&format!(" {nombre} "))
        })
        .max_by_key(|m| m.nombre.chars().count())
        .map(|m| m.id)
}

// ---------- Quién gana ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ganador {
    Nova,
    Google,
}

/// Si cambió de los dos lados, gana el cambio más nuevo. Empate o fecha que
/// no se entiende: gana Google (es lo que ves en el celular).
pub fn quien_gana(nova: &str, google: Option<&str>) -> Ganador {
    let leer = |t: &str| DateTime::parse_from_rfc3339(t).ok();
    match (leer(nova), google.and_then(leer)) {
        (Some(n), Some(g)) if n > g => Ganador::Nova,
        _ => Ganador::Google,
    }
}

/// Cómo está el evento en Nova cuando llega un cambio de Google.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnNova<'a> {
    NoEsta,
    /// Lo tenés y no lo cambiaste desde la última vez.
    Igual,
    /// Lo cambiaste y falta subirlo (`actualizado_en`).
    Cambiado(&'a str),
    /// Lo borraste y falta avisar a Google (`borrado_en`).
    Borrado(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accion {
    /// Guardar lo de Google.
    Aplicar,
    /// Lo borraste en Nova pero después lo editaron en Google: vuelve, y
    /// se olvida el borrado.
    Restaurar,
    /// Gana lo de Nova: no se toca, y se sube en esta misma vuelta.
    Ignorar,
    /// Se borró en Google y no lo habías tocado: se borra en Nova.
    Borrar,
    /// Se borró en Google pero lo editaste en Nova: gana la edición y se
    /// vuelve a crear allá.
    Recrear,
    /// Se borró de los dos lados: solo hay que olvidar el borrado pendiente.
    OlvidarBorrado,
    Nada,
}

/// Qué hacer con un cambio de Google según cómo está el evento en Nova.
pub fn decidir(cancelado: bool, en_nova: EnNova, actualizado_google: Option<&str>) -> Accion {
    match (cancelado, en_nova) {
        (true, EnNova::NoEsta) => Accion::Nada,
        (true, EnNova::Igual) => Accion::Borrar,
        (true, EnNova::Cambiado(_)) => Accion::Recrear,
        (true, EnNova::Borrado(_)) => Accion::OlvidarBorrado,
        (false, EnNova::NoEsta | EnNova::Igual) => Accion::Aplicar,
        (false, EnNova::Cambiado(nova)) => match quien_gana(nova, actualizado_google) {
            Ganador::Google => Accion::Aplicar,
            Ganador::Nova => Accion::Ignorar,
        },
        (false, EnNova::Borrado(borrado)) => match quien_gana(borrado, actualizado_google) {
            Ganador::Google => Accion::Restaurar,
            Ganador::Nova => Accion::Ignorar,
        },
    }
}

// ---------- La primera vez ----------

/// Dos eventos son "el mismo" si tienen el mismo título (sin importar
/// mayúsculas ni acentos) y empiezan en el mismo momento.
pub fn clave_para_juntar(titulo: &str, inicio: &str) -> Option<String> {
    let momento = match Momento::parsear(inicio).ok()? {
        Momento::Fecha(f) => texto_dia(f),
        Momento::FechaHora(fh) => fh.with_timezone(&Utc).format("%Y-%m-%dT%H:%M").to_string(),
    };
    Some(format!("{}|{momento}", normalizar(titulo)))
}

/// ¿Entra en lo que se sincroniza? Lo que terminó antes de `desde` queda
/// solo donde está (para no subir ni traer años de historia).
pub fn vigente(inicio: &str, fin: Option<&str>, recurrencia: Recurrencia, repetir_hasta: Option<&str>, desde: NaiveDate) -> bool {
    let dia = |t: &str| Momento::parsear(t).ok().map(|m| m.fecha());
    let ultimo = match recurrencia {
        Recurrencia::Ninguna => fin.and_then(dia).or_else(|| dia(inicio)),
        _ => match repetir_hasta {
            Some(h) => dia(h),
            None => return true,
        },
    };
    ultimo.is_none_or(|u| u >= desde)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg() -> FixedOffset {
        FixedOffset::west_opt(3 * 3600).unwrap()
    }

    fn con_hora(t: &str) -> Option<CuandoGoogle> {
        Some(CuandoGoogle { date_time: Some(t.into()), ..Default::default() })
    }

    fn dia(t: &str) -> Option<CuandoGoogle> {
        Some(CuandoGoogle { date: Some(t.into()), ..Default::default() })
    }

    fn d(t: &str) -> NaiveDate {
        NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap()
    }

    fn materias() -> Vec<MateriaConocida> {
        vec![
            MateriaConocida { id: 1, nombre: "Análisis".into() },
            MateriaConocida { id: 2, nombre: "Análisis II".into() },
            MateriaConocida { id: 3, nombre: "Física".into() },
        ]
    }

    fn evento(titulo: &str, start: Option<CuandoGoogle>, end: Option<CuandoGoogle>) -> EventoGoogle {
        EventoGoogle { id: "g1".into(), summary: Some(titulo.into()), start, end, ..Default::default() }
    }

    fn traducido(llegada: Llegada) -> Traducido {
        match llegada {
            Llegada::Evento(t) => t,
            otra => panic!("esperaba un evento, llegó {otra:?}"),
        }
    }

    #[test]
    fn lee_el_json_de_google() {
        let json = r#"{
            "kind": "calendar#event", "id": "abc", "status": "confirmed", "etag": "\"3\"",
            "updated": "2026-11-17T15:04:05.123Z", "summary": "Parcial de Física",
            "start": {"dateTime": "2026-11-20T09:00:00-03:00", "timeZone": "America/Argentina/Buenos_Aires"},
            "end": {"dateTime": "2026-11-20T11:00:00-03:00"},
            "eventType": "default", "reminders": {"useDefault": true}
        }"#;
        let ev: EventoGoogle = serde_json::from_str(json).unwrap();
        let t = traducido(traducir(&ev, arg(), &materias()));
        assert_eq!(t.titulo, "Parcial de Física");
        assert_eq!(t.tipo, TipoEvento::Examen);
        assert_eq!(t.materia_id, Some(3));
        assert_eq!(t.inicio, "2026-11-20T09:00:00-03:00");
        assert_eq!(t.fin.as_deref(), Some("2026-11-20T11:00:00-03:00"));
        assert!(!t.todo_el_dia);
    }

    #[test]
    fn pasa_la_hora_al_huso_de_la_compu() {
        let ev = evento("Llamada", con_hora("2026-11-20T12:00:00Z"), con_hora("2026-11-20T13:00:00Z"));
        let t = traducido(traducir(&ev, arg(), &[]));
        assert_eq!(t.inicio, "2026-11-20T09:00:00-03:00");
    }

    #[test]
    fn todo_el_dia_con_fin_incluido() {
        let un_dia = evento("Feriado", dia("2026-11-20"), dia("2026-11-21"));
        let t = traducido(traducir(&un_dia, arg(), &[]));
        assert_eq!((t.inicio.as_str(), t.fin.as_deref(), t.todo_el_dia), ("2026-11-20", None, true));

        let viaje = evento("Viaje", dia("2026-12-01"), dia("2026-12-06"));
        assert_eq!(traducido(traducir(&viaje, arg(), &[])).fin.as_deref(), Some("2026-12-05"));
    }

    #[test]
    fn respeta_lo_que_guardo_nova() {
        let mut ev = evento("Algo raro", con_hora("2026-11-20T09:00:00-03:00"), con_hora("2026-11-20T10:00:00-03:00"));
        ev.extended_properties = Some(Extendidas {
            private: HashMap::from([
                ("nova_tipo".into(), "examen".into()),
                ("nova_materia".into(), "física".into()),
                ("nova_sin_fin".into(), "1".into()),
            ]),
        });
        let t = traducido(traducir(&ev, arg(), &materias()));
        assert_eq!((t.tipo, t.materia_id, t.fin), (TipoEvento::Examen, Some(3), None));
    }

    #[test]
    fn cumpleanos_de_google() {
        let mut ev = evento("Cumpleaños de María José", dia("2026-11-23"), dia("2026-11-24"));
        ev.event_type = Some("birthday".into());
        ev.recurrence = Some(vec!["RRULE:FREQ=YEARLY".into()]);
        let t = traducido(traducir(&ev, arg(), &[]));
        assert_eq!(t.tipo, TipoEvento::Cumpleanos);
        assert_eq!(t.persona.as_deref(), Some("María José"));
        assert_eq!(t.recurrencia, Recurrencia::Anual);
    }

    #[test]
    fn borrados_movidas_y_lugar_de_trabajo() {
        let borrado = EventoGoogle { id: "g1".into(), status: Some("cancelled".into()), ..Default::default() };
        assert_eq!(traducir(&borrado, arg(), &[]), Llegada::Borrado);

        let cancelada = EventoGoogle {
            id: "clase_20261123T180000Z".into(),
            status: Some("cancelled".into()),
            recurring_event_id: Some("clase".into()),
            original_start_time: con_hora("2026-11-23T15:00:00-03:00"),
            ..Default::default()
        };
        assert_eq!(
            traducir(&cancelada, arg(), &[]),
            Llegada::Cancelada { serie: "clase".into(), fecha: "2026-11-23".into() }
        );

        let mut movida = evento("Análisis", con_hora("2026-11-24T15:00:00-03:00"), None);
        movida.recurring_event_id = Some("clase".into());
        movida.original_start_time = con_hora("2026-11-23T18:00:00Z");
        match traducir(&movida, arg(), &materias()) {
            Llegada::Movida { serie, fecha, evento } => {
                assert_eq!((serie.as_str(), fecha.as_str()), ("clase", "2026-11-23"));
                assert_eq!(evento.inicio, "2026-11-24T15:00:00-03:00");
            }
            otra => panic!("{otra:?}"),
        }

        let mut lugar = evento("Oficina", dia("2026-11-20"), dia("2026-11-21"));
        lugar.event_type = Some("workingLocation".into());
        assert_eq!(traducir(&lugar, arg(), &[]), Llegada::Ignorar);
        assert_eq!(traducir(&evento("Sin fecha", None, None), arg(), &[]), Llegada::Ignorar);
    }

    #[test]
    fn recurrencias_que_nova_sabe_repetir() {
        let lunes = Momento::parsear("2026-11-02T15:00:00-03:00").unwrap();
        let leer = |lineas: &[&str]| {
            let lineas: Vec<String> = lineas.iter().map(|l| l.to_string()).collect();
            leer_recurrencia(&lineas, &lunes, arg())
        };

        let r = leer(&["RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20261208T025959Z", "EXDATE;TZID=America/Argentina/Buenos_Aires:20261123T150000"]).unwrap();
        assert_eq!(r.recurrencia, Recurrencia::Semanal);
        // 02:59 UTC del 8 es el 7 a la noche en Argentina.
        assert_eq!(r.hasta, Some(d("2026-12-07")));
        assert_eq!(r.excepciones, [d("2026-11-23")]);

        let r = leer(&["RRULE:FREQ=WEEKLY;WKST=SU;COUNT=4", "EXDATE:20261116T180000Z,20261109T180000Z"]).unwrap();
        assert_eq!(r.hasta, Some(d("2026-11-23")));
        assert_eq!(r.excepciones, [d("2026-11-09"), d("2026-11-16")]);

        let cumple = Momento::parsear("2000-11-23").unwrap();
        let anual = leer_recurrencia(&["RRULE:FREQ=YEARLY;BYMONTH=11;BYMONTHDAY=23".into()], &cumple, arg()).unwrap();
        assert_eq!((anual.recurrencia, anual.hasta), (Recurrencia::Anual, None));
    }

    #[test]
    fn recurrencias_que_no() {
        let lunes = Momento::parsear("2026-11-02T15:00:00-03:00").unwrap();
        for raras in [
            "RRULE:FREQ=DAILY",
            "RRULE:FREQ=MONTHLY;BYMONTHDAY=2",
            "RRULE:FREQ=WEEKLY;INTERVAL=2",
            "RRULE:FREQ=WEEKLY;BYDAY=MO,WE",
            "RRULE:FREQ=WEEKLY;BYDAY=TU",
        ] {
            assert!(leer_recurrencia(&[raras.into()], &lunes, arg()).is_none(), "{raras}");
        }
        assert!(leer_recurrencia(&["RRULE:FREQ=WEEKLY".into(), "RDATE:20261210T180000Z".into()], &lunes, arg()).is_none());

        let mut ev = evento("Gimnasio", con_hora("2026-11-02T08:00:00-03:00"), con_hora("2026-11-02T09:00:00-03:00"));
        ev.recurrence = Some(vec!["RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR".into()]);
        assert_eq!(traducir(&ev, arg(), &[]), Llegada::SerieRara);
    }

    #[test]
    fn escribe_la_recurrencia_y_la_vuelve_a_leer() {
        let lunes = Momento::parsear("2026-11-02T15:00:00-03:00").unwrap();
        let lineas = escribir_recurrencia(Recurrencia::Semanal, &lunes, Some(d("2026-12-07")), &[d("2026-11-23")]);
        assert_eq!(lineas, ["RRULE:FREQ=WEEKLY;UNTIL=20261208T025959Z", "EXDATE:20261123T180000Z"]);
        let r = leer_recurrencia(&lineas, &lunes, arg()).unwrap();
        assert_eq!((r.recurrencia, r.hasta, r.excepciones), (Recurrencia::Semanal, Some(d("2026-12-07")), vec![d("2026-11-23")]));

        let cumple = Momento::parsear("2000-11-23").unwrap();
        assert_eq!(
            escribir_recurrencia(Recurrencia::Anual, &cumple, None, &[d("2026-11-23")]),
            ["RRULE:FREQ=YEARLY", "EXDATE;VALUE=DATE:20261123"]
        );
        assert!(escribir_recurrencia(Recurrencia::Ninguna, &lunes, None, &[]).is_empty());
    }

    fn para_subir<'a>(titulo: &'a str, inicio: &'a str, fin: Option<&'a str>) -> ParaSubir<'a> {
        ParaSubir {
            titulo,
            tipo: TipoEvento::Examen,
            inicio,
            fin,
            materia: Some("Física"),
            persona: None,
            notas: None,
            recurrencia: Recurrencia::Ninguna,
            repetir_hasta: None,
            excepciones: Vec::new(),
            es_repeticion: false,
        }
    }

    #[test]
    fn arma_el_cuerpo_para_google() {
        let c = cuerpo(&para_subir("Parcial", "2026-11-20T09:00:00-03:00", None), ZONA_POR_DEFECTO).unwrap();
        assert_eq!(c["start"]["dateTime"], "2026-11-20T09:00:00-03:00");
        assert_eq!(c["end"]["dateTime"], "2026-11-20T10:00:00-03:00");
        assert_eq!(c["start"]["timeZone"], ZONA_POR_DEFECTO);
        assert_eq!(c["extendedProperties"]["private"]["nova_tipo"], "examen");
        assert_eq!(c["extendedProperties"]["private"]["nova_materia"], "Física");
        assert_eq!(c["extendedProperties"]["private"]["nova_sin_fin"], "1");
        assert_eq!(c["recurrence"], json!([]));

        let viaje = cuerpo(&para_subir("Viaje", "2026-12-01", Some("2026-12-05")), ZONA_POR_DEFECTO).unwrap();
        assert_eq!((viaje["start"]["date"].as_str(), viaje["end"]["date"].as_str()), (Some("2026-12-01"), Some("2026-12-06")));

        let movida = ParaSubir { es_repeticion: true, ..para_subir("Clase", "2026-11-24", None) };
        assert!(cuerpo(&movida, ZONA_POR_DEFECTO).unwrap().get("recurrence").is_none());
    }

    #[test]
    fn ida_y_vuelta_sin_perder_nada() {
        let mut p = para_subir("Parcial de Física", "2026-11-20T09:00:00-03:00", None);
        p.notas = Some("Aula 12");
        let c = cuerpo(&p, ZONA_POR_DEFECTO).unwrap();
        let mut ev: EventoGoogle = serde_json::from_value(c).unwrap();
        ev.id = "g1".into();
        let t = traducido(traducir(&ev, arg(), &materias()));
        assert_eq!(t.inicio, "2026-11-20T09:00:00-03:00");
        assert_eq!(t.fin, None);
        assert_eq!(t.tipo, TipoEvento::Examen);
        assert_eq!(t.materia_id, Some(3));
        assert_eq!(t.notas.as_deref(), Some("Aula 12"));
    }

    #[test]
    fn adivina_el_tipo_por_el_titulo() {
        let casos = [
            ("Parcial de Análisis", TipoEvento::Examen),
            ("Final Física", TipoEvento::Examen),
            ("Recuperatorio", TipoEvento::Examen),
            ("Entrega TP2", TipoEvento::Entrega),
            ("Trabajo práctico de Física", TipoEvento::Entrega),
            ("Cumple de Juan", TipoEvento::Cumpleanos),
            ("Boca vs River", TipoEvento::Partido),
            ("Clase de consulta", TipoEvento::Clase),
            ("Teórica de Análisis", TipoEvento::Clase),
            ("Dentista", TipoEvento::Otro),
            ("Tpomar mate", TipoEvento::Otro),
        ];
        for (titulo, tipo) in casos {
            assert_eq!(adivinar_tipo(titulo), tipo, "{titulo}");
        }
    }

    #[test]
    fn persona_y_materia_del_titulo() {
        assert_eq!(persona_de("Cumple de Juan").as_deref(), Some("Juan"));
        assert_eq!(persona_de("cumpleaños del Tano").as_deref(), Some("Tano"));
        assert_eq!(persona_de("Cumple Sofi!").as_deref(), Some("Sofi"));
        assert_eq!(persona_de("Cumple"), None);

        assert_eq!(materia_de("Parcial de Análisis II", &materias()), Some(2));
        assert_eq!(materia_de("parcial de analisis", &materias()), Some(1));
        assert_eq!(materia_de("Parcial de Química", &materias()), None);
        // "Física" no está adentro de otra palabra.
        assert_eq!(materia_de("Astrofísica", &materias()), None);
    }

    #[test]
    fn gana_el_cambio_mas_nuevo() {
        assert_eq!(quien_gana("2026-11-17T12:05:00-03:00", Some("2026-11-17T15:00:00.000Z")), Ganador::Nova);
        assert_eq!(quien_gana("2026-11-17T12:00:00-03:00", Some("2026-11-17T15:30:00.000Z")), Ganador::Google);
        assert_eq!(quien_gana("2026-11-17T12:00:00-03:00", None), Ganador::Google);
    }

    #[test]
    fn decide_segun_lo_que_hay_en_nova() {
        let g = Some("2026-11-17T15:30:00.000Z");
        assert_eq!(decidir(false, EnNova::NoEsta, g), Accion::Aplicar);
        assert_eq!(decidir(false, EnNova::Igual, g), Accion::Aplicar);
        assert_eq!(decidir(false, EnNova::Cambiado("2026-11-17T12:40:00-03:00"), g), Accion::Ignorar);
        assert_eq!(decidir(false, EnNova::Cambiado("2026-11-17T12:10:00-03:00"), g), Accion::Aplicar);
        assert_eq!(decidir(false, EnNova::Borrado("2026-11-17T12:10:00-03:00"), g), Accion::Restaurar);
        assert_eq!(decidir(false, EnNova::Borrado("2026-11-17T12:40:00-03:00"), g), Accion::Ignorar);
        assert_eq!(decidir(true, EnNova::Igual, None), Accion::Borrar);
        assert_eq!(decidir(true, EnNova::Cambiado("2026-11-17T12:00:00-03:00"), None), Accion::Recrear);
        assert_eq!(decidir(true, EnNova::Borrado("x"), None), Accion::OlvidarBorrado);
        assert_eq!(decidir(true, EnNova::NoEsta, None), Accion::Nada);
    }

    #[test]
    fn juntar_y_ventana() {
        assert_eq!(
            clave_para_juntar("Parcial de Análisis ", "2026-11-20T09:00:00-03:00"),
            clave_para_juntar("parcial de analisis", "2026-11-20T12:00:00Z")
        );
        assert_ne!(clave_para_juntar("Parcial", "2026-11-20"), clave_para_juntar("Parcial", "2026-11-21"));

        let desde = d("2026-10-18");
        assert!(vigente("2026-11-20", None, Recurrencia::Ninguna, None, desde));
        assert!(!vigente("2026-09-01", None, Recurrencia::Ninguna, None, desde));
        assert!(vigente("2026-10-10", Some("2026-10-20"), Recurrencia::Ninguna, None, desde));
        assert!(vigente("2020-03-02T15:00:00-03:00", None, Recurrencia::Semanal, None, desde));
        assert!(!vigente("2026-03-02", None, Recurrencia::Semanal, Some("2026-07-01"), desde));
    }
}
