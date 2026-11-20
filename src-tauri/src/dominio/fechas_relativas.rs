//! Fechas y horas como las dice la gente: "el jueves", "mañana", "a las 3",
//! "20/11", "9 y media". El modelo copia lo que dijo el usuario y acá se
//! resuelve con la fecha de hoy, sin depender de que el modelo haga cuentas.
//!
//! También acepta el formato exacto (`2026-11-20`, `09:00`), por si el modelo
//! ya lo resolvió.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Weekday};

use super::lenguaje::normalizar;

const DIAS: &[(&str, Weekday)] = &[
    ("lunes", Weekday::Mon),
    ("martes", Weekday::Tue),
    ("miercoles", Weekday::Wed),
    ("jueves", Weekday::Thu),
    ("viernes", Weekday::Fri),
    ("sabado", Weekday::Sat),
    ("domingo", Weekday::Sun),
];

const MESES: &[&str] = &[
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];

/// Nombre del día en español ("jueves"), para el prompt y los mensajes.
pub fn nombre_dia(dia: Weekday) -> &'static str {
    DIAS.iter().find(|(_, d)| *d == dia).map(|(n, _)| *n).unwrap_or("")
}

/// "miercoles" → "miércoles", "sabado" → "sábado" (para mostrar).
pub fn nombre_dia_con_tilde(dia: Weekday) -> &'static str {
    match dia {
        Weekday::Wed => "miércoles",
        Weekday::Sat => "sábado",
        otro => nombre_dia(otro),
    }
}

pub fn nombre_mes(mes: u32) -> &'static str {
    MESES.get(mes as usize - 1).copied().unwrap_or("")
}

fn error_fecha(texto: &str) -> String {
    format!("No entendí la fecha «{texto}». Pasala como AAAA-MM-DD.")
}

/// Resuelve una fecha. Los días de la semana van siempre para adelante:
/// "el martes" dicho un martes es el de la semana que viene (para hoy está
/// "hoy").
pub fn resolver_fecha(texto: &str, hoy: NaiveDate) -> Result<NaiveDate, String> {
    let original = texto.trim();
    // Solo el ISO completo: `parse_from_str` también aceptaría "5-3-27" como
    // el año 5.
    if original.len() == 10
        && original.as_bytes()[4] == b'-'
        && let Ok(fecha) = NaiveDate::parse_from_str(original, "%Y-%m-%d")
    {
        return Ok(fecha);
    }

    let mut t = normalizar(original);
    for prefijo in ["para el ", "para ", "el dia ", "el ", "este ", "esta ", "dia "] {
        if let Some(resto) = t.strip_prefix(prefijo) {
            t = resto.to_string();
            break;
        }
    }
    let t = t.trim_end_matches(['.', ',', '!', '?']).trim();

    match t {
        "hoy" => return Ok(hoy),
        "manana" => return Ok(hoy + Duration::days(1)),
        "pasado manana" | "pasado" => return Ok(hoy + Duration::days(2)),
        "ayer" => return Ok(hoy - Duration::days(1)),
        "en una semana" | "dentro de una semana" | "la semana que viene" | "la proxima semana" => {
            return Ok(hoy + Duration::days(7));
        }
        _ => {}
    }

    // "en 3 días", "dentro de 2 semanas"
    let sin_dentro = t.strip_prefix("dentro de ").or_else(|| t.strip_prefix("en "));
    if let Some(resto) = sin_dentro {
        let mut partes = resto.split_whitespace();
        if let (Some(n), Some(unidad), None) = (partes.next(), partes.next(), partes.next()) {
            let n: Option<i64> = match n {
                "un" | "una" => Some(1),
                otro => otro.parse().ok(),
            };
            let factor = match unidad {
                "dia" | "dias" => Some(1),
                "semana" | "semanas" => Some(7),
                _ => None,
            };
            if let (Some(n), Some(f)) = (n, factor)
                && (0..=400).contains(&n)
            {
                return Ok(hoy + Duration::days(n * f));
            }
        }
        return Err(error_fecha(original));
    }

    // "20/11", "20/11/2026", "20-11"
    if let Some(fecha) = fecha_con_barras(t, hoy) {
        return fecha.ok_or_else(|| error_fecha(original));
    }

    // Día de la semana, solo o con número: "jueves", "jueves que viene",
    // "jueves de la semana que viene", "jueves 19".
    let mut palabras: Vec<&str> = t.split_whitespace().collect();
    if let Some(&primera) = palabras.first()
        && let Some(&(_, dia)) = DIAS.iter().find(|(n, _)| *n == primera)
    {
        palabras.remove(0);
        let resto = palabras.join(" ");
        let resto = resto.trim();
        return match resto {
            "" | "que viene" | "proximo" | "proxima" => Ok(proximo(dia, hoy)),
            "de la semana que viene" | "de la proxima semana" | "de la semana proxima" => {
                let lunes_que_viene = hoy + Duration::days(7 - i64::from(hoy.weekday().num_days_from_monday()));
                Ok(lunes_que_viene + Duration::days(i64::from(dia.num_days_from_monday())))
            }
            otro => {
                // "jueves 19" o "jueves 19 de noviembre": el número manda y el
                // día de la semana se usa para verificar.
                let fecha = numero_de_dia(otro, hoy).ok_or_else(|| error_fecha(original))?;
                if fecha.weekday() != dia {
                    return Err(format!(
                        "El {} cae {}, no {}. Preguntale al usuario cuál es.",
                        fecha.day(),
                        nombre_dia_con_tilde(fecha.weekday()),
                        nombre_dia_con_tilde(dia)
                    ));
                }
                Ok(fecha)
            }
        };
    }

    // "próximo jueves", "el próximo jueves"
    if let Some(resto) = t.strip_prefix("proximo ").or_else(|| t.strip_prefix("proxima "))
        && let Some(&(_, dia)) = DIAS.iter().find(|(n, _)| *n == resto)
    {
        return Ok(proximo(dia, hoy));
    }

    numero_de_dia(t, hoy).ok_or_else(|| error_fecha(original))
}

