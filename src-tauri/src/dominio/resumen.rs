//! El resumen del día (SPEC 4.6) en una plantilla fija: sin IA, así es
//! instantáneo, no gasta tokens y siempre dice lo mismo con los mismos datos.
//! Lo usan la herramienta `resumen_del_dia` y la lectura en voz alta al abrir.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventoDelDia {
    pub titulo: String,
    /// "09:00"; `None` si es de todo el día.
    pub hora: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct DatosResumen {
    /// Ya pasó el mediodía o la tarde: cambia el saludo.
    pub saludo: String,
    pub eventos: Vec<EventoDelDia>,
    /// Pendientes que vencen hoy, de la más importante a la menos.
    pub para_hoy: Vec<String>,
    pub vencidas: usize,
    /// "Boca juega hoy a las 21:30 contra Racing (de local, la Liga Profesional)"
    pub futbol: Vec<String>,
    /// El título de la noticia más relevante.
    pub noticia: Option<String>,
    /// "22 grados y despejado, máxima de 26"
    pub clima: Option<String>,
}

fn lista(cosas: &[String]) -> String {
    match cosas {
        [] => String::new(),
        [una] => una.clone(),
        [primeras @ .., ultima] => format!("{} y {ultima}", primeras.join(", ")),
    }
}

fn evento(e: &EventoDelDia) -> String {
    match &e.hora {
        Some(h) => format!("{} a las {}", e.titulo, h.trim_start_matches('0').replace(":00", "")),
        None => e.titulo.clone(),
    }
}

/// El texto para leer en voz alta. Corto: como mucho unas cinco oraciones.
pub fn texto(d: &DatosResumen) -> String {
    let mut oraciones: Vec<String> = Vec::new();
    oraciones.push(format!("{}.", d.saludo));

    match d.eventos.as_slice() {
        [] => oraciones.push("Hoy no tenés nada en la agenda.".into()),
        eventos if eventos.len() <= 3 => {
            let textos: Vec<String> = eventos.iter().map(evento).collect();
            oraciones.push(format!("Hoy tenés {}.", lista(&textos)));
        }
        eventos => {
            let textos: Vec<String> = eventos.iter().take(2).map(evento).collect();
            oraciones.push(format!("Hoy tenés {} eventos, empezando por {}.", eventos.len(), lista(&textos)));
        }
    }

    match (d.para_hoy.as_slice(), d.vencidas) {
        ([], 0) => {}
        ([], v) => oraciones.push(format!("Tenés {} vencida{}.", cantidad(v), if v == 1 { "" } else { "s" })),
        ([una], v) => oraciones.push(format!("Para hoy: {una}{}.", vencidas_extra(v))),
        (varias, v) => oraciones.push(format!(
            "Tenés {} cosas para hoy; la primera, {}{}.",
            cantidad(varias.len()),
            varias[0],
            vencidas_extra(v)
        )),
    }

    for f in &d.futbol {
        oraciones.push(format!("{f}."));
    }
    if let Some(c) = &d.clima {
        oraciones.push(format!("Afuera, {c}."));
    }
    if let Some(n) = &d.noticia {
        oraciones.push(format!("Y entre las noticias: {n}."));
    }
    oraciones.join(" ").replace("..", ".")
}

fn vencidas_extra(v: usize) -> String {
    match v {
        0 => String::new(),
        1 => ", y hay una vencida".into(),
        v => format!(", y hay {} vencidas", cantidad(v)),
    }
}

/// Hasta diez, en palabras (se lee mejor en voz alta).
fn cantidad(n: usize) -> String {
    const PALABRAS: [&str; 11] = ["cero", "una", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez"];
    PALABRAS.get(n).map(|p| p.to_string()).unwrap_or_else(|| n.to_string())
}

/// "Buen día", "Buenas tardes" o "Buenas noches" según la hora.
pub fn saludo(hora: u32) -> &'static str {
    match hora {
        5..=12 => "Buen día",
        13..=19 => "Buenas tardes",
        _ => "Buenas noches",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(titulo: &str, hora: Option<&str>) -> EventoDelDia {
        EventoDelDia { titulo: titulo.into(), hora: hora.map(String::from) }
    }

    #[test]
    fn dia_vacio() {
        let d = DatosResumen { saludo: "Buen día".into(), ..Default::default() };
        assert_eq!(texto(&d), "Buen día. Hoy no tenés nada en la agenda.");
    }

    #[test]
    fn dia_completo() {
        let d = DatosResumen {
            saludo: "Buen día".into(),
            eventos: vec![ev("Parcial de Análisis", Some("09:00")), ev("Gimnasio", Some("19:30"))],
            para_hoy: vec!["Estudiar para Análisis".into(), "Llamar al dentista".into()],
            vencidas: 1,
            futbol: vec!["Boca juega hoy a las 21:30 contra Racing (de local, la Liga Profesional)".into()],
            noticia: Some("Battaglia, ante un desafío con sabor a revancha".into()),
            clima: Some("22 grados y despejado, máxima de 26".into()),
        };
        assert_eq!(
            texto(&d),
            "Buen día. Hoy tenés Parcial de Análisis a las 9 y Gimnasio a las 19:30. \
             Tenés dos cosas para hoy; la primera, Estudiar para Análisis, y hay una vencida. \
             Boca juega hoy a las 21:30 contra Racing (de local, la Liga Profesional). \
             Afuera, 22 grados y despejado, máxima de 26. \
             Y entre las noticias: Battaglia, ante un desafío con sabor a revancha."
        );
    }

    #[test]
    fn muchos_eventos_y_solo_vencidas() {
        let d = DatosResumen {
            saludo: "Buenas tardes".into(),
            eventos: vec![ev("A", None), ev("B", Some("10:00")), ev("C", None), ev("D", None)],
            vencidas: 3,
            ..Default::default()
        };
        assert_eq!(texto(&d), "Buenas tardes. Hoy tenés 4 eventos, empezando por A y B a las 10. Tenés tres vencidas.");
    }

    #[test]
    fn saludos() {
        assert_eq!(saludo(8), "Buen día");
        assert_eq!(saludo(15), "Buenas tardes");
        assert_eq!(saludo(23), "Buenas noches");
        assert_eq!(saludo(2), "Buenas noches");
    }
}
