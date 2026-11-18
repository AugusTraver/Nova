//! Reglas de responsabilidades automáticas (SPEC 4.2).
//!
//! Dado un evento, decide qué responsabilidad automática "debería" existir:
//! título, fecha límite y prioridad. Es lógica pura; qué hacer con las que ya
//! existen en la base lo resuelve `repos::automaticas`.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use super::texto::{Prioridad, TipoEvento, TipoRegla};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Regla {
    pub activa: bool,
    /// Cuántos días antes del evento vence la responsabilidad.
    pub dias_antes: u32,
    /// Título con marcadores: {materia}, {entrega}, {persona}, {titulo}.
    pub plantilla: String,
}

/// Las tres reglas configurables. Se guardan en `preferencias` como JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reglas {
    pub examen: Regla,
    pub entrega: Regla,
    pub cumpleanos: Regla,
}

impl Default for Reglas {
    fn default() -> Self {
        let regla = |dias_antes, plantilla: &str| Regla { activa: true, dias_antes, plantilla: plantilla.into() };
        Reglas {
            examen: regla(1, "Estudiar para {materia}"),
            entrega: regla(1, "Terminar {entrega}"),
            cumpleanos: regla(3, "Comprar regalo para {persona}"),
        }
    }
}

impl Reglas {
    /// Qué regla aplica a un tipo de evento (si aplica alguna).
    pub fn para(&self, tipo: TipoEvento) -> Option<(TipoRegla, &Regla)> {
        match tipo {
            TipoEvento::Examen => Some((TipoRegla::Examen, &self.examen)),
            TipoEvento::Entrega => Some((TipoRegla::Entrega, &self.entrega)),
            TipoEvento::Cumpleanos => Some((TipoRegla::Cumpleanos, &self.cumpleanos)),
            _ => None,
        }
    }

    pub fn validar(&self) -> Result<(), String> {
        for (nombre, regla) in [("exámenes", &self.examen), ("entregas", &self.entrega), ("cumpleaños", &self.cumpleanos)] {
            if regla.dias_antes > 60 {
                return Err(format!("La regla de {nombre} no puede avisar más de 60 días antes."));
            }
            if regla.plantilla.trim().is_empty() {
                return Err(format!("La regla de {nombre} necesita un título."));
            }
        }
        Ok(())
    }
}

/// Lo que importa de un evento para las reglas.
pub struct DatosEvento<'a> {
    pub tipo: TipoEvento,
    pub titulo: &'a str,
    pub persona: Option<&'a str>,
    /// Nombre de la materia, si el evento tiene una.
    pub materia: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Deseada {
    pub regla: TipoRegla,
    pub titulo: String,
    /// Fecha de la repetición del evento a la que corresponde.
    pub evento_fecha: NaiveDate,
    /// Fecha límite según la regla, sin ajustar (ver `fecha_limite`).
    pub fecha_limite_ideal: NaiveDate,
    pub prioridad: Prioridad,
}

/// La responsabilidad automática que corresponde a la próxima repetición de
/// un evento, o `None` si su tipo no tiene regla, la regla está apagada o el
/// evento ya pasó.
pub fn deseada(evento: &DatosEvento, proxima_fecha: Option<NaiveDate>, reglas: &Reglas) -> Option<Deseada> {
    let (tipo_regla, regla) = reglas.para(evento.tipo)?;
    if !regla.activa {
        return None;
    }
    let evento_fecha = proxima_fecha?;
    Some(Deseada {
        regla: tipo_regla,
        titulo: completar_plantilla(&regla.plantilla, evento),
        evento_fecha,
        fecha_limite_ideal: evento_fecha - Duration::days(i64::from(regla.dias_antes)),
        prioridad: match tipo_regla {
            TipoRegla::Examen | TipoRegla::Entrega => Prioridad::Alta,
            TipoRegla::Cumpleanos => Prioridad::Media,
        },
    })
}

/// La fecha límite nunca queda antes de `piso` (el día en que se creó la
/// responsabilidad). Si cargás hoy un cumpleaños que es pasado mañana, "comprar
/// regalo" vence hoy en vez de figurar como vencida desde ayer.
pub fn fecha_limite(ideal: NaiveDate, piso: NaiveDate) -> NaiveDate {
    ideal.max(piso)
}

