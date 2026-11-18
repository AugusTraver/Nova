//! Repeticiones de eventos recurrentes.
//!
//! No se guarda una fila por repetición: se guarda la serie (el evento con su
//! `recurrencia`) y acá se calculan las repeticiones que caen en un rango.
//! Es lógica pura (no toca la base), así que se testea con datos fijos.

use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDate};

use super::momento::Momento;
use super::texto::Recurrencia;

/// Lo que hace falta de un evento para calcular sus repeticiones.
pub struct Serie<'a> {
    pub inicio: Momento,
    pub fin: Option<Momento>,
    pub recurrencia: Recurrencia,
    /// Última fecha (incluida) en la que puede empezar una repetición.
    pub repetir_hasta: Option<NaiveDate>,
    /// Fechas de inicio que se saltean.
    pub excepciones: &'a HashSet<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Repeticion {
    pub inicio: Momento,
    pub fin: Option<Momento>,
}

impl Repeticion {
    pub fn fecha(&self) -> NaiveDate {
        self.inicio.fecha()
    }
}

/// Las repeticiones que tocan el rango `[desde, hasta]` (ambos incluidos), en orden.
/// Un evento que dura varios días entra si alguno de sus días cae en el rango.
pub fn repeticiones(serie: &Serie, desde: NaiveDate, hasta: NaiveDate) -> Vec<Repeticion> {
    let primera = serie.inicio.fecha();
    // Cuántos días dura (0 si empieza y termina el mismo día).
    let duracion = serie
        .fin
        .map(|f| (f.fecha() - primera).num_days().max(0))
        .unwrap_or(0);

    let candidatas: Vec<NaiveDate> = match serie.recurrencia {
        Recurrencia::Ninguna => vec![primera],
        Recurrencia::Semanal => {
            // Saltamos directo a la primera semana que puede tocar el rango, en
            // vez de recorrer todas desde el principio de la serie.
            let atraso = (desde - primera).num_days() - duracion;
            let primera_semana = if atraso > 0 { atraso / 7 } else { 0 };
            (primera_semana..)
                .map(|k| primera + Duration::weeks(k))
                .take_while(|f| *f <= hasta)
                .collect()
        }
        Recurrencia::Anual => {
            let desde_anio = primera.year().max(desde.year() - 1);
            (desde_anio..=hasta.year())
                .map(|anio| mismo_dia_en(primera, anio))
                .collect()
        }
    };

    candidatas
        .into_iter()
        .filter(|f| *f >= primera)
        .filter(|f| serie.repetir_hasta.is_none_or(|tope| *f <= tope))
        .filter(|f| !serie.excepciones.contains(f))
        .filter(|f| *f + Duration::days(duracion) >= desde && *f <= hasta)
        .map(|f| {
            let corrimiento = (f - primera).num_days();
            Repeticion {
                inicio: serie.inicio.mover_dias(corrimiento),
                fin: serie.fin.map(|fin| fin.mover_dias(corrimiento)),
            }
        })
        .collect()
}

/// La primera repetición que empieza en `desde` o después (busca hasta dos años).
pub fn proxima(serie: &Serie, desde: NaiveDate) -> Option<Repeticion> {
    repeticiones(serie, desde, desde + Duration::days(366 * 2))
        .into_iter()
        .find(|r| r.fecha() >= desde)
}

