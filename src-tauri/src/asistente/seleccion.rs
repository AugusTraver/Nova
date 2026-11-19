//! Qué herramientas se le mandan al modelo en cada pedido.
//!
//! Las definiciones de las 14 herramientas pesan unos 800 tokens y se mandan
//! en cada vuelta. El plan gratis de Groq cuenta tokens por minuto, así que
//! se mandan solo las del tema del pedido ("¿cuándo juega Boca?" no necesita
//! las de la agenda). Es por palabras clave: si no se reconoce el tema, van
//! todas (mejor gastar de más que dejar al modelo sin poder hacer algo).

use crate::dominio::lenguaje::normalizar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tema {
    Agenda,
    Responsabilidades,
    Materias,
    Futbol,
    Noticias,
    Resumen,
}

impl Tema {
    const TODOS: [Tema; 6] =
        [Tema::Agenda, Tema::Responsabilidades, Tema::Materias, Tema::Futbol, Tema::Noticias, Tema::Resumen];

    fn herramientas(self) -> &'static [&'static str] {
        match self {
            Tema::Agenda => &["crear_evento", "editar_evento", "borrar_evento", "listar_eventos"],
            Tema::Responsabilidades => &["crear_responsabilidad", "completar_responsabilidad", "listar_responsabilidades"],
            Tema::Materias => &["crear_materia", "listar_materias"],
            Tema::Futbol => &["consultar_partidos", "consultar_tabla", "consultar_jugador"],
            Tema::Noticias => &["consultar_noticias"],
            Tema::Resumen => &["resumen_del_dia"],
        }
    }

    /// Palabras (sin tildes) que lo delatan. Con `*` al final, cualquier
    /// palabra que empiece así ("agend*" = agenda, agendame, agendá…). Con un
    /// espacio, una frase.
    fn palabras(self) -> &'static [&'static str] {
        match self {
            Tema::Agenda => &[
                "agend*", "evento*", "parcial*", "examen*", "final*", "clase*", "entrega*", "turno*", "reunion*",
                "cumple*", "cita*", "dentista", "medico*", "calendario", "cancel*", "borr*", "elimin*", "mov*",
                "cambi*", "pasa*", "corre*", "a las", "de la tarde", "de la manana", "de la noche", "lunes", "martes",
                "miercoles", "jueves", "viernes", "sabado*", "domingo*", "manana", "pasado", "semana*", "hoy",
                "hora*", "dia", "dias", "fecha*",
            ],
            Tema::Responsabilidades => &[
                "record*", "acorda*", "anot*", "tarea*", "pendiente*", "responsabilidad*", "hice", "hiciste",
                "termin*", "hecha*", "hecho", "complet*", "compr*", "pag*", "estudi*", "tengo que", "hacer",
                "vence*", "vencid*",
            ],
            Tema::Materias => &["materia*", "facu*", "cursada*", "cuatri*"],
            Tema::Futbol => &[
                "boca", "seleccion", "partido*", "jueg*", "jug*", "gan*", "perdi*", "empat*", "gol*", "tabla",
                "torneo", "posicion*", "futbol", "messi", "scaloni", "resultado*", "fecha",
            ],
            Tema::Noticias => &["noticia*", "titular*", "diario*", "que paso", "novedad*", "pasando"],
            Tema::Resumen => &["que tengo", "mi dia", "resumen*", "resumi*", "como viene", "hoy"],
        }
    }

    fn aparece_en(self, palabras: &[String], frase: &str) -> bool {
        self.palabras().iter().any(|clave| {
            if clave.contains(' ') {
                frase.contains(clave)
            } else if let Some(prefijo) = clave.strip_suffix('*') {
                palabras.iter().any(|p| p.starts_with(prefijo))
            } else {
                palabras.iter().any(|p| p == clave)
            }
        })
    }
}

/// Las palabras del texto, sin tildes ni signos.
fn palabras(texto: &str) -> Vec<String> {
    normalizar(texto).split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()).map(String::from).collect()
}

fn temas(texto: &str) -> Vec<Tema> {
    let palabras = palabras(texto);
    let frase = format!(" {} ", palabras.join(" "));
    Tema::TODOS.into_iter().filter(|t| t.aparece_en(&palabras, &frase)).collect()
}

