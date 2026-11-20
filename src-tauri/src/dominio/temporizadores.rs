//! Temporizadores (v0.7.1): "avisame en 10 minutos que saque la ropa",
//! "avisame a las 6 que llame a mamá". Lo puro: cuándo vence y qué decir.

use chrono::{DateTime, Duration, FixedOffset};

use super::fechas_relativas::{combinar, resolver_hora};

/// Un día como mucho: para más, mejor una responsabilidad o un evento.
pub const MINUTOS_MAXIMOS: i64 = 24 * 60;
const LARGO_MAXIMO_TEXTO: usize = 200;

/// Cuándo vence: en `minutos` o a la `hora` dicha (hoy, o mañana si ya pasó).
pub fn cuando(minutos: Option<i64>, hora: Option<&str>, ahora: DateTime<FixedOffset>) -> Result<DateTime<FixedOffset>, String> {
    match (minutos, hora.map(str::trim).filter(|h| !h.is_empty())) {
        (Some(m), None) => {
            if !(1..=MINUTOS_MAXIMOS).contains(&m) {
                return Err("Puedo avisarte desde dentro de un minuto hasta dentro de un día.".into());
            }
            Ok(ahora + Duration::minutes(m))
        }
        (None, Some(h)) => {
            let hora = resolver_hora(h)?;
            let hoy = combinar(ahora.date_naive(), hora, *ahora.offset());
            // Si ya pasó (o es ahora mismo), es mañana.
            Ok(if hoy <= ahora { hoy + Duration::days(1) } else { hoy })
        }
        (Some(_), Some(_)) => Err("Decime en cuántos minutos o a qué hora, una de las dos.".into()),
        (None, None) => Err("¿En cuánto tiempo te aviso?".into()),
    }
}

pub fn validar_texto(texto: &str) -> Result<String, String> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Err("¿De qué te aviso?".into());
    }
    Ok(texto.chars().take(LARGO_MAXIMO_TEXTO).collect())
}

/// "a las 17:25" o "mañana a las 06:00": para que el modelo confirme.
pub fn describir(vence: DateTime<FixedOffset>, ahora: DateTime<FixedOffset>) -> String {
    let hora = vence.format("%H:%M");
    if vence.date_naive() == ahora.date_naive() { format!("hoy a las {hora}") } else { format!("mañana a las {hora}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_las(texto: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(&format!("2026-11-20T{texto}:00-03:00")).unwrap()
    }

    #[test]
    fn en_minutos() {
        assert_eq!(cuando(Some(10), None, a_las("17:15")).unwrap(), a_las("17:25"));
        assert!(cuando(Some(0), None, a_las("17:15")).is_err());
        assert!(cuando(Some(2000), None, a_las("17:15")).is_err());
        assert!(cuando(None, None, a_las("17:15")).is_err());
        assert!(cuando(Some(5), Some("18"), a_las("17:15")).is_err());
    }

    #[test]
    fn a_una_hora() {
        // "a las 6" sin aclarar es de la tarde (la regla de siempre).
        assert_eq!(cuando(None, Some("6"), a_las("17:15")).unwrap(), a_las("18:00"));
        assert_eq!(cuando(None, Some("17:30"), a_las("17:15")).unwrap(), a_las("17:30"));
        // Si ya pasó, es mañana.
        let manana = cuando(None, Some("9"), a_las("17:15")).unwrap();
        assert_eq!(manana.to_rfc3339(), "2026-11-21T09:00:00-03:00");
        assert_eq!(describir(manana, a_las("17:15")), "mañana a las 09:00");
        assert_eq!(describir(a_las("18:00"), a_las("17:15")), "hoy a las 18:00");
    }

    #[test]
    fn el_texto_no_puede_estar_vacio() {
        assert!(validar_texto("  ").is_err());
        assert_eq!(validar_texto(" sacar la ropa ").unwrap(), "sacar la ropa");
    }
}
