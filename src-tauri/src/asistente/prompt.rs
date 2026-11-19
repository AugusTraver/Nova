//! Las instrucciones de sistema de cada turno: quién es Nova, las reglas de
//! SPEC 5.2, la fecha y hora de ahora y un resumen corto de los datos.
//!
//! Se cuida el largo: el plan gratis de Groq cuenta tokens por minuto.

use chrono::{Datelike, Duration, NaiveDate, Timelike, Weekday};

use crate::dominio::fechas_relativas::{nombre_dia_con_tilde, nombre_mes};
use crate::dominio::momento::Momento;
use crate::dominio::perfil::Perfil;
use crate::repos::Ahora;
use crate::repos::eventos::Ocurrencia;
use crate::repos::materias::Materia;
use crate::repos::responsabilidades::Responsabilidad;

/// Lo que el modelo ve de los datos en cada turno.
pub struct Contexto {
    pub materias: Vec<Materia>,
    /// Próximos días.
    pub eventos: Vec<Ocurrencia>,
    /// Pendientes, ya ordenadas (vencidas y próximas primero).
    pub pendientes: Vec<Responsabilidad>,
    /// Cómo llamarlo y cómo hablarle.
    pub perfil: Perfil,
}

pub const DIAS_DE_AGENDA: i64 = 7;
const MAXIMO_EVENTOS: usize = 15;
const MAXIMO_PENDIENTES: usize = 10;

const REGLAS: &str = "\
Sos Nova, el asistente personal que vive en el escritorio del usuario. Hablás en español rioplatense \
(vos, tenés, dale), con onda y muy breve: una o dos oraciones, sin listas largas ni markdown.

