//! Avisos del sistema (v0.6): qué notificar y cuándo. Lógica pura: recibe lo
//! que hay (eventos, responsabilidades, partidos), la hora y lo ya avisado, y
//! devuelve los avisos que tocan ahora. La tarea de fondo la llama cada minuto.
//!
//! Cada aviso tiene una clave única ("evento:12:2026-11-18T14:00…") para no
//! repetirlo, y una ventana en la que vale: si la compu estaba apagada o en
//! "no molestar" y la ventana ya pasó, se descarta en vez de llegar tarde.

use chrono::{DateTime, Duration, FixedOffset, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

use super::texto::TipoEvento;

/// Lo que se guarda en preferencias (clave `avisos`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesAvisos {
    /// Avisar antes de cada evento con hora.
    pub eventos: bool,
    /// Cuántos minutos antes: 10, 15, 30 o 60.
    pub minutos_antes: u32,
    /// "Hoy vence…", todo junto, a la hora de `hora_vencimientos`.
    pub vencimientos: bool,
    /// "09:00".
    pub hora_vencimientos: String,
    /// "Arranca Boca" al empezar el partido.
    pub partidos: bool,
    /// "Hora de salir" antes de la primera clase o examen del día.
    pub salir: bool,
    /// Lo que tardás en llegar (en minutos).
    pub minutos_viaje: u32,
    /// Sin avisos de `desde` a `hasta` (puede cruzar la medianoche).
    pub no_molestar: bool,
    pub desde: String,
    pub hasta: String,
    /// En "no molestar" tampoco escucha "Oye Nova".
    pub no_molestar_pausa_palabra: bool,
    /// Además de la notificación, Nova lo dice en voz alta.
    pub en_voz: bool,
}

impl Default for AjustesAvisos {
    fn default() -> Self {
        AjustesAvisos {
            eventos: true,
            minutos_antes: 30,
            vencimientos: true,
            hora_vencimientos: "09:00".into(),
            partidos: true,
            salir: true,
            minutos_viaje: 60,
            no_molestar: true,
            desde: "23:00".into(),
            hasta: "08:00".into(),
            no_molestar_pausa_palabra: true,
            en_voz: false,
        }
    }
}

pub const MINUTOS_ANTES_VALIDOS: [u32; 4] = [10, 15, 30, 60];

fn hora(texto: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(texto.trim(), "%H:%M").ok()
}

impl AjustesAvisos {
    pub fn validar(mut self) -> Result<Self, String> {
        if !MINUTOS_ANTES_VALIDOS.contains(&self.minutos_antes) {
            return Err("El aviso antes de un evento puede ser de 10, 15, 30 o 60 minutos.".into());
        }
        if !(5..=240).contains(&self.minutos_viaje) {
            return Err("El viaje tiene que durar entre 5 minutos y 4 horas.".into());
        }
        for (texto, nombre) in
            [(&mut self.hora_vencimientos, "del aviso de vencimientos"), (&mut self.desde, "de \"no molestar\""), (&mut self.hasta, "de \"no molestar\"")]
        {
            let Some(h) = hora(texto) else { return Err(format!("La hora {nombre} va como 09:00.")) };
            *texto = h.format("%H:%M").to_string();
        }
        if self.desde == self.hasta {
            return Err("\"No molestar\" tiene que empezar y terminar a horas distintas.".into());
        }
        Ok(self)
    }

    /// ¿Esta hora cae en "no molestar"? (23:00 a 08:00 cruza la medianoche).
    pub fn en_no_molestar(&self, ahora: DateTime<FixedOffset>) -> bool {
        if !self.no_molestar {
            return false;
        }
        let (Some(desde), Some(hasta)) = (hora(&self.desde), hora(&self.hasta)) else { return false };
        let t = ahora.time();
        if desde < hasta { desde <= t && t < hasta } else { t >= desde || t < hasta }
    }
}

/// Un evento con hora de los próximos días.
#[derive(Debug, Clone)]
pub struct EventoProximo {
    pub id: i64,
    pub titulo: String,
    pub tipo: TipoEvento,
    pub inicio: DateTime<FixedOffset>,
}