/// Reemplaza los marcadores. Si falta el dato (un examen sin materia, un
/// cumpleaños sin persona), usa el título del evento.
fn completar_plantilla(plantilla: &str, evento: &DatosEvento) -> String {
    let o_titulo = |valor: Option<&str>| {
        valor
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or(evento.titulo)
            .to_string()
    };
    plantilla
        .replace("{materia}", &o_titulo(evento.materia))
        .replace("{persona}", &o_titulo(evento.persona))
        .replace("{entrega}", evento.titulo)
        .replace("{titulo}", evento.titulo)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    fn evento<'a>(tipo: TipoEvento, titulo: &'a str) -> DatosEvento<'a> {
        DatosEvento { tipo, titulo, persona: None, materia: None }
    }

    #[test]
    fn examen_estudiar_un_dia_antes() {
        let mut parcial = evento(TipoEvento::Examen, "Primer parcial");
        parcial.materia = Some("Análisis II");
        let r = deseada(&parcial, Some(d("2026-11-19")), &Reglas::default()).unwrap();
        assert_eq!(r.titulo, "Estudiar para Análisis II");
        assert_eq!(r.fecha_limite_ideal, d("2026-11-18"));
        assert_eq!(r.prioridad, Prioridad::Alta);
        assert_eq!(r.regla, TipoRegla::Examen);
    }

    #[test]
    fn entrega_terminar_un_dia_antes() {
        let tp = evento(TipoEvento::Entrega, "TP de Algoritmos");
        let r = deseada(&tp, Some(d("2026-11-18")), &Reglas::default()).unwrap();
        assert_eq!(r.titulo, "Terminar TP de Algoritmos");
        assert_eq!(r.fecha_limite_ideal, d("2026-11-17"));
    }

    #[test]
    fn cumpleanos_regalo_tres_dias_antes() {
        let mut cumple = evento(TipoEvento::Cumpleanos, "Cumple de Juan");
        cumple.persona = Some("Juan");
        let r = deseada(&cumple, Some(d("2026-11-20")), &Reglas::default()).unwrap();
        assert_eq!(r.titulo, "Comprar regalo para Juan");
        assert_eq!(r.fecha_limite_ideal, d("2026-11-17"));
        assert_eq!(r.prioridad, Prioridad::Media);
    }

    #[test]
    fn sin_materia_o_persona_usa_el_titulo() {
        let cumple = evento(TipoEvento::Cumpleanos, "Cumple de la abuela");
        let r = deseada(&cumple, Some(d("2026-11-20")), &Reglas::default()).unwrap();
        assert_eq!(r.titulo, "Comprar regalo para Cumple de la abuela");

        let mut parcial = evento(TipoEvento::Examen, "Final de Física");
        parcial.materia = Some("  ");
        let r = deseada(&parcial, Some(d("2026-11-20")), &Reglas::default()).unwrap();
        assert_eq!(r.titulo, "Estudiar para Final de Física");
    }

    #[test]
    fn otros_tipos_no_generan_nada() {
        for tipo in [TipoEvento::Clase, TipoEvento::Partido, TipoEvento::Otro] {
            assert!(deseada(&evento(tipo, "x"), Some(d("2026-11-20")), &Reglas::default()).is_none());
        }
    }

    #[test]
    fn regla_apagada_o_evento_pasado_no_generan_nada() {
        let mut reglas = Reglas::default();
        reglas.examen.activa = false;
        let parcial = evento(TipoEvento::Examen, "Parcial");
        assert!(deseada(&parcial, Some(d("2026-11-20")), &reglas).is_none());
        assert!(deseada(&parcial, None, &Reglas::default()).is_none());
    }

    #[test]
    fn respeta_dias_y_plantilla_configurados() {
        let mut reglas = Reglas::default();
        reglas.examen.dias_antes = 7;
        reglas.examen.plantilla = "Repasar {materia} ({titulo})".into();
        let mut parcial = evento(TipoEvento::Examen, "Recuperatorio");
        parcial.materia = Some("Física");
        let r = deseada(&parcial, Some(d("2026-11-20")), &reglas).unwrap();
        assert_eq!(r.titulo, "Repasar Física (Recuperatorio)");
        assert_eq!(r.fecha_limite_ideal, d("2026-11-13"));
    }

    #[test]
    fn la_fecha_limite_no_queda_antes_de_crearla() {
        assert_eq!(fecha_limite(d("2026-11-16"), d("2026-11-17")), d("2026-11-17"));
        assert_eq!(fecha_limite(d("2026-11-19"), d("2026-11-17")), d("2026-11-19"));
    }

    #[test]
    fn valida_las_reglas() {
        assert!(Reglas::default().validar().is_ok());
        let mut reglas = Reglas::default();
        reglas.entrega.dias_antes = 90;
        assert!(reglas.validar().is_err());
        let mut reglas = Reglas::default();
        reglas.cumpleanos.plantilla = " ".into();
        assert!(reglas.validar().is_err());
    }
}
