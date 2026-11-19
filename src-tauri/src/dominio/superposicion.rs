//! ¿Dos eventos se pisan? Solo cuentan los que tienen hora: los de todo el
//! día (un cumpleaños, una entrega) conviven con cualquier cosa.

use chrono::{DateTime, Duration, FixedOffset};

use super::momento::Momento;

/// Un evento con hora y sin fin se toma como de una hora.
pub const MINUTOS_SIN_FIN: i64 = 60;

/// El rango [inicio, fin) de un evento con hora; `None` si es de todo el día
/// o el texto no se entiende.
pub fn rango(inicio: &str, fin: Option<&str>) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    let Ok(Momento::FechaHora(desde)) = Momento::parsear(inicio) else { return None };
    let hasta = match fin.map(Momento::parsear) {
        Some(Ok(Momento::FechaHora(h))) if h > desde => h,
        _ => desde + Duration::minutes(MINUTOS_SIN_FIN),
    };
    Some((desde, hasta))
}

/// Se pisan si uno empieza antes de que termine el otro. Que uno termine justo
/// cuando arranca el otro (de 18 a 19 y de 19 a 20) no es pisarse.
pub fn se_superponen(a: (&str, Option<&str>), b: (&str, Option<&str>)) -> bool {
    match (rango(a.0, a.1), rango(b.0, b.1)) {
        (Some((a0, a1)), Some((b0, b1))) => a0 < b1 && b0 < a1,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H18: &str = "2026-11-19T18:00:00-03:00";
    const H19: &str = "2026-11-19T19:00:00-03:00";
    const H1830: &str = "2026-11-19T18:30:00-03:00";
    const H2030: &str = "2026-11-19T20:30:00-03:00";

    #[test]
    fn el_perro_y_el_entrenamiento_se_pisan() {
        assert!(se_superponen((H18, Some(H19)), (H1830, Some(H2030))));
        assert!(se_superponen((H1830, Some(H2030)), (H18, Some(H19))));
    }

    #[test]
    fn uno_detras_del_otro_no() {
        assert!(!se_superponen((H18, Some(H19)), (H19, Some(H2030))));
    }

    #[test]
    fn sin_fin_dura_una_hora() {
        assert!(se_superponen((H18, None), (H1830, Some(H2030))));
        assert!(!se_superponen((H18, None), (H19, None)));
    }

    #[test]
    fn todo_el_dia_no_choca_con_nada() {
        assert!(!se_superponen(("2026-11-19", None), (H18, Some(H19))));
        assert!(!se_superponen(("2026-11-19", Some("2026-11-20")), ("2026-11-19", None)));
    }
}
