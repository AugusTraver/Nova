//! Cuándo vence un dato de internet guardado en la caché.
//!
//! La caché guarda, junto con cada dato, desde cuándo hay que volver a
//! pedirlo (`vence_en`). Estas funciones calculan ese momento; la tarea de
//! fondo solo compara "¿ya pasó?". Si un pedido falla, se reintenta cada vez
//! más espaciado, para no insistir sin internet.

use chrono::{DateTime, Duration, FixedOffset, NaiveTime, TimeZone};

/// Hora a la que se renuevan los datos "de una vez por día".
pub const HORA_DIARIA: u32 = 6;

/// La próxima vez que den las 6 de la mañana (hoy si todavía no son, si no
/// mañana). Así el fixture y la tabla están frescos cuando abrís el panel a
/// la mañana, aunque la compu haya estado prendida toda la noche.
pub fn proxima_manana(ahora: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let seis = NaiveTime::from_hms_opt(HORA_DIARIA, 0, 0).expect("hora válida");
    let hoy = ahora.date_naive().and_time(seis);
    let fecha = if ahora.naive_local() < hoy { hoy } else { hoy + Duration::days(1) };
    // `from_local_datetime` arma el momento en la misma zona que `ahora`.
    ahora.timezone().from_local_datetime(&fecha).single().unwrap_or(ahora + Duration::hours(24))
}

/// Lo primero entre la próxima mañana y `otro` (si hay y es futuro).
pub fn lo_antes(ahora: DateTime<FixedOffset>, otro: Option<DateTime<FixedOffset>>) -> DateTime<FixedOffset> {
    let manana = proxima_manana(ahora);
    match otro {
        Some(o) if o > ahora && o < manana => o,
        _ => manana,
    }
}

/// Espera antes de reintentar después de `fallas` fallas seguidas:
/// 5, 15, 30 y después cada 60 minutos.
pub fn espera_reintento(fallas: u32) -> Duration {
    match fallas {
        0 | 1 => Duration::minutes(5),
        2 => Duration::minutes(15),
        3 => Duration::minutes(30),
        _ => Duration::minutes(60),
    }
}

/// ¿Ya hay que volver a pedirlo?
pub fn vencido(vence_en: DateTime<FixedOffset>, ahora: DateTime<FixedOffset>) -> bool {
    ahora >= vence_en
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(texto: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(texto).unwrap()
    }

    #[test]
    fn la_proxima_manana_es_hoy_o_manana_a_las_6() {
        assert_eq!(proxima_manana(t("2026-11-17T03:00:00-03:00")), t("2026-11-17T06:00:00-03:00"));
        assert_eq!(proxima_manana(t("2026-11-17T06:00:00-03:00")), t("2026-11-18T06:00:00-03:00"));
        assert_eq!(proxima_manana(t("2026-11-17T23:59:00-03:00")), t("2026-11-18T06:00:00-03:00"));
    }

    #[test]
    fn lo_antes_entre_la_manana_y_otro_momento() {
        let ahora = t("2026-11-17T14:00:00-03:00");
        // El partido termina esta noche: se refresca después del partido.
        let despues_del_partido = t("2026-11-17T23:15:00-03:00");
        assert_eq!(lo_antes(ahora, Some(despues_del_partido)), despues_del_partido);
        // Un momento pasado o lejano no sirve: queda la mañana.
        assert_eq!(lo_antes(ahora, Some(t("2026-11-17T10:00:00-03:00"))), t("2026-11-18T06:00:00-03:00"));
        assert_eq!(lo_antes(ahora, Some(t("2026-11-25T10:00:00-03:00"))), t("2026-11-18T06:00:00-03:00"));
        assert_eq!(lo_antes(ahora, None), t("2026-11-18T06:00:00-03:00"));
    }

    #[test]
    fn los_reintentos_se_espacian() {
        assert_eq!(espera_reintento(1), Duration::minutes(5));
        assert_eq!(espera_reintento(2), Duration::minutes(15));
        assert_eq!(espera_reintento(3), Duration::minutes(30));
        assert_eq!(espera_reintento(9), Duration::minutes(60));
    }

    #[test]
    fn vence_justo_a_la_hora() {
        let vence = t("2026-11-17T06:00:00-03:00");
        assert!(!vencido(vence, t("2026-11-17T05:59:59-03:00")));
        assert!(vencido(vence, vence));
    }
}