/// ¿Es solo un saludo o un "gracias"? Ahí no hace falta ninguna herramienta.
fn es_charla(texto: &str) -> bool {
    const CHARLA: [&str; 16] = [
        "hola", "buenas", "buen", "buenos", "dia", "tardes", "noches", "gracias", "chau", "como", "andas", "estas",
        "va", "todo", "bien", "nova",
    ];
    let palabras = palabras(texto);
    !palabras.is_empty() && palabras.iter().all(|p| CHARLA.contains(&p.as_str()) || p.starts_with("jaj"))
}

/// Las herramientas para este pedido. `anterior` es el mensaje anterior del
/// usuario, si es de esta charla: un mensaje sin tema propio ("sí, el de
/// Física", "dale") sigue el tema de antes. `None` = todas.
pub fn para_el_pedido(texto: &str, anterior: Option<&str>) -> Option<Vec<&'static str>> {
    if es_charla(texto) {
        return Some(vec![]);
    }
    let mut elegidos = temas(texto);
    if elegidos.is_empty() {
        elegidos = anterior.map(temas).unwrap_or_default();
    }
    if elegidos.is_empty() {
        return None;
    }
    // Las responsabilidades automáticas salen de la agenda: si se habla de una
    // se puede necesitar la otra ("borrá el parcial y lo de estudiar").
    if elegidos.contains(&Tema::Agenda) && !elegidos.contains(&Tema::Responsabilidades) {
        elegidos.push(Tema::Responsabilidades);
    }
    let mut nombres: Vec<&'static str> = Tema::TODOS
        .into_iter()
        .filter(|t| elegidos.contains(t))
        .flat_map(|t| t.herramientas().iter().copied())
        .collect();
    nombres.dedup();
    Some(nombres)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elegidas(texto: &str) -> Vec<&'static str> {
        para_el_pedido(texto, None).expect("debería reconocer el tema")
    }

    #[test]
    fn un_saludo_no_lleva_herramientas() {
        assert_eq!(para_el_pedido("Hola, ¿cómo andás?", None), Some(vec![]));
        assert_eq!(para_el_pedido("gracias nova jajaja", None), Some(vec![]));
        assert_eq!(para_el_pedido("buen día", Some("tengo parcial el jueves")), Some(vec![]));
    }

    #[test]
    fn futbol_solo_lleva_lo_de_futbol() {
        assert_eq!(elegidas("¿Cuándo juega Boca?"), ["consultar_partidos", "consultar_tabla", "consultar_jugador"]);
        assert!(elegidas("¿cómo le fue a Messi?").contains(&"consultar_jugador"));
    }

    #[test]
    fn la_agenda_trae_tambien_las_responsabilidades() {
        let h = elegidas("tengo parcial de Análisis el jueves a las 9");
        assert!(h.contains(&"crear_evento") && h.contains(&"crear_responsabilidad"));
        assert!(!h.contains(&"consultar_partidos") && !h.contains(&"consultar_noticias"));
    }

    #[test]
    fn reconoce_cada_tema() {
        assert!(elegidas("recordame comprar el regalo de Juan").contains(&"crear_responsabilidad"));
        assert!(elegidas("ya hice lo de la luz").contains(&"completar_responsabilidad"));
        assert!(elegidas("agregá la materia Física").contains(&"crear_materia"));
        assert_eq!(elegidas("¿qué noticias hay?"), ["consultar_noticias"]);
        assert!(elegidas("¿qué tengo mañana?").contains(&"resumen_del_dia"));
        assert!(elegidas("borrá lo del dentista").contains(&"borrar_evento"));
        assert!(elegidas("pasalo a las 10 de la tarde").contains(&"editar_evento"));
    }

    #[test]
    fn una_respuesta_corta_sigue_el_tema_de_antes() {
        let h = para_el_pedido("sí, el de Física", Some("tengo parcial el jueves")).unwrap();
        assert!(h.contains(&"crear_evento"));
        let h = para_el_pedido("dale", Some("¿cuándo juega Boca?")).unwrap();
        assert_eq!(h, ["consultar_partidos", "consultar_tabla", "consultar_jugador"]);
        // Si el mensaje nuevo tiene su tema, no arrastra el de antes.
        let h = para_el_pedido("¿cuándo juega Boca?", Some("¿qué tengo mañana?")).unwrap();
        assert_eq!(h, ["consultar_partidos", "consultar_tabla", "consultar_jugador"]);
    }

    #[test]
    fn si_no_reconoce_el_tema_van_todas() {
        assert_eq!(para_el_pedido("ponele color rojo", None), None);
        assert_eq!(para_el_pedido("sí", None), None);
    }
}