/// El próximo `dia` después de hoy (de 1 a 7 días).
fn proximo(dia: Weekday, hoy: NaiveDate) -> NaiveDate {
    let hoy_n = i64::from(hoy.weekday().num_days_from_monday());
    let dia_n = i64::from(dia.num_days_from_monday());
    let diferencia = (dia_n - hoy_n).rem_euclid(7);
    hoy + Duration::days(if diferencia == 0 { 7 } else { diferencia })
}

/// "20/11", "20/11/26", "20-11-2026". `None` si no tiene esa forma;
/// `Some(None)` si la tiene pero la fecha no existe.
fn fecha_con_barras(t: &str, hoy: NaiveDate) -> Option<Option<NaiveDate>> {
    let partes: Vec<&str> = t.split(['/', '-']).collect();
    if !(2..=3).contains(&partes.len()) || !partes.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    let dia: u32 = partes[0].parse().ok()?;
    let mes: u32 = partes[1].parse().ok()?;
    Some(match partes.get(2) {
        Some(anio) => {
            let anio: i32 = anio.parse().ok()?;
            let anio = if anio < 100 { 2000 + anio } else { anio };
            NaiveDate::from_ymd_opt(anio, mes, dia)
        }
        None => siguiente_con_mes(dia, mes, hoy),
    })
}

/// El próximo `dia/mes` desde hoy (este año o el que viene).
fn siguiente_con_mes(dia: u32, mes: u32, hoy: NaiveDate) -> Option<NaiveDate> {
    let este_anio = NaiveDate::from_ymd_opt(hoy.year(), mes, dia)?;
    if este_anio >= hoy { Some(este_anio) } else { NaiveDate::from_ymd_opt(hoy.year() + 1, mes, dia) }
}