Reglas:
- Para crear o cambiar datos usá las herramientas. Nunca digas que hiciste algo si la herramienta no lo confirmó.
- La agenda y las pendientes de abajo están al día: para consultas de esos días contestá con eso, sin llamar herramientas.
- En fechas y horas pasale a la herramienta lo que dijo el usuario (\"el jueves\", \"a las 9\"): el sistema las resuelve.
- Si falta un dato importante (qué día, la hora de una clase o examen, la materia ambigua), preguntá antes de crear. No inventes.
- Para borrar usá borrar_evento: el usuario confirma con un botón. Hasta que confirme, no está borrado.
- Para editar o borrar necesitás el id: está en la agenda de abajo o buscalo con listar_eventos.
- Si una herramienta devuelve \"error\", contalo corto o preguntá lo que falta.
- Fútbol (Boca, la Selección y los jugadores que sigue) y noticias: consultalos con sus herramientas, no los sepas de memoria. Si traen \"aviso\", aclará que puede no estar al día.
- Al terminar, confirmá qué hiciste y cuándo (\"Listo, parcial de Análisis el jueves 19 a las 9\"). \
Si se crearon responsabilidades automáticas, nombralas.";

/// Se agrega cuando el pedido llegó hablando: la respuesta se va a leer.
pub const POR_VOZ: &str = "\n\nEl usuario te habló y tu respuesta se va a leer en voz alta: una sola oración corta, \
sin listas, emojis, símbolos ni números con decimales. Las horas, como se dicen (\"a las nueve\", \"a las siete y media\").";

fn dia_corto(dia: Weekday) -> &'static str {
    match dia {
        Weekday::Mon => "lun",
        Weekday::Tue => "mar",
        Weekday::Wed => "mié",
        Weekday::Thu => "jue",
        Weekday::Fri => "vie",
        Weekday::Sat => "sáb",
        Weekday::Sun => "dom",
    }
}

fn fecha_corta(f: NaiveDate) -> String {
    format!("{} {}/{}", dia_corto(f.weekday()), f.day(), f.month())
}

pub fn armar(ahora: &Ahora, contexto: &Contexto) -> String {
    let hoy = ahora.hoy;
    let mut texto = String::from(REGLAS);
    texto.push_str(&contexto.perfil.instrucciones());

    texto.push_str(&format!(
        "\n\nAhora: {} {} de {} de {}, {:02}:{:02} (huso {}).\nPróximos días: {}.",
        nombre_dia_con_tilde(hoy.weekday()),
        hoy.day(),
        nombre_mes(hoy.month()),
        hoy.year(),
        ahora.momento.hour(),
        ahora.momento.minute(),
        ahora.momento.offset(),
        (1..=14).map(|n| fecha_corta(hoy + Duration::days(n))).collect::<Vec<_>>().join(", ")
    ));

    texto.push_str("\n\nMaterias: ");
    if contexto.materias.is_empty() {
        texto.push_str("ninguna todavía.");
    } else {
        let nombres: Vec<&str> = contexto.materias.iter().map(|m| m.nombre.as_str()).collect();
        texto.push_str(&nombres.join(", "));
        texto.push('.');
    }

    let nombre_materia = |id: Option<i64>| {
        id.and_then(|id| contexto.materias.iter().find(|m| m.id == id)).map(|m| m.nombre.as_str())
    };

    texto.push_str(&format!("\n\nAgenda de los próximos {DIAS_DE_AGENDA} días:"));
    if contexto.eventos.is_empty() {
        texto.push_str(" nada.");
    }
    for o in contexto.eventos.iter().take(MAXIMO_EVENTOS) {
        let fecha = crate::repos::fecha(&o.fecha).map(fecha_corta).unwrap_or_else(|| o.fecha.clone());
        let hora = match Momento::parsear(&o.inicio) {
            Ok(Momento::FechaHora(fh)) => format!(" {}", fh.format("%H:%M")),
            _ => String::new(),
        };
        let mut detalle = vec![o.evento.tipo.como_texto()];
        if let Some(m) = nombre_materia(o.evento.materia_id) {
            detalle.push(m);
        }
        texto.push_str(&format!("\n- [id {}] {fecha}{hora} {} ({})", o.evento.id, o.evento.titulo, detalle.join(", ")));
    }
    if contexto.eventos.len() > MAXIMO_EVENTOS {
        texto.push_str(&format!("\n- y {} más (usá listar_eventos).", contexto.eventos.len() - MAXIMO_EVENTOS));
    }

    texto.push_str("\n\nPendientes:");
    if contexto.pendientes.is_empty() {
        texto.push_str(" ninguna.");
    }
    for r in contexto.pendientes.iter().take(MAXIMO_PENDIENTES) {
        let vence = match r.fecha_limite.as_deref().and_then(crate::repos::fecha) {
            Some(f) if f < hoy => format!(" (vencida, era {})", fecha_corta(f)),
            Some(f) => format!(" (vence {})", fecha_corta(f)),
            None => String::new(),
        };
        texto.push_str(&format!("\n- [id {}] {}{vence}", r.id, r.titulo));
    }
    if contexto.pendientes.len() > MAXIMO_PENDIENTES {
        texto.push_str(&format!(
            "\n- y {} más (usá listar_responsabilidades).",
            contexto.pendientes.len() - MAXIMO_PENDIENTES
        ));
    }
    texto
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::dominio::texto::{OrigenResponsabilidad, TipoEvento};
    use crate::repos::materias::{self, DatosMateria};
    use crate::repos::{eventos, fecha, responsabilidades};

    #[test]
    fn trae_la_fecha_los_dias_y_los_datos_con_ids() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo_a_las("2026-11-17", "14:05");
        let analisis =
            materias::crear(&base, DatosMateria { nombre: "Análisis II".into(), color: "celeste".into() }, &ahora).unwrap();
        let mut datos = eventos::tests::datos("Parcial", TipoEvento::Examen, "2026-11-19T09:00:00-03:00");
        datos.materia_id = Some(analisis.id);
        eventos::crear(&mut base, datos, &ahora).unwrap();
        responsabilidades::crear(
            &base,
            responsabilidades::tests::datos("Pagar la luz", Some("2026-11-16")),
            OrigenResponsabilidad::Manual,
            &ahora,
        )
        .unwrap();

        let contexto = Contexto {
            materias: materias::listar(&base).unwrap(),
            eventos: eventos::ocurrencias(&base, fecha("2026-11-17").unwrap(), fecha("2026-11-24").unwrap()).unwrap(),
            pendientes: responsabilidades::listar(&base).unwrap(),
            perfil: Perfil { como_llamarte: "Juli".into(), ..Perfil::default() },
        };
        let texto = armar(&ahora, &contexto);
        assert!(texto.contains("Sobre el usuario:\n- Llamalo Juli."));

        assert!(texto.contains("Ahora: martes 17 de noviembre de 2026, 14:05 (huso -03:00)."));
        assert!(texto.contains("Próximos días: mié 18/11, jue 19/11, vie 20/11"));
        assert!(texto.contains("Materias: Análisis II."));
        assert!(texto.contains("- [id 1] jue 19/11 09:00 Parcial (examen, Análisis II)"));
        assert!(texto.contains("Estudiar para Análisis II (vence mié 18/11)"));
        assert!(texto.contains("Pagar la luz (vencida, era lun 16/11)"));
    }

    #[test]
    fn sin_datos_lo_dice_y_es_corto() {
        let contexto = Contexto { materias: vec![], eventos: vec![], pendientes: vec![], perfil: Perfil::default() };
        let texto = armar(&Ahora::fijo("2026-11-17"), &contexto);
        assert!(!texto.contains("Sobre el usuario"));
        assert!(texto.contains("Materias: ninguna todavía."));
        assert!(texto.contains("Agenda de los próximos 7 días: nada."));
        assert!(texto.contains("Pendientes: ninguna."));
        // Unos 400 tokens: entra cómodo en el límite por minuto.
        assert!(texto.chars().count() < 2000, "largo {}", texto.chars().count());
    }
}