/// El mismo día y mes en otro año. El 29 de febrero, en años no bisiestos,
/// cae el 28 (así el cumpleaños no desaparece tres de cada cuatro años).
fn mismo_dia_en(fecha: NaiveDate, anio: i32) -> NaiveDate {
    fecha
        .with_year(anio)
        .or_else(|| NaiveDate::from_ymd_opt(anio, 2, 28))
        .expect("el 28 de febrero existe todos los años")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    fn m(texto: &str) -> Momento {
        Momento::parsear(texto).unwrap()
    }

    fn inicios(lista: &[Repeticion]) -> Vec<String> {
        lista.iter().map(|r| r.inicio.a_texto()).collect()
    }

    fn serie<'a>(inicio: &str, recurrencia: Recurrencia, sin: &'a HashSet<NaiveDate>) -> Serie<'a> {
        Serie { inicio: m(inicio), fin: None, recurrencia, repetir_hasta: None, excepciones: sin }
    }

    #[test]
    fn evento_comun_aparece_solo_si_cae_en_el_rango() {
        let sin = HashSet::new();
        let parcial = serie("2026-11-19T09:00:00-03:00", Recurrencia::Ninguna, &sin);
        assert_eq!(repeticiones(&parcial, d("2026-11-17"), d("2026-11-20")).len(), 1);
        assert!(repeticiones(&parcial, d("2026-11-20"), d("2026-11-30")).is_empty());
    }

    #[test]
    fn clase_semanal_se_repite_a_la_misma_hora() {
        let sin = HashSet::new();
        let clase = serie("2026-08-10T15:00:00-03:00", Recurrencia::Semanal, &sin);
        let lista = repeticiones(&clase, d("2026-11-16"), d("2026-11-30"));
        assert_eq!(
            inicios(&lista),
            ["2026-11-16T15:00:00-03:00", "2026-11-23T15:00:00-03:00", "2026-11-30T15:00:00-03:00"]
        );
    }

    #[test]
    fn la_serie_no_empieza_antes_de_su_primera_fecha() {
        let sin = HashSet::new();
        let clase = serie("2026-11-18T10:00:00-03:00", Recurrencia::Semanal, &sin);
        let lista = repeticiones(&clase, d("2026-11-01"), d("2026-11-26"));
        assert_eq!(inicios(&lista), ["2026-11-18T10:00:00-03:00", "2026-11-25T10:00:00-03:00"]);
    }

    #[test]
    fn respeta_repetir_hasta_y_las_excepciones() {
        let feriado: HashSet<NaiveDate> = [d("2026-11-23")].into();
        let mut clase = serie("2026-11-02T15:00:00-03:00", Recurrencia::Semanal, &feriado);
        clase.repetir_hasta = Some(d("2026-12-01"));
        let lista = repeticiones(&clase, d("2026-11-01"), d("2026-12-31"));
        assert_eq!(
            inicios(&lista),
            [
                "2026-11-02T15:00:00-03:00",
                "2026-11-09T15:00:00-03:00",
                "2026-11-16T15:00:00-03:00",
                "2026-11-30T15:00:00-03:00",
            ]
        );
    }

    #[test]
    fn cumpleanos_anual() {
        let sin = HashSet::new();
        let cumple = serie("2001-11-20", Recurrencia::Anual, &sin);
        let lista = repeticiones(&cumple, d("2026-01-01"), d("2027-12-31"));
        assert_eq!(inicios(&lista), ["2026-11-20", "2027-11-20"]);
    }

    #[test]
    fn el_29_de_febrero_cae_el_28_en_anios_comunes() {
        let sin = HashSet::new();
        let cumple = serie("2004-02-29", Recurrencia::Anual, &sin);
        assert_eq!(inicios(&repeticiones(&cumple, d("2027-01-01"), d("2027-12-31"))), ["2027-02-28"]);
        assert_eq!(inicios(&repeticiones(&cumple, d("2028-01-01"), d("2028-12-31"))), ["2028-02-29"]);
    }

    #[test]
    fn evento_de_varios_dias_entra_si_lo_toca_el_rango() {
        let sin = HashSet::new();
        let mut semana = serie("2026-11-16", Recurrencia::Ninguna, &sin);
        semana.fin = Some(m("2026-11-20"));
        let lista = repeticiones(&semana, d("2026-11-18"), d("2026-11-25"));
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].fin.unwrap().a_texto(), "2026-11-20");
    }

    #[test]
    fn clase_semanal_con_fin_corre_las_dos_puntas() {
        let sin = HashSet::new();
        let mut clase = serie("2026-11-16T15:00:00-03:00", Recurrencia::Semanal, &sin);
        clase.fin = Some(m("2026-11-16T17:00:00-03:00"));
        let lista = repeticiones(&clase, d("2026-11-23"), d("2026-11-23"));
        assert_eq!(lista[0].fin.unwrap().a_texto(), "2026-11-23T17:00:00-03:00");
    }

    #[test]
    fn proxima_encuentra_la_siguiente_desde_una_fecha() {
        let sin = HashSet::new();
        let cumple = serie("2001-03-05", Recurrencia::Anual, &sin);
        assert_eq!(proxima(&cumple, d("2026-11-17")).unwrap().inicio.a_texto(), "2027-03-05");
        assert_eq!(proxima(&cumple, d("2027-03-05")).unwrap().inicio.a_texto(), "2027-03-05");
    }
}
