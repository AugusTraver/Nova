//! Ayudas para leer texto libre en español (lo que escribe el usuario o lo
//! que copia el modelo de lo que dijo el usuario).

/// Minúsculas, sin tildes ni diéresis, ñ → n, espacios de más afuera.
/// "  Análisis II " → "analisis ii". Sirve para comparar sin que importen
/// las tildes ("mañana" y "manana" son lo mismo).
pub fn normalizar(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    for c in texto.trim().chars().flat_map(char::to_lowercase) {
        salida.push(match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            otro => otro,
        });
    }
    // Varios espacios seguidos cuentan como uno.
    salida.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ¿El mensaje es solo un "sí" o un "no" a una confirmación pendiente?
/// `Some(true)` = confirma, `Some(false)` = cancela, `None` = es otra cosa
/// (y la confirmación pendiente se descarta).
///
/// Tiene que ser el mensaje entero: "sí, pero mejor el viernes" no confirma.
pub fn interpretar_confirmacion(texto: &str) -> Option<bool> {
    let limpio: String = normalizar(texto)
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect();
    let limpio = limpio.split_whitespace().collect::<Vec<_>>().join(" ");

    const SI: &[&str] = &[
        "si", "sii", "dale", "ok", "oka", "okey", "bueno", "de una", "obvio", "confirmo", "si dale",
        "si borralo", "si borrala", "borralo", "borrala", "si porfa", "si por favor", "si confirmo", "listo",
        // Hablando sale de muchas formas: "eliminalo", "sacalo", "hacelo".
        "eliminalo", "eliminala", "si eliminalo", "si eliminala", "sacalo", "sacala", "si sacalo", "si sacala",
        "quitalo", "quitala", "borrar", "eliminar", "si borrar", "si eliminar", "borra", "elimina", "hacelo",
        "si hacelo", "claro", "si claro", "confirmado", "dale borralo", "dale eliminalo",
    ];
    const NO: &[&str] = &[
        "no", "nop", "cancela", "cancelar", "cancelalo", "mejor no", "no lo borres", "no la borres", "deja",
        "dejalo", "dejala", "no gracias", "nada",
    ];
    if SI.contains(&limpio.as_str()) {
        Some(true)
    } else if NO.contains(&limpio.as_str()) {
        Some(false)
    } else {
        None
    }
}

/// Con un "¿Borro «X»…?" esperando: ¿el mensaje vuelve a pedir borrar eso
/// mismo? ("Eliminá lo de pasear al perro" con «Pasear al perro" pendiente.)
/// Hace falta un verbo de borrar y alguna palabra del título que no sea de
/// relleno. Si es así, cuenta como un "sí": el usuario ya lo pidió dos veces.
pub fn vuelve_a_pedir_borrar(texto: &str, titulo: &str) -> bool {
    const VERBOS: &[&str] = &["borr", "elimin", "sac", "quit", "cancel"];
    const RELLENO: &[&str] = &["para", "con", "del", "los", "las", "una", "uno", "que", "por", "evento", "tarea"];
    let texto = normalizar(texto);
    let palabras: Vec<&str> = texto.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()).collect();
    let pide_borrar = palabras.iter().any(|p| VERBOS.iter().any(|v| p.starts_with(v)));
    if !pide_borrar {
        return false;
    }
    let titulo = normalizar(titulo);
    titulo
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| p.chars().count() >= 3 && !RELLENO.contains(p))
        .any(|p| palabras.contains(&p))
}

/// A veces el modelo devuelve la misma respuesta dos veces pegada
/// ("Listo, X.Listo, X."). Si el texto es una frase repetida, queda una.
pub fn sin_repetir(texto: &str) -> String {
    let texto = texto.trim();
    let caracteres: Vec<char> = texto.chars().collect();
    let n = caracteres.len();
    // Se prueba cortar en la mitad, salteando los espacios del medio.
    for corte in (n / 2).saturating_sub(2)..=(n / 2 + 2).min(n) {
        let primera: String = caracteres[..corte].iter().collect();
        let segunda: String = caracteres[corte..].iter().collect();
        if primera.trim().chars().count() >= 8 && primera.trim() == segunda.trim() {
            return primera.trim().to_string();
        }
    }
    texto.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saca_la_respuesta_repetida() {
        let doble = "Listo, pasear al perro a las seis de la mañana.Listo, pasear al perro a las seis de la mañana.";
        assert_eq!(sin_repetir(doble), "Listo, pasear al perro a las seis de la mañana.");
        assert_eq!(sin_repetir("Dale, anotado. Dale, anotado."), "Dale, anotado.");
        // Lo normal queda igual.
        assert_eq!(sin_repetir("Sí, sí."), "Sí, sí.");
        assert_eq!(sin_repetir("Tenés parcial el jueves."), "Tenés parcial el jueves.");
    }

    #[test]
    fn repetir_el_pedido_de_borrar_lo_mismo_confirma() {
        assert!(vuelve_a_pedir_borrar("Eliminar la tarea, pasear al perro.", "Pasear al perro"));
        assert!(vuelve_a_pedir_borrar("Sacá el paseo al perro", "Pasear al perro"));
        assert!(vuelve_a_pedir_borrar("borrá el parcial de Análisis", "Parcial de Análisis II"));
        // Otra cosa, u otro evento: no.
        assert!(!vuelve_a_pedir_borrar("¿A qué hora es lo del perro?", "Pasear al perro"));
        assert!(!vuelve_a_pedir_borrar("Eliminá el parcial", "Pasear al perro"));
        assert!(!vuelve_a_pedir_borrar("borrá la tarea", "Tarea de inglés"));
    }

    #[test]
    fn entiende_como_se_dice_hablando() {
        assert_eq!(interpretar_confirmacion("Eliminalo."), Some(true));
        assert_eq!(interpretar_confirmacion("Sí, sacalo"), Some(true));
        assert_eq!(interpretar_confirmacion("Dale, borralo."), Some(true));
    }

    #[test]
    fn normaliza_tildes_mayusculas_y_espacios() {
        assert_eq!(normalizar("  Análisis   II "), "analisis ii");
        assert_eq!(normalizar("MAÑANA"), "manana");
        assert_eq!(normalizar("Pingüino"), "pinguino");
    }

    #[test]
    fn entiende_si_y_no() {
        assert_eq!(interpretar_confirmacion("Sí"), Some(true));
        assert_eq!(interpretar_confirmacion("sí, borralo!"), Some(true));
        assert_eq!(interpretar_confirmacion("dale"), Some(true));
        assert_eq!(interpretar_confirmacion("No."), Some(false));
        assert_eq!(interpretar_confirmacion("mejor no"), Some(false));
    }

    #[test]
    fn otra_cosa_no_es_confirmacion() {
        assert_eq!(interpretar_confirmacion("sí, pero el viernes"), None);
        assert_eq!(interpretar_confirmacion("¿qué tengo mañana?"), None);
        assert_eq!(interpretar_confirmacion(""), None);
    }
}
