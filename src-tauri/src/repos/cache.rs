//! Caché de datos de internet (tabla `cache_externa`, migración 3).
//!
//! Cada clave guarda el último dato bueno, cuándo vence y el último error.
//! Un pedido que falla NO borra el dato anterior: el widget lo sigue
//! mostrando con un aviso ("Sin conexión · datos de hace 3 h").

use chrono::{DateTime, FixedOffset, SecondsFormat};
use rusqlite::{Connection, OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::dominio::refresco;
use crate::error::Resultado;
use crate::repos::{Ahora, texto_fecha};

/// Lo que se le manda a la interfaz: el dato (si hay) y su estado.
/// `T` es el tipo del dato (partidos, noticias...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cacheado<T> {
    pub datos: Option<T>,
    /// Cuándo se obtuvo el dato que se muestra (ISO 8601).
    pub obtenido_en: Option<String>,
    /// El error del último intento, si falló. Con `datos` presentes, el
    /// widget muestra lo viejo con un aviso.
    pub error: Option<String>,
}

impl<T> Cacheado<T> {
    pub fn vacio() -> Self {
        Cacheado { datos: None, obtenido_en: None, error: None }
    }

    /// Transforma el dato sin perder el estado (`map` de un Option, pero
    /// conservando cuándo se obtuvo y el error).
    pub fn mapear<U>(self, f: impl FnOnce(T) -> U) -> Cacheado<U> {
        Cacheado { datos: self.datos.map(f), obtenido_en: self.obtenido_en, error: self.error }
    }
}

