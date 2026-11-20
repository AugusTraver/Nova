//! Qué herramientas se le mandan al modelo en cada pedido.
//!
//! Las definiciones de las 14 herramientas pesan unos 800 tokens y se mandan
//! en cada vuelta. El plan gratis de Groq cuenta tokens por minuto, así que
//! se mandan solo las del tema del pedido ("¿cuándo juega Boca?" no necesita
//! las de la agenda). Es por palabras clave: si no se reconoce el tema, van
//! todas las de siempre (mejor gastar de más que dejar al modelo sin poder
//! hacer algo), pero nunca las de la compu: esas solo van si se pidieron.
//!
//! Desde v0.7.1 hay palabras **débiles**: "martes", "cambiá", "borrá" o "hoy"
//! suman la agenda solo si el pedido no es de la compu. Así "en notas.txt
//! cambiá martes por jueves" no manda también las herramientas de la agenda.

use std::path::Path;

use crate::compu::herramientas::{ABRIR, ARCHIVOS, MUSICA};
use crate::dominio::compu::se_puede_abrir_con_programa;
use crate::dominio::lenguaje::normalizar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tema {
    Agenda,
    Responsabilidades,
    Materias,
    Futbol,
    Noticias,
    Resumen,
    /// v0.7.1: abrir páginas, búsquedas, videos y programas.
    Compu,
    Archivos,
    Musica,
}

impl Tema {
    /// Los temas de siempre: son los que van cuando no se reconoce el tema
    /// (`definiciones_de(None)`); acá solo lo usan los tests.
    #[cfg(test)]
    const DE_SIEMPRE: [Tema; 6] =
        [Tema::Agenda, Tema::Responsabilidades, Tema::Materias, Tema::Futbol, Tema::Noticias, Tema::Resumen];
    const TODOS: [Tema; 9] = [
        Tema::Agenda,
        Tema::Responsabilidades,
        Tema::Materias,
        Tema::Futbol,
        Tema::Noticias,
        Tema::Resumen,
        Tema::Compu,
        Tema::Archivos,
        Tema::Musica,
    ];

    fn es_de_la_compu(self) -> bool {
        matches!(self, Tema::Compu | Tema::Archivos | Tema::Musica)
    }

