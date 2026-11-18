//! Un momento de un evento: una fecha sola (eventos de todo el día) o una
//! fecha con hora y zona horaria. Se guarda como texto ISO 8601.

use chrono::{DateTime, Duration, FixedOffset, NaiveDate, SecondsFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Momento {
    /// "2026-11-20"
    Fecha(NaiveDate),
    /// "2026-11-20T09:00:00-03:00"
    FechaHora(DateTime<FixedOffset>),
}

impl Momento {
    pub fn parsear(texto: &str) -> Result<Self, String> {
        if let Ok(fecha) = NaiveDate::parse_from_str(texto, "%Y-%m-%d") {
            return Ok(Momento::Fecha(fecha));
        }
        DateTime::parse_from_rfc3339(texto)
            .map(Momento::FechaHora)
            .map_err(|_| format!("fecha inválida: {texto}"))
    }

    pub fn es_fecha_sola(&self) -> bool {
        matches!(self, Momento::Fecha(_))
    }

    /// El día en el que cae, en la zona horaria con la que se guardó.
    pub fn fecha(&self) -> NaiveDate {
        match self {
            Momento::Fecha(f) => *f,
            Momento::FechaHora(fh) => fh.date_naive(),
        }
    }

    /// El mismo momento corrido `dias` días (misma hora, misma zona).
    ///
    /// Se mantiene la zona horaria original. En Argentina no hay horario de
    /// verano, así que una clase de las 9:00 sigue siendo a las 9:00.
    pub fn mover_dias(&self, dias: i64) -> Self {
        match self {
            Momento::Fecha(f) => Momento::Fecha(*f + Duration::days(dias)),
            Momento::FechaHora(fh) => Momento::FechaHora(*fh + Duration::days(dias)),
        }
    }

    pub fn a_texto(&self) -> String {
        match self {
            Momento::Fecha(f) => f.format("%Y-%m-%d").to_string(),
            Momento::FechaHora(fh) => fh.to_rfc3339_opts(SecondsFormat::Secs, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_y_escribe_los_dos_formatos() {
        let fecha = Momento::parsear("2026-11-20").unwrap();
        assert!(fecha.es_fecha_sola());
        assert_eq!(fecha.a_texto(), "2026-11-20");

        let hora = Momento::parsear("2026-11-20T09:00:00-03:00").unwrap();
        assert!(!hora.es_fecha_sola());
        assert_eq!(hora.a_texto(), "2026-11-20T09:00:00-03:00");
        assert_eq!(hora.fecha(), NaiveDate::from_ymd_opt(2026, 11, 20).unwrap());
    }

    #[test]
    fn la_fecha_es_la_del_huso_guardado() {
        // 23:30 en Argentina es el día siguiente en UTC, pero el evento es del 20.
        let tarde = Momento::parsear("2026-11-20T23:30:00-03:00").unwrap();
        assert_eq!(tarde.fecha(), NaiveDate::from_ymd_opt(2026, 11, 20).unwrap());
    }

    #[test]
    fn mover_dias_conserva_la_hora() {
        let clase = Momento::parsear("2026-11-16T15:00:00-03:00").unwrap();
        assert_eq!(clase.mover_dias(7).a_texto(), "2026-11-23T15:00:00-03:00");
        let cumple = Momento::parsear("2026-12-31").unwrap();
        assert_eq!(cumple.mover_dias(1).a_texto(), "2027-01-01");
    }

    #[test]
    fn rechaza_textos_raros() {
        assert!(Momento::parsear("el jueves").is_err());
        assert!(Momento::parsear("2026-13-01").is_err());
        assert!(Momento::parsear("2026-11-20T09:00").is_err()); // sin zona horaria
    }
}