fn marca(momento: DateTime<FixedOffset>) -> String {
    momento.to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// Lee lo guardado para `clave`.
pub fn leer<T: DeserializeOwned>(conexion: &Connection, clave: &str) -> Resultado<Cacheado<T>> {
    let fila: Option<(Option<String>, Option<String>, Option<String>)> = conexion
        .query_row(
            "SELECT valor_json, obtenido_en, error FROM cache_externa WHERE clave = ?1",
            [clave],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
        )
        .optional()?;
    let Some((json, obtenido_en, error)) = fila else {
        return Ok(Cacheado::vacio());
    };
    // Si el formato guardado cambió entre versiones y ya no se lee, se trata
    // como "sin dato" (se vuelve a pedir), en vez de romper el widget.
    let datos = json.and_then(|j| serde_json::from_str(&j).ok());
    Ok(Cacheado { datos, obtenido_en, error })
}

/// ¿Hay que volver a pedir `clave`? También si nunca se pidió.
pub fn vencida(conexion: &Connection, clave: &str, ahora: &Ahora) -> Resultado<bool> {
    let vence: Option<String> = conexion
        .query_row("SELECT vence_en FROM cache_externa WHERE clave = ?1", [clave], |f| f.get(0))
        .optional()?;
    Ok(match vence.and_then(|v| DateTime::parse_from_rfc3339(&v).ok()) {
        Some(vence) => refresco::vencido(vence, ahora.momento),
        None => true,
    })
}

/// Guarda un dato bueno y cuándo vence.
pub fn guardar<T: Serialize>(
    conexion: &Connection,
    clave: &str,
    valor: &T,
    vence_en: DateTime<FixedOffset>,
    ahora: &Ahora,
) -> Resultado<()> {
    conexion.execute(
        "INSERT INTO cache_externa (clave, valor_json, obtenido_en, vence_en, error, fallas_seguidas)
         VALUES (?1, ?2, ?3, ?4, NULL, 0)
         ON CONFLICT (clave) DO UPDATE SET valor_json = excluded.valor_json,
             obtenido_en = excluded.obtenido_en, vence_en = excluded.vence_en,
             error = NULL, fallas_seguidas = 0",
        params![clave, serde_json::to_string(valor)?, ahora.marca, marca(vence_en)],
    )?;
    Ok(())
}

/// Anota que el pedido falló. El dato anterior queda; el próximo intento se
/// espacia según cuántas veces seguidas falló.
pub fn anotar_error(conexion: &Connection, clave: &str, error: &str, ahora: &Ahora) -> Resultado<()> {
    let fallas: u32 = conexion
        .query_row("SELECT fallas_seguidas FROM cache_externa WHERE clave = ?1", [clave], |f| f.get(0))
        .optional()?
        .unwrap_or(0);
    let fallas = fallas + 1;
    let vence = ahora.momento + refresco::espera_reintento(fallas);
    conexion.execute(
        "INSERT INTO cache_externa (clave, valor_json, obtenido_en, vence_en, error, fallas_seguidas)
         VALUES (?1, NULL, NULL, ?2, ?3, ?4)
         ON CONFLICT (clave) DO UPDATE SET vence_en = excluded.vence_en,
             error = excluded.error, fallas_seguidas = excluded.fallas_seguidas",
        params![clave, marca(vence), error, fallas],
    )?;
    Ok(())
}

/// Hace que `clave` venza ya (por ejemplo, la tabla cuando termina un partido).
pub fn vencer(conexion: &Connection, clave: &str, ahora: &Ahora) -> Resultado<()> {
    conexion.execute("UPDATE cache_externa SET vence_en = ?2 WHERE clave = ?1", params![clave, ahora.marca])?;
    Ok(())
}

/// Cambia solo cuándo vence (por ejemplo, reintentar pronto un partido en
/// vivo aunque haya fallado).
pub fn cambiar_vencimiento(conexion: &Connection, clave: &str, vence_en: DateTime<FixedOffset>) -> Resultado<()> {
    conexion.execute("UPDATE cache_externa SET vence_en = ?2 WHERE clave = ?1", params![clave, marca(vence_en)])?;
    Ok(())
}

/// Hace que venzan ya todas las claves que empiezan con `prefijo`
/// (por ejemplo, al cambiar los feeds o los jugadores en Ajustes).
pub fn vencer_prefijo(conexion: &Connection, prefijo: &str, ahora: &Ahora) -> Resultado<()> {
    conexion.execute(
        "UPDATE cache_externa SET vence_en = ?2 WHERE substr(clave, 1, length(?1)) = ?1",
        params![prefijo, ahora.marca],
    )?;
    Ok(())
}

/// Suma `cantidad` pedidos al contador del día y dice si todavía entran en
/// el tope. Si no entran, no suma nada y devuelve `false`.
pub fn contar_pedidos(conexion: &Connection, servicio: &str, cantidad: u32, tope: u32, ahora: &Ahora) -> Resultado<bool> {
    let fecha = texto_fecha(ahora.hoy);
    let hechos: u32 = conexion
        .query_row(
            "SELECT cantidad FROM pedidos_por_dia WHERE servicio = ?1 AND fecha = ?2",
            params![servicio, fecha],
            |f| f.get(0),
        )
        .optional()?
        .unwrap_or(0);
    if hechos + cantidad > tope {
        return Ok(false);
    }
    conexion.execute(
        "INSERT INTO pedidos_por_dia (servicio, fecha, cantidad) VALUES (?1, ?2, ?3)
         ON CONFLICT (servicio, fecha) DO UPDATE SET cantidad = cantidad + excluded.cantidad",
        params![servicio, fecha, cantidad],
    )?;
    // Los días viejos no sirven para nada.
    conexion.execute("DELETE FROM pedidos_por_dia WHERE fecha < date(?1, '-7 days')", [fecha])?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::db;

    #[test]
    fn guarda_lee_y_vence() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo_a_las("2026-11-17", "14:00");
        assert_eq!(leer::<Vec<u32>>(&base, "x").unwrap(), Cacheado::vacio());
        assert!(vencida(&base, "x", &ahora).unwrap());

        guardar(&base, "x", &vec![1, 2], ahora.momento + Duration::hours(1), &ahora).unwrap();
        let leido = leer::<Vec<u32>>(&base, "x").unwrap();
        assert_eq!(leido.datos, Some(vec![1, 2]));
        assert_eq!(leido.obtenido_en.as_deref(), Some("2026-11-17T14:00:00-03:00"));
        assert!(!vencida(&base, "x", &ahora).unwrap());
        assert!(vencida(&base, "x", &Ahora::fijo_a_las("2026-11-17", "15:00")).unwrap());

        vencer(&base, "x", &ahora).unwrap();
        assert!(vencida(&base, "x", &ahora).unwrap());
    }

    #[test]
    fn un_error_conserva_el_dato_y_espacia_los_reintentos() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo_a_las("2026-11-17", "14:00");
        guardar(&base, "x", &"viejo", ahora.momento, &ahora).unwrap();

        anotar_error(&base, "x", "Sin conexión", &ahora).unwrap();
        let leido = leer::<String>(&base, "x").unwrap();
        assert_eq!(leido.datos.as_deref(), Some("viejo"));
        assert_eq!(leido.error.as_deref(), Some("Sin conexión"));
        assert!(!vencida(&base, "x", &Ahora::fijo_a_las("2026-11-17", "14:04")).unwrap());
        assert!(vencida(&base, "x", &Ahora::fijo_a_las("2026-11-17", "14:05")).unwrap());

        // La segunda falla espera 15 minutos.
        anotar_error(&base, "x", "Sin conexión", &ahora).unwrap();
        assert!(!vencida(&base, "x", &Ahora::fijo_a_las("2026-11-17", "14:14")).unwrap());

        // Un dato bueno borra el error.
        guardar(&base, "x", &"nuevo", ahora.momento, &ahora).unwrap();
        assert_eq!(leer::<String>(&base, "x").unwrap().error, None);

        // Un error sin dato previo deja la fila sin dato.
        anotar_error(&base, "y", "404", &ahora).unwrap();
        let y = leer::<String>(&base, "y").unwrap();
        assert_eq!((y.datos, y.error.as_deref()), (None, Some("404")));
    }

    #[test]
    fn un_dato_con_otro_formato_cuenta_como_vacio() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        guardar(&base, "x", &"texto", ahora.momento, &ahora).unwrap();
        assert_eq!(leer::<Vec<u32>>(&base, "x").unwrap().datos, None);
    }

    #[test]
    fn vence_por_prefijo() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo_a_las("2026-11-17", "14:00");
        let luego = ahora.momento + Duration::hours(5);
        guardar(&base, "noticias:a", &1, luego, &ahora).unwrap();
        guardar(&base, "noticias:b", &1, luego, &ahora).unwrap();
        guardar(&base, "futbol:a", &1, luego, &ahora).unwrap();
        vencer_prefijo(&base, "noticias:", &ahora).unwrap();
        assert!(vencida(&base, "noticias:a", &ahora).unwrap());
        assert!(vencida(&base, "noticias:b", &ahora).unwrap());
        assert!(!vencida(&base, "futbol:a", &ahora).unwrap());
    }

    #[test]
    fn el_tope_diario_de_pedidos() {
        let base = db::en_memoria();
        let hoy = Ahora::fijo("2026-11-17");
        assert!(contar_pedidos(&base, "espn", 1, 3, &hoy).unwrap());
        assert!(contar_pedidos(&base, "espn", 2, 3, &hoy).unwrap());
        assert!(!contar_pedidos(&base, "espn", 1, 3, &hoy).unwrap());
        // Otro servicio y otro día cuentan aparte.
        assert!(contar_pedidos(&base, "feeds", 2, 3, &hoy).unwrap());
        assert!(contar_pedidos(&base, "espn", 3, 3, &Ahora::fijo("2026-11-18")).unwrap());
        assert!(!contar_pedidos(&base, "espn", 4, 3, &Ahora::fijo("2026-11-19")).unwrap());
    }
}
