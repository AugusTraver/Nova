//! Arreglos de texto para la voz (puros, con tests).
//!
//! - Lo que devuelve Whisper: con silencio o ruido a veces "alucina" frases
//!   que vio mucho al entrenarse ("Subtítulos realizados por la comunidad
//!   de Amara.org"). Esas se descartan.
//! - Lo que va a leer Piper: sin markdown ni emojis, y partido en oraciones
//!   para empezar a hablar antes de tener todo sintetizado.

use crate::dominio::lenguaje::normalizar;

/// Frases que Whisper inventa sobre silencio o ruido (normalizadas).
const ALUCINACIONES: &[&str] = &[
    "subtitulos realizados por la comunidad de amara.org",
    "subtitulos por la comunidad de amara.org",
    "gracias por ver",
    "gracias por ver el video",
    "gracias por mirar",
    "suscribete",
    "suscribete al canal",
    "musica",
    "aplausos",
    "risas",
    "gracias",
    "chau",
    "...",
    "y",
    "eh",
    "mm",
    "mmm",
];

/// El texto limpio, o `None` si no hay nada que valga la pena mandar.
pub fn limpiar_transcripcion(texto: &str) -> Option<String> {
    // Sin lo que va entre corchetes o paréntesis: "[Música]", "(risas)".
    let mut limpio = String::with_capacity(texto.len());
    let mut profundidad = 0;
    for c in texto.chars() {
        match c {
            '[' | '(' => profundidad += 1,
            ']' | ')' if profundidad > 0 => profundidad -= 1,
            _ if profundidad == 0 => limpio.push(c),
            _ => {}
        }
    }
    let limpio = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    let clave: String = normalizar(&limpio)
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.')
        .trim_end_matches('.')
        .to_string();
    if clave.is_empty() || ALUCINACIONES.contains(&clave.as_str()) {
        return None;
    }
    // Hace falta al menos una letra.
    if !limpio.chars().any(char::is_alphabetic) {
        return None;
    }
    Some(limpio)
}

/// Deja el texto listo para leer: sin markdown, sin emojis ni símbolos que
/// Piper leería raro.
pub fn para_leer(texto: &str) -> String {
    let sin_marcas: String = texto
        .chars()
        .filter(|c| !matches!(c, '*' | '_' | '`' | '#' | '>' | '|' | '~'))
        // Emojis y pictogramas: fuera (el rango de símbolos de Unicode).
        .filter(|c| !matches!(*c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F | 0x200D))
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    sin_marcas
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" - ", ", ")
        .replace(" – ", ", ")
}

/// Parte en oraciones (por `.`, `?`, `!` o `;` seguidos de espacio). Las muy
/// cortas se pegan a la siguiente, para no hacer pausas raras.
pub fn oraciones(texto: &str) -> Vec<String> {
    let mut partes: Vec<String> = Vec::new();
    let mut actual = String::new();
    let letras: Vec<char> = texto.chars().collect();
    for (i, c) in letras.iter().enumerate() {
        actual.push(*c);
        let fin = matches!(c, '.' | '?' | '!' | ';') && letras.get(i + 1).is_none_or(|s| s.is_whitespace());
        if fin && actual.trim().chars().count() >= 25 {
            partes.push(actual.trim().to_string());
            actual.clear();
        }
    }
    if !actual.trim().is_empty() {
        partes.push(actual.trim().to_string());
    }
    partes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descarta_alucinaciones_y_ruido() {
        assert_eq!(limpiar_transcripcion(" Subtítulos realizados por la comunidad de Amara.org "), None);
        assert_eq!(limpiar_transcripcion("¡Gracias por ver!"), None);
        assert_eq!(limpiar_transcripcion("[Música]"), None);
        assert_eq!(limpiar_transcripcion("..."), None);
        assert_eq!(limpiar_transcripcion("   "), None);
        assert_eq!(limpiar_transcripcion("123"), None);
    }

    #[test]
    fn deja_los_pedidos_de_verdad() {
        assert_eq!(
            limpiar_transcripcion(" Recordame comprar el regalo de Juan  para el viernes.\n"),
            Some("Recordame comprar el regalo de Juan para el viernes.".into())
        );
        assert_eq!(limpiar_transcripcion("(risas) ¿Qué tengo hoy?"), Some("¿Qué tengo hoy?".into()));
        // "gracias" solo es ruido, pero dentro de una frase vale.
        assert_eq!(limpiar_transcripcion("Gracias, ¿y mañana?"), Some("Gracias, ¿y mañana?".into()));
    }

    #[test]
    fn prepara_para_leer() {
        assert_eq!(para_leer("**Listo** 👍\nTe lo anoto - para el viernes."), "Listo Te lo anoto, para el viernes.");
    }

    #[test]
    fn parte_en_oraciones() {
        assert_eq!(
            oraciones("Listo, te lo anoto para el viernes. Sí. ¿Querés que te lo recuerde el jueves a la noche?"),
            vec!["Listo, te lo anoto para el viernes.", "Sí. ¿Querés que te lo recuerde el jueves a la noche?"]
        );
        assert_eq!(oraciones("Las 3.5 horas"), vec!["Las 3.5 horas"]);
        assert!(oraciones("").is_empty());
    }
}