/// "20", "20 de noviembre", "20 de noviembre de 2027".
fn numero_de_dia(t: &str, hoy: NaiveDate) -> Option<NaiveDate> {
    let palabras: Vec<&str> = t.split_whitespace().filter(|p| *p != "de" && *p != "del").collect();
    let dia: u32 = palabras.first()?.parse().ok()?;
    match palabras.as_slice() {
        [_] => {
            // Solo el número: este mes si todavía no pasó, si no el que viene.
            let este_mes = NaiveDate::from_ymd_opt(hoy.year(), hoy.month(), dia);
            if let Some(f) = este_mes.filter(|f| *f >= hoy) {
                return Some(f);
            }
            let (anio, mes) = if hoy.month() == 12 { (hoy.year() + 1, 1) } else { (hoy.year(), hoy.month() + 1) };
            NaiveDate::from_ymd_opt(anio, mes, dia)
        }
        [_, mes] => {
            let mes = MESES.iter().position(|m| m == mes)? as u32 + 1;
            siguiente_con_mes(dia, mes, hoy)
        }
        [_, mes, anio] => {
            let mes = MESES.iter().position(|m| m == mes)? as u32 + 1;
            NaiveDate::from_ymd_opt(anio.parse().ok()?, mes, dia)
        }
        _ => None,
    }
}

fn error_hora(texto: &str) -> String {
    format!("No entendí la hora «{texto}». Pasala como HH:MM (24 horas).")
}

/// Resuelve una hora. Sin aclaración, de 1 a 7 es de la tarde ("a las 3" son
/// las 15:00) y de 8 a 12 es tal cual: así se habla de clases y turnos.
/// Con cero adelante ("07:30") o con "de la mañana" se respeta.
pub fn resolver_hora(texto: &str) -> Result<NaiveTime, String> {
    resolver_hora_con(texto, true)
}

/// La hora de fin de algo que empieza a `inicio`. Devuelve la hora y si cae
/// al día siguiente. "De 22 a 2" termina a las 2 del otro día (y no a las
/// 14, como diría la regla de la tarde); "de 9 a 11" termina a las 11.
pub fn resolver_hora_fin(texto: &str, inicio: NaiveTime) -> Result<(NaiveTime, bool), String> {
    let con_regla = resolver_hora_con(texto, true)?;
    if con_regla > inicio {
        return Ok((con_regla, false));
    }
    let literal = resolver_hora_con(texto, false)?;
    Ok((literal.min(con_regla), true))
}

/// El modelo a veces pasa "de 6 a 7" a "06:00" y "07:00", y con el cero
/// adelante la hora cuenta como dicha en 24 horas: el evento quedaba a la
/// mañana. Si el usuario no la dijo con el cero ni aclaró que era a la
/// mañana, se le saca el cero para que valga la regla de la tarde (v0.7.1).
pub fn hora_como_la_dijo(hora: &str, pedido: &str) -> String {
    let h = hora.trim();
    let bytes = h.as_bytes();
    let con_cero = bytes.len() == 5
        && bytes[0] == b'0'
        && (b'1'..=b'7').contains(&bytes[1])
        && (bytes[2] == b':' || bytes[2] == b'.')
        && bytes[3..].iter().all(u8::is_ascii_digit);
    if !con_cero {
        return h.to_string();
    }
    let p = normalizar(pedido);
    let aclaro_manana = ["de la manana", "a la manana", "por la manana", "en la manana", "madrugada", " am", "a.m", "temprano"]
        .iter()
        .any(|m| p.contains(m));
    if aclaro_manana || p.contains(&h[..2]) {
        return h.to_string();
    }
    h[1..].to_string()
}