    fn herramientas(self) -> &'static [&'static str] {
        match self {
            Tema::Agenda => &["crear_evento", "editar_evento", "borrar_evento", "listar_eventos"],
            Tema::Responsabilidades => &["crear_responsabilidad", "completar_responsabilidad", "listar_responsabilidades"],
            Tema::Materias => &["crear_materia", "listar_materias"],
            Tema::Futbol => &["consultar_partidos", "consultar_tabla", "consultar_jugador"],
            Tema::Noticias => &["consultar_noticias"],
            Tema::Resumen => &["resumen_del_dia"],
            Tema::Compu => &ABRIR,
            Tema::Archivos => &ARCHIVOS,
            Tema::Musica => &MUSICA,
        }
    }

    /// Palabras (sin tildes) que lo delatan. Con `*` al final, cualquier
    /// palabra que empiece así ("agend*" = agenda, agendame, agendá…). Con un
    /// espacio, una frase.
    fn palabras(self) -> &'static [&'static str] {
        match self {
            Tema::Agenda => &[
                "agend*", "evento*", "parcial*", "examen*", "final*", "clase*", "entrega*", "turno*", "reunion*",
                "cumple*", "cita*", "dentista", "medico*", "calendario", "de la tarde", "de la manana", "de la noche",
            ],
            Tema::Responsabilidades => {
                &["record*", "acorda*", "tarea*", "pendiente*", "responsabilidad*", "vence*", "vencid*"]
            }
            Tema::Materias => &["materia*", "cursada*", "cuatri*"],
            Tema::Futbol => &[
                "boca", "seleccion", "partido*", "jueg*", "jug*", "gan*", "perdi*", "empat*", "gol*", "tabla",
                "torneo", "posicion*", "futbol", "messi", "scaloni", "resultado*", "fecha",
            ],
            Tema::Noticias => &["noticia*", "titular*", "diario*", "que paso", "novedad*", "pasando"],
            Tema::Resumen => &["que tengo", "mi dia", "resumen*", "resumi*", "como viene"],
            Tema::Compu => &[
                // "abr*" agarraría "abril".
                "abri", "abrime", "abrir", "abrilo", "abrila", "abre", "abrimelo", "poneme", "pone", "pon", "reproduc*", "google*", "busca*", "youtube", "video*", "pagina*",
                "web", "sitio*", "navegador", "pestana*", "programa*", "app", "aplicacion*", "link", "www", "http*",
                "spotify", "chrome", "calculadora", "vscode", "visual", "code",
            ],
            Tema::Archivos => &[
                "archivo*", "carpeta*", "documento*", "descargas", "escritorio", "deshac*", "deshace", "txt",
            ],
            Tema::Musica => &[
                "pausa*", "musica", "tema", "cancion*", "volumen", "silenci*", "mute", "play", "despausa*",
            ],
        }
    }

    /// Las que suman el tema solo si el pedido no es de la compu.
    fn palabras_debiles(self) -> &'static [&'static str] {
        match self {
            Tema::Agenda => &[
                "cancel*", "borr*", "elimin*", "mov*", "cambi*", "pasa*", "corre*", "a las", "lunes", "martes",
                "miercoles", "jueves", "viernes", "sabado*", "domingo*", "manana", "pasado", "semana*", "hoy",
                "hora*", "dia", "dias", "fecha*",
            ],
            Tema::Responsabilidades => &[
                "anot*", "hice", "hiciste", "termin*", "hecha*", "hecho", "complet*", "compr*", "pag*", "estudi*",
                "tengo que", "hacer",
            ],
            Tema::Resumen => &["hoy"],
            // "La página de la facultad" no es de materias.
            Tema::Materias => &["facu*"],
            Tema::Musica => &["siguiente", "anterior", "subi*", "baja*"],
            _ => &[],
        }
    }

    fn coincide(claves: &[&str], palabras: &[String], frase: &str) -> bool {
        claves.iter().any(|clave| {
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

/// ¿Nombra un archivo con extensión ("notas.txt", "tp.docx")?
fn menciona_archivo(texto: &str) -> bool {
    texto.split_whitespace().any(|t| {
        let t = t.trim_matches(|c: char| !c.is_alphanumeric());
        t.contains('.') && !t.starts_with("www.") && se_puede_abrir_con_programa(Path::new(t))
    })
}

fn temas(texto: &str) -> Vec<Tema> {
    let palabras = palabras(texto);
    let frase = format!(" {} ", palabras.join(" "));
    let fuertes: Vec<Tema> = Tema::TODOS
        .into_iter()
        .filter(|t| Tema::coincide(t.palabras(), &palabras, &frase) || (*t == Tema::Archivos && menciona_archivo(texto)))
        .collect();
    let de_la_compu = fuertes.iter().any(|t| t.es_de_la_compu());
    let debiles = Tema::TODOS.into_iter().filter(|t| Tema::coincide(t.palabras_debiles(), &palabras, &frase));
    // Una débil de la compu ("subí", "siguiente") solo cuenta si ya hay otra
    // de la compu; una débil de los otros temas, solo si no la hay.
    let debiles: Vec<Tema> = debiles.filter(|t| t.es_de_la_compu() == de_la_compu).collect();
    Tema::TODOS.into_iter().filter(|t| fuertes.contains(t) || debiles.contains(t)).collect()
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
/// Física", "dale") sigue el tema de antes. `None` = todas las de siempre
/// (sin las de la compu).
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
    let mut nombres: Vec<&'static str> = Vec::new();
    for tema in Tema::TODOS.into_iter().filter(|t| elegidos.contains(t)) {
        for nombre in tema.herramientas() {
            // `abrir_archivo` está en dos temas: va una vez.
            if !nombres.contains(nombre) {
                nombres.push(nombre);
            }
        }
    }
    Some(nombres)
}

/// Las de siempre, para cuando no se reconoce el tema (ver `definiciones_de`).
#[cfg(test)]
fn de_siempre() -> Vec<&'static str> {
    Tema::DE_SIEMPRE.into_iter().flat_map(|t| t.herramientas().iter().copied()).collect()
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
        assert!(!h.iter().any(|n| crate::compu::herramientas::es_de_la_compu(n)));
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
        assert!(elegidas("movelo al viernes").contains(&"editar_evento"));
        assert!(elegidas("¿qué tengo hoy?").contains(&"resumen_del_dia"));
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
        let h = para_el_pedido("el primero", Some("abrí Visual Studio")).unwrap();
        assert!(h.contains(&"abrir_programa"));
    }

    #[test]
    fn si_no_reconoce_el_tema_van_todas_las_de_siempre() {
        assert_eq!(para_el_pedido("ponele color rojo", None), None);
        assert_eq!(para_el_pedido("sí", None), None);
        assert!(!de_siempre().iter().any(|n| crate::compu::herramientas::es_de_la_compu(n)));
    }

    // ---------- v0.7.1 ----------

    fn sin_agenda(h: &[&str]) -> bool {
        !h.contains(&"crear_evento") && !h.contains(&"crear_responsabilidad") && !h.contains(&"resumen_del_dia")
    }

    #[test]
    fn los_pedidos_de_la_compu_llevan_solo_lo_de_la_compu() {
        let h = elegidas("Oye Nova, poneme el primer video que sale cuando buscás Riquelme");
        assert!(h.contains(&"poner_video") && sin_agenda(&h), "{h:?}");
        let h = elegidas("Buscame en Google recetas de pizza");
        assert_eq!(h, ABRIR);
        let h = elegidas("Abrí Visual Studio Code");
        assert_eq!(h, ABRIR);
        let h = elegidas("abrime un video de lo-fi");
        assert!(h.contains(&"poner_video"));
        let h = elegidas("abrí la página de la facultad");
        assert_eq!(h, ABRIR);
        assert!(!elegidas("el 3 de abril tengo parcial").contains(&"abrir_pagina"));
        assert!(elegidas("tengo que entregar el tp de la facu").contains(&"crear_materia"));
    }

    #[test]
    fn un_cambio_de_archivo_no_trae_la_agenda() {
        let h = elegidas("En notas.txt cambiá martes por jueves");
        assert!(h.contains(&"cambiar_archivo") && sin_agenda(&h), "{h:?}");
        let h = elegidas("¿qué dice el archivo notas.txt?");
        assert!(h.contains(&"leer_archivo") && sin_agenda(&h));
        let h = elegidas("abrí la carpeta de descargas");
        assert!(h.contains(&"abrir_archivo") && sin_agenda(&h));
        assert_eq!(h.iter().filter(|n| **n == "abrir_archivo").count(), 1);
        let h = elegidas("deshacé eso");
        assert!(h.contains(&"deshacer_cambio"));
        let h = elegidas("creá un archivo compras con leche y huevos");
        assert!(h.contains(&"crear_archivo") && sin_agenda(&h), "{h:?}");
        let h = elegidas("borrá tp-viejo.docx");
        assert!(h.contains(&"borrar_archivo") && !h.contains(&"borrar_evento"), "{h:?}");
    }

    #[test]
    fn la_musica_lleva_una_sola_herramienta() {
        assert_eq!(elegidas("pausá la música"), MUSICA);
        assert_eq!(elegidas("subí el volumen"), MUSICA);
        assert_eq!(elegidas("pasá al siguiente tema"), MUSICA);
        // "Siguiente" o "subí" solos no son música.
        assert!(!elegidas("la semana siguiente tengo parcial").contains(&"controlar_musica"));
    }
}
