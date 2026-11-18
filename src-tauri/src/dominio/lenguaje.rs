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

#[cfg(test)]
mod tests {
    use super::*;

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