/// `regla_tarde`: si de 1 a 7 sin aclarar se lee como de la tarde.
fn resolver_hora_con(texto: &str, regla_tarde: bool) -> Result<NaiveTime, String> {
    let original = texto.trim();
    let mut t = normalizar(original);
    for prefijo in ["a las ", "a la ", "las ", "la "] {
        if let Some(resto) = t.strip_prefix(prefijo) {
            t = resto.to_string();
            break;
        }
    }
    let t = t.trim_end_matches(['.', ',', '!', '?']).trim().to_string();

    match t.as_str() {
        "mediodia" | "el mediodia" => return Ok(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
        "medianoche" => return Ok(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        _ => {}
    }

    // Separa el número del principio ("15:30", "9.30", "9h", "3pm") del resto.
    let fin_numero = t.find(|c: char| !(c.is_ascii_digit() || c == ':' || c == '.')).unwrap_or(t.len());
    let (numero, resto) = t.split_at(fin_numero);
    if numero.is_empty() {
        return Err(error_hora(original));
    }
    let mut piezas = numero.split([':', '.']);
    let texto_hora = piezas.next().unwrap_or("");
    let texto_minutos = piezas.next();
    if piezas.next().is_some() || texto_hora.is_empty() || texto_hora.len() > 2 {
        return Err(error_hora(original));
    }
    let mut hora: u32 = texto_hora.parse().map_err(|_| error_hora(original))?;
    let mut minutos: u32 = match texto_minutos {
        Some(m) if m.len() == 2 => m.parse().map_err(|_| error_hora(original))?,
        Some(_) => return Err(error_hora(original)),
        None => 0,
    };

    let resto = resto.trim();
    let resto = resto
        .strip_prefix("hs")
        .or_else(|| resto.strip_prefix("horas"))
        .or_else(|| resto.strip_prefix("hora"))
        .or_else(|| resto.strip_prefix('h'))
        .unwrap_or(resto)
        .trim();

    // "y media", "y cuarto", "menos cuarto" (antes del momento del día).
    let (resto, ajuste) = if let Some(r) = resto.strip_prefix("y media") {
        (r, 30i64)
    } else if let Some(r) = resto.strip_prefix("y cuarto") {
        (r, 15)
    } else if let Some(r) = resto.strip_prefix("menos cuarto") {
        (r, -15)
    } else {
        (resto, 0)
    };
    if ajuste != 0 && texto_minutos.is_some() {
        return Err(error_hora(original));
    }
    let resto = resto.trim();

    enum Parte {
        Manana,
        Tarde,
        Noche,
        SinAclarar,
    }
    let parte = match resto {
        "" => Parte::SinAclarar,
        "am" | "a.m." | "de la manana" | "de la madrugada" | "a la manana" => Parte::Manana,
        "pm" | "p.m." | "de la tarde" | "a la tarde" => Parte::Tarde,
        "de la noche" | "a la noche" => Parte::Noche,
        _ => return Err(error_hora(original)),
    };

    if hora > 23 || minutos > 59 {
        return Err(error_hora(original));
    }
    let explicita_24h = hora >= 13 || texto_hora.starts_with('0');
    hora = match parte {
        Parte::Manana if hora == 12 => 0,
        Parte::Manana => hora,
        Parte::Tarde | Parte::Noche if explicita_24h => hora,
        Parte::Noche if hora == 12 => 0,
        Parte::Tarde | Parte::Noche if hora < 12 => hora + 12,
        Parte::Tarde | Parte::Noche => hora,
        Parte::SinAclarar if regla_tarde && !explicita_24h && (1..=7).contains(&hora) => hora + 12,
        Parte::SinAclarar => hora,
    };
    if hora > 23 {
        return Err(error_hora(original));
    }

    let base = i64::from(hora) * 60 + i64::from(minutos) + ajuste;
    let base = base.rem_euclid(24 * 60);
    hora = (base / 60) as u32;
    minutos = (base % 60) as u32;
    NaiveTime::from_hms_opt(hora, minutos, 0).ok_or_else(|| error_hora(original))
}

/// Junta fecha y hora en el huso horario dado (el del sistema).
pub fn combinar(fecha: NaiveDate, hora: NaiveTime, huso: FixedOffset) -> DateTime<FixedOffset> {
    // Con un huso fijo cada hora local existe una sola vez, así que `single`
    // nunca falla; igual caemos a la interpretación UTC por las dudas.
    huso.from_local_datetime(&fecha.and_time(hora))
        .single()
        .unwrap_or_else(|| huso.from_utc_datetime(&fecha.and_time(hora)))
}

#[cfg(test)]
mod tests_hora_como_la_dijo {
    use super::*;

    #[test]
    fn saca_el_cero_que_puso_el_modelo() {
        assert_eq!(hora_como_la_dijo("06:00", "agendame fútbol de 6 a 7"), "6:00");
        assert_eq!(resolver_hora(&hora_como_la_dijo("06:00", "de 6 a 7")).unwrap(), NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        assert_eq!(hora_como_la_dijo("07:30", "mañana a las 7 y media"), "7:30");
    }

    #[test]
    fn respeta_lo_que_aclaraste() {
        assert_eq!(hora_como_la_dijo("06:00", "a las 6 de la mañana"), "06:00");
        assert_eq!(hora_como_la_dijo("06:00", "a las 06:00"), "06:00");
        assert_eq!(hora_como_la_dijo("05:00", "salgo a correr a las 5 am"), "05:00");
        assert_eq!(hora_como_la_dijo("18:00", "a las 6"), "18:00");
        assert_eq!(hora_como_la_dijo("09:00", "a las 9"), "09:00");
        assert_eq!(hora_como_la_dijo("6", "a las 6"), "6");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Martes 17 de noviembre de 2026.
    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 17).unwrap()
    }

    fn f(texto: &str) -> String {
        resolver_fecha(texto, hoy()).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|e| e)
    }

    fn h(texto: &str) -> String {
        resolver_hora(texto).map(|t| t.format("%H:%M").to_string()).unwrap_or_else(|e| e)
    }

    #[test]
    fn palabras_relativas() {
        assert_eq!(f("hoy"), "2026-11-17");
        assert_eq!(f("Mañana"), "2026-11-18");
        assert_eq!(f("manana"), "2026-11-18");
        assert_eq!(f("pasado mañana"), "2026-11-19");
        assert_eq!(f("ayer"), "2026-11-16");
        assert_eq!(f("en 3 días"), "2026-11-20");
        assert_eq!(f("dentro de 2 semanas"), "2026-12-01");
        assert_eq!(f("en una semana"), "2026-11-24");
    }

    #[test]
    fn dias_de_la_semana_van_para_adelante() {
        assert_eq!(f("el jueves"), "2026-11-19");
        assert_eq!(f("jueves"), "2026-11-19");
        assert_eq!(f("el miércoles"), "2026-11-18");
        assert_eq!(f("el lunes"), "2026-11-23");
        // Hoy es martes: "el martes" es el de la semana que viene.
        assert_eq!(f("el martes"), "2026-11-24");
        assert_eq!(f("el próximo viernes"), "2026-11-20");
        assert_eq!(f("el viernes que viene"), "2026-11-20");
        assert_eq!(f("Sábado"), "2026-11-21");
    }

    #[test]
    fn semana_que_viene() {
        assert_eq!(f("el jueves de la semana que viene"), "2026-11-26");
        assert_eq!(f("lunes de la próxima semana"), "2026-11-23");
    }

    #[test]
    fn dia_con_numero_se_verifica() {
        assert_eq!(f("jueves 19"), "2026-11-19");
        assert_eq!(f("el jueves 19 de noviembre"), "2026-11-19");
        assert!(f("jueves 20").contains("cae viernes"));
    }

    #[test]
    fn fechas_numericas() {
        assert_eq!(f("2026-11-20"), "2026-11-20");
        assert_eq!(f("20/11"), "2026-11-20");
        assert_eq!(f("20/11/2026"), "2026-11-20");
        assert_eq!(f("5-3-27"), "2027-03-05");
        // Ya pasó este año: es la del que viene.
        assert_eq!(f("10/3"), "2027-03-10");
        assert_eq!(f("el 25"), "2026-11-25");
        assert_eq!(f("el 2"), "2026-12-02");
        assert_eq!(f("20 de noviembre"), "2026-11-20");
        assert_eq!(f("3 de marzo"), "2027-03-03");
        assert_eq!(f("3 de marzo de 2028"), "2028-03-03");
    }

    #[test]
    fn fechas_que_no_existen_o_no_se_entienden() {
        assert!(f("31/11").starts_with("No entendí"));
        assert!(f("el finde").starts_with("No entendí"));
        assert!(f("en tres meses").starts_with("No entendí"));
        assert!(f("").starts_with("No entendí"));
    }

    #[test]
    fn diciembre_pasa_al_anio_siguiente() {
        let fin_de_anio = NaiveDate::from_ymd_opt(2026, 12, 30).unwrap();
        let r = resolver_fecha("el 2", fin_de_anio).unwrap();
        assert_eq!(r, NaiveDate::from_ymd_opt(2027, 1, 2).unwrap());
        let r = resolver_fecha("el viernes", fin_de_anio).unwrap();
        assert_eq!(r, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    }

    #[test]
    fn horas_explicitas() {
        assert_eq!(h("09:00"), "09:00");
        assert_eq!(h("15:30"), "15:30");
        assert_eq!(h("a las 21.15"), "21:15");
        assert_eq!(h("18hs"), "18:00");
        assert_eq!(h("07:30"), "07:30");
        assert_eq!(h("0:15"), "00:15");
    }

    #[test]
    fn horas_sin_aclarar() {
        assert_eq!(h("a las 9"), "09:00");
        assert_eq!(h("9"), "09:00");
        assert_eq!(h("a las 3"), "15:00");
        assert_eq!(h("7"), "19:00");
        assert_eq!(h("8"), "08:00");
        assert_eq!(h("12"), "12:00");
        assert_eq!(h("las 10 y media"), "10:30");
        assert_eq!(h("4 y cuarto"), "16:15");
        assert_eq!(h("9 menos cuarto"), "08:45");
    }

    #[test]
    fn momento_del_dia() {
        assert_eq!(h("7 de la mañana"), "07:00");
        assert_eq!(h("3 de la tarde"), "15:00");
        assert_eq!(h("9 de la noche"), "21:00");
        assert_eq!(h("12 de la noche"), "00:00");
        assert_eq!(h("2 de la madrugada"), "02:00");
        assert_eq!(h("8pm"), "20:00");
        assert_eq!(h("10 am"), "10:00");
        assert_eq!(h("mediodía"), "12:00");
        assert_eq!(h("6 y media de la tarde"), "18:30");
    }

    #[test]
    fn horas_invalidas() {
        assert!(h("25").starts_with("No entendí"));
        assert!(h("9:7").starts_with("No entendí"));
        assert!(h("tipo 9").starts_with("No entendí"));
        assert!(h("9:30 y media").starts_with("No entendí"));
        assert!(h("").starts_with("No entendí"));
    }

    #[test]
    fn hora_de_fin_segun_el_inicio() {
        let t = |h: u32| NaiveTime::from_hms_opt(h, 0, 0).unwrap();
        assert_eq!(resolver_hora_fin("11", t(9)).unwrap(), (t(11), false));
        assert_eq!(resolver_hora_fin("5", t(15)).unwrap(), (t(17), false));
        assert_eq!(resolver_hora_fin("1", t(10)).unwrap(), (t(13), false));
        assert_eq!(resolver_hora_fin("2", t(22)).unwrap(), (t(2), true));
        assert!(resolver_hora_fin("00:30", t(23)).unwrap().1);
    }

    #[test]
    fn combina_con_el_huso_del_sistema() {
        let huso = FixedOffset::west_opt(3 * 3600).unwrap();
        let fecha = resolver_fecha("el jueves", hoy()).unwrap();
        let hora = resolver_hora("a las 9").unwrap();
        assert_eq!(combinar(fecha, hora, huso).to_rfc3339(), "2026-11-19T09:00:00-03:00");
    }
}