/// Un partido de un equipo seguido.
#[derive(Debug, Clone)]
pub struct PartidoProximo {
    pub id: String,
    /// "Boca", "la Selección".
    pub equipo: String,
    pub rival: String,
    pub competencia: String,
    pub inicio: DateTime<FixedOffset>,
}

/// Lo que hay para revisar.
#[derive(Debug, Clone, Default)]
pub struct Pendientes {
    pub eventos: Vec<EventoProximo>,
    /// Títulos de lo pendiente que vence hoy.
    pub vencen_hoy: Vec<String>,
    pub partidos: Vec<PartidoProximo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aviso {
    pub clave: String,
    pub titulo: String,
    pub cuerpo: String,
}

/// Un partido se avisa hasta media hora después de que arrancó.
const MINUTOS_PARTIDO_VALE: i64 = 30;
/// "Hora de salir" vale hasta 10 minutos antes del evento (después ya no sirve).
const MINUTOS_SALIR_ULTIMO: i64 = 10;

/// "9:00", "14:30".
fn hora_corta(t: DateTime<FixedOffset>) -> String {
    format!("{}:{:02}", t.hour(), t.minute())
}

fn minutos_en_texto(m: u32) -> String {
    match m {
        60 => "1 hora".into(),
        m if m % 60 == 0 => format!("{} horas", m / 60),
        m if m > 60 => format!("{} h {} min", m / 60, m % 60),
        m => format!("{m} minutos"),
    }
}

/// "A", "A y B", "A, B y C".
fn enumerar(cosas: &[String]) -> String {
    match cosas {
        [] => String::new(),
        [una] => una.clone(),
        [resto @ .., ultima] => format!("{} y {ultima}", resto.join(", ")),
    }
}

/// Los avisos que tocan ahora y no se mandaron (`ya_enviado` dice si una clave
/// ya salió). En "no molestar" no devuelve nada: lo que siga valiendo al
/// terminar se manda en ese momento.
pub fn que_avisar(
    pendientes: &Pendientes,
    ajustes: &AjustesAvisos,
    ahora: DateTime<FixedOffset>,
    ya_enviado: impl Fn(&str) -> bool,
) -> Vec<Aviso> {
    if ajustes.en_no_molestar(ahora) {
        return Vec::new();
    }
    let mut avisos = Vec::new();
    let mut agregar = |clave: String, titulo: String, cuerpo: String| {
        if !ya_enviado(&clave) {
            avisos.push(Aviso { clave, titulo, cuerpo });
        }
    };

    // Hora de salir: la primera clase o examen de cada día.
    if ajustes.salir {
        let mut primeras: Vec<&EventoProximo> = Vec::new();
        for e in pendientes.eventos.iter().filter(|e| matches!(e.tipo, TipoEvento::Clase | TipoEvento::Examen)) {
            match primeras.iter_mut().find(|p| p.inicio.date_naive() == e.inicio.date_naive()) {
                Some(p) if e.inicio < p.inicio => *p = e,
                Some(_) => {}
                None => primeras.push(e),
            }
        }
        let viaje = Duration::minutes(i64::from(ajustes.minutos_viaje));
        for e in primeras {
            let desde = e.inicio - viaje;
            if desde <= ahora && ahora < e.inicio - Duration::minutes(MINUTOS_SALIR_ULTIMO) {
                agregar(
                    format!("salir:{}", e.inicio.date_naive()),
                    "Hora de salir".into(),
                    format!("{} a las {}. Tardás {} en llegar.", e.titulo, hora_corta(e.inicio), minutos_en_texto(ajustes.minutos_viaje)),
                );
            }
        }
    }

    if ajustes.eventos {
        let antes = Duration::minutes(i64::from(ajustes.minutos_antes));
        for e in &pendientes.eventos {
            if e.inicio - antes <= ahora && ahora < e.inicio {
                let faltan = ((e.inicio - ahora).num_seconds() + 59) / 60;
                let cuando = if faltan <= 1 { "ya".to_string() } else { format!("en {}", minutos_en_texto(faltan as u32)) };
                agregar(
                    format!("evento:{}:{}", e.id, e.inicio.to_rfc3339()),
                    e.titulo.clone(),
                    format!("Empieza {cuando}, a las {}.", hora_corta(e.inicio)),
                );
            }
        }
    }

    if ajustes.vencimientos
        && !pendientes.vencen_hoy.is_empty()
        && hora(&ajustes.hora_vencimientos).is_some_and(|h| ahora.time() >= h)
    {
        let n = pendientes.vencen_hoy.len();
        agregar(
            format!("vence:{}", ahora.date_naive()),
            if n == 1 { "Hoy vence".into() } else { format!("Hoy vencen {n} cosas") },
            format!("{}.", enumerar(&pendientes.vencen_hoy)),
        );
    }

    if ajustes.partidos {
        for p in &pendientes.partidos {
            if p.inicio <= ahora && ahora < p.inicio + Duration::minutes(MINUTOS_PARTIDO_VALE) {
                agregar(
                    format!("partido:{}", p.id),
                    format!("Arranca {}", p.equipo),
                    format!("Contra {} · {}", p.rival, p.competencia),
                );
            }
        }
    }
    avisos
}

/// Lo que dice Nova si los avisos van también en voz.
pub fn para_decir(aviso: &Aviso) -> String {
    format!("{}. {}", aviso.titulo, aviso.cuerpo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(texto: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(&format!("2026-11-19T{texto}:00-03:00")).unwrap()
    }

    fn evento(id: i64, titulo: &str, tipo: TipoEvento, inicio: &str) -> EventoProximo {
        EventoProximo { id, titulo: titulo.into(), tipo, inicio: t(inicio) }
    }

    fn claves(avisos: &[Aviso]) -> Vec<String> {
        avisos.iter().map(|a| a.clave.split(':').next().unwrap().to_string()).collect()
    }

    fn nada_enviado(_: &str) -> bool {
        false
    }

    #[test]
    fn avisa_media_hora_antes_de_un_evento() {
        let p = Pendientes { eventos: vec![evento(1, "Dentista", TipoEvento::Otro, "14:00")], ..Default::default() };
        let a = AjustesAvisos::default();
        assert!(que_avisar(&p, &a, t("13:29"), nada_enviado).is_empty());
        let avisos = que_avisar(&p, &a, t("13:30"), nada_enviado);
        assert_eq!(avisos.len(), 1);
        assert_eq!(avisos[0].titulo, "Dentista");
        assert_eq!(avisos[0].cuerpo, "Empieza en 30 minutos, a las 14:00.");
        // Si ya empezó, no.
        assert!(que_avisar(&p, &a, t("14:00"), nada_enviado).is_empty());
    }

    #[test]
    fn no_repite_lo_ya_avisado() {
        let p = Pendientes { eventos: vec![evento(1, "Dentista", TipoEvento::Otro, "14:00")], ..Default::default() };
        let a = AjustesAvisos::default();
        let clave = que_avisar(&p, &a, t("13:30"), nada_enviado)[0].clave.clone();
        assert!(que_avisar(&p, &a, t("13:40"), |c| c == clave).is_empty());
    }

    #[test]
    fn hora_de_salir_para_la_primera_clase() {
        let p = Pendientes {
            eventos: vec![
                evento(1, "Análisis II", TipoEvento::Clase, "09:00"),
                evento(2, "Física", TipoEvento::Clase, "11:00"),
            ],
            ..Default::default()
        };
        let a = AjustesAvisos { eventos: false, ..Default::default() };
        let avisos = que_avisar(&p, &a, t("08:00"), nada_enviado);
        assert_eq!(avisos.len(), 1);
        assert_eq!(avisos[0].titulo, "Hora de salir");
        assert_eq!(avisos[0].cuerpo, "Análisis II a las 9:00. Tardás 1 hora en llegar.");
        // A las 10 ya no tiene sentido (y Física no es la primera).
        assert!(que_avisar(&p, &a, t("10:00"), nada_enviado).is_empty());
    }

    #[test]
    fn vencimientos_juntos_a_la_hora_elegida() {
        let p = Pendientes { vencen_hoy: vec!["TP de Física".into(), "Pagar la luz".into()], ..Default::default() };
        let a = AjustesAvisos::default();
        assert!(que_avisar(&p, &a, t("08:59"), nada_enviado).is_empty());
        let avisos = que_avisar(&p, &a, t("15:00"), nada_enviado);
        assert_eq!(avisos[0].titulo, "Hoy vencen 2 cosas");
        assert_eq!(avisos[0].cuerpo, "TP de Física y Pagar la luz.");
        assert_eq!(avisos[0].clave, "vence:2026-11-19");
    }

    #[test]
    fn arranca_boca() {
        let p = Pendientes {
            partidos: vec![PartidoProximo {
                id: "401".into(),
                equipo: "Boca".into(),
                rival: "River".into(),
                competencia: "Liga Profesional".into(),
                inicio: t("17:00"),
            }],
            ..Default::default()
        };
        let a = AjustesAvisos::default();
        assert!(que_avisar(&p, &a, t("16:59"), nada_enviado).is_empty());
        let avisos = que_avisar(&p, &a, t("17:01"), nada_enviado);
        assert_eq!(avisos[0].titulo, "Arranca Boca");
        assert_eq!(avisos[0].cuerpo, "Contra River · Liga Profesional");
        // Pasada la media hora ya no.
        assert!(que_avisar(&p, &a, t("17:31"), nada_enviado).is_empty());
    }

    #[test]
    fn no_molestar_cruza_la_medianoche_y_lo_vencido_se_descarta() {
        let a = AjustesAvisos::default();
        assert!(a.en_no_molestar(t("23:30")));
        assert!(a.en_no_molestar(t("03:00")));
        assert!(!a.en_no_molestar(t("08:00")));
        assert!(!AjustesAvisos { no_molestar: false, ..a.clone() }.en_no_molestar(t("03:00")));

        // Un evento a las 7:00: su aviso (6:30) cae en no molestar y a las 8 ya pasó.
        let p = Pendientes { eventos: vec![evento(1, "Gimnasio", TipoEvento::Otro, "07:00")], ..Default::default() };
        assert!(que_avisar(&p, &a, t("06:30"), nada_enviado).is_empty());
        assert!(que_avisar(&p, &a, t("08:00"), nada_enviado).is_empty());
        // Uno a las 8:20: al terminar no molestar todavía vale.
        let p = Pendientes { eventos: vec![evento(2, "Colectivo", TipoEvento::Otro, "08:20")], ..Default::default() };
        assert_eq!(claves(&que_avisar(&p, &a, t("08:00"), nada_enviado)), ["evento"]);
    }

    #[test]
    fn se_apaga_cada_tipo() {
        let p = Pendientes {
            eventos: vec![evento(1, "Análisis II", TipoEvento::Clase, "09:00")],
            vencen_hoy: vec!["TP".into()],
            ..Default::default()
        };
        let todo = AjustesAvisos::default();
        assert_eq!(claves(&que_avisar(&p, &todo, t("09:00") - Duration::minutes(20), nada_enviado)), ["salir", "evento"]);
        let sin = AjustesAvisos { eventos: false, salir: false, vencimientos: false, ..todo };
        assert!(que_avisar(&p, &sin, t("10:00"), nada_enviado).is_empty());
    }

    #[test]
    fn valida_los_ajustes() {
        assert!(AjustesAvisos::default().validar().is_ok());
        assert!(AjustesAvisos { minutos_antes: 20, ..Default::default() }.validar().is_err());
        assert!(AjustesAvisos { desde: "25:00".into(), ..Default::default() }.validar().is_err());
        let a = AjustesAvisos { hora_vencimientos: " 9:00 ".into(), ..Default::default() }.validar().unwrap();
        assert_eq!(a.hora_vencimientos, "09:00");
        assert!(AjustesAvisos { desde: "08:00".into(), hasta: "08:00".into(), ..Default::default() }.validar().is_err());
    }

    #[test]
    fn textos() {
        assert_eq!(minutos_en_texto(15), "15 minutos");
        assert_eq!(minutos_en_texto(90), "1 h 30 min");
        assert_eq!(minutos_en_texto(120), "2 horas");
        assert_eq!(enumerar(&["A".into(), "B".into(), "C".into()]), "A, B y C");
    }
}
