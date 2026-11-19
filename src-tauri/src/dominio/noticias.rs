//! Qué noticias mostrar (SPEC 4.5), sin red: elegir candidatos, armar el
//! pedido a la IA, leer su respuesta y, si no hay IA, ordenar por palabras
//! clave. Todo puro y con tests.

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use crate::asistente::mensajes::Mensaje;
use crate::dominio::lenguaje::normalizar;
use crate::noticias::{Noticia, Titular};
use crate::noticias::rss::recortar;

/// Cuántos titulares se le muestran a la IA (cuidando el límite de tokens).
pub const MAX_CANDIDATOS: usize = 30;
/// Cuántas noticias muestra el widget.
pub const MIN_ELEGIDAS: usize = 5;
pub const MAX_ELEGIDAS: usize = 8;
/// Más viejo que esto no es noticia.
pub const HORAS_VIGENCIA: i64 = 36;
/// Sin IA, como mucho tantas de la misma fuente.
pub const POR_FUENTE: usize = 3;
/// La IA se consulta como mucho cada tanto (minutos).
pub const MINUTOS_ENTRE_IA: i64 = 240;
/// Los feeds se vuelven a bajar cada tanto (minutos).
pub const MINUTOS_ENTRE_FEEDS: i64 = 30;

fn fecha(t: &Titular) -> Option<DateTime<FixedOffset>> {
    t.publicado.as_deref().and_then(|f| DateTime::parse_from_rfc3339(f).ok())
}

fn horas_desde(t: &Titular, ahora: DateTime<FixedOffset>) -> Option<i64> {
    fecha(t).map(|f| (ahora - f).num_hours())
}

/// Los titulares que vale la pena mirar: sin repetidos (mismo título en dos
/// feeds), recientes y mezclando fuentes (de a una por fuente, en ronda, así
/// un feed que publica mucho no tapa al resto).
pub fn candidatos(titulares: &[Titular], ahora: DateTime<FixedOffset>) -> Vec<Titular> {
    let mut vistos: Vec<String> = Vec::new();
    let mut frescos: Vec<&Titular> = titulares
        .iter()
        .filter(|t| horas_desde(t, ahora).is_none_or(|h| h <= HORAS_VIGENCIA))
        .filter(|t| {
            let clave = normalizar(&t.titulo);
            if vistos.contains(&clave) {
                false
            } else {
                vistos.push(clave);
                true
            }
        })
        .collect();
    // Los más nuevos primero; los sin fecha, al final.
    frescos.sort_by_key(|t| std::cmp::Reverse(fecha(t)));

    let mut fuentes: Vec<&str> = Vec::new();
    for t in &frescos {
        if !fuentes.contains(&t.fuente.as_str()) {
            fuentes.push(&t.fuente);
        }
    }
    let mut colas: Vec<std::collections::VecDeque<&Titular>> = fuentes
        .iter()
        .map(|f| frescos.iter().copied().filter(|t| t.fuente == *f).collect())
        .collect();
    let mut elegidos = Vec::new();
    while elegidos.len() < MAX_CANDIDATOS && colas.iter().any(|c| !c.is_empty()) {
        for cola in colas.iter_mut() {
            if let Some(t) = cola.pop_front() {
                elegidos.push(t.clone());
                if elegidos.len() == MAX_CANDIDATOS {
                    break;
                }
            }
        }
    }
    elegidos
}

/// ¿Cambiaron los candidatos desde la última vez que se eligió?
pub fn hay_novedades(anteriores: &[String], actuales: &[Titular]) -> bool {
    actuales.iter().any(|t| !anteriores.contains(&t.id))
}

/// "Boca, Selección Argentina y IA" → ["boca", "seleccion argentina", "ia"].
pub fn intereses(texto: &str) -> Vec<String> {
    normalizar(texto)
        .replace(" y ", ",")
        .split([',', ';', '\n'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Palabras que cuentan para cada interés. Los intereses conocidos suman
/// algunos sinónimos (sin IA, "IA" no encontraría "ChatGPT").
fn palabras_de(interes: &str) -> Vec<String> {
    let extra: &[&str] = match interes {
        "boca" | "boca juniors" => &["boca", "bombonera", "xeneize", "xeneizes"],
        "seleccion" | "seleccion argentina" => &["seleccion", "scaloni", "messi", "albiceleste"],
        "futbol argentino" | "futbol" => &["river", "racing", "independiente", "san lorenzo", "liga profesional", "afa", "copa argentina", "libertadores", "sudamericana"],
        "ia" | "inteligencia artificial" => &["ia", "inteligencia artificial", "ai", "openai", "chatgpt", "gemini", "claude", "anthropic", "llm", "llms", "modelo de lenguaje"],
        "desarrollo web" | "programacion" | "web" => &["javascript", "typescript", "css", "html", "react", "frontend", "web", "navegador", "chrome", "node"],
        _ => &[],
    };
    let mut palabras: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
    palabras.push(interes.to_string());
    palabras
}

/// ¿Aparece `buscada` en el título como palabra (o frase) entera?
/// "ia" no tiene que encontrar "Italia".
fn aparece(titulo_palabras: &[String], buscada: &str) -> bool {
    let partes: Vec<&str> = buscada.split_whitespace().collect();
    if partes.is_empty() {
        return false;
    }
    titulo_palabras.windows(partes.len()).any(|ventana| ventana.iter().zip(&partes).all(|(a, b)| a == b))
}

fn palabras_del_titulo(t: &Titular) -> Vec<String> {
    normalizar(&t.titulo)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

/// Sin IA: un punto por interés que aparece en el título, más un poco por
/// ser reciente (de 0 a 1).
pub fn puntaje(t: &Titular, intereses: &[String], ahora: DateTime<FixedOffset>) -> f64 {
    let palabras = palabras_del_titulo(t);
    let coincidencias = intereses
        .iter()
        .filter(|i| palabras_de(i).iter().any(|p| aparece(&palabras, p)))
        .count() as f64;
    let reciente = horas_desde(t, ahora).map(|h| 1.0 - (h.max(0) as f64 / HORAS_VIGENCIA as f64)).unwrap_or(0.0);
    coincidencias * 2.0 + reciente.clamp(0.0, 1.0)
}

fn a_noticia(t: &Titular, resumen: String) -> Noticia {
    Noticia {
        id: t.id.clone(),
        titulo: t.titulo.clone(),
        resumen,
        fuente: t.fuente.clone(),
        url: t.url.clone(),
        publicado: t.publicado.clone(),
    }
}

/// El orden de respaldo: las que coinciden con los intereses primero, como
/// mucho `POR_FUENTE` de cada fuente. Si no alcanzan, se completa con las
/// más nuevas. El resumen es la bajada del feed (si hay).
pub fn ordenar_sin_ia(candidatos: &[Titular], intereses: &[String], ahora: DateTime<FixedOffset>) -> Vec<Noticia> {
    let mut con_puntaje: Vec<(f64, &Titular)> = candidatos.iter().map(|t| (puntaje(t, intereses, ahora), t)).collect();
    // `total_cmp` ordena f64 (los NaN no pueden aparecer acá, pero así compila sin unwrap).
    con_puntaje.sort_by(|a, b| b.0.total_cmp(&a.0));
    let cantidad = MIN_ELEGIDAS + 1;
    let mut elegidas: Vec<Noticia> = Vec::new();
    for (_, t) in &con_puntaje {
        if elegidas.len() == cantidad {
            break;
        }
        if elegidas.iter().filter(|n| n.fuente == t.fuente).count() >= POR_FUENTE {
            continue;
        }
        let resumen = t.bajada.as_deref().map(|b| recortar(b, 110)).unwrap_or_default();
        elegidas.push(a_noticia(t, resumen));
    }
    elegidas
}

/// El pedido a la IA: los candidatos numerados y qué tiene que devolver.
pub fn armar_pedido(candidatos: &[Titular], intereses: &str) -> Vec<Mensaje> {
    let sistema = format!(
        "Elegís noticias para el panel personal de un usuario argentino. Sus intereses: {intereses}.\n\
         De la lista, elegí entre {MIN_ELEGIDAS} y {MAX_ELEGIDAS} titulares que le importen, de la más relevante a la menos. \
         Evitá repetir el mismo tema. Para cada uno escribí un resumen de una línea (máximo 90 letras), en español rioplatense, \
         usando solo lo que dicen el título y la bajada: no inventes datos.\n\
         Respondé SOLO con JSON, sin texto alrededor. En \"titulo\" copiá el título tal cual está en la lista:          {{\"noticias\":[{{\"n\":3,\"titulo\":\"...\",\"resumen\":\"...\"}}]}}"
    );
    let lista: Vec<String> = candidatos
        .iter()
        .enumerate()
        .map(|(i, t)| match &t.bajada {
            Some(b) => format!("{}. [{}] {} — {}", i + 1, t.fuente, t.titulo, recortar(b, 120)),
            None => format!("{}. [{}] {}", i + 1, t.fuente, t.titulo),
        })
        .collect();
    vec![Mensaje::Sistema(sistema), Mensaje::Usuario(lista.join("\n"))]
}

#[derive(Deserialize)]
struct RespuestaIa {
    noticias: Vec<ElegidaIa>,
}

#[derive(Deserialize)]
struct ElegidaIa {
    #[serde(default)]
    n: Option<usize>,
    #[serde(default)]
    titulo: Option<String>,
    #[serde(default)]
    resumen: String,
}

/// Qué candidato eligió la IA. Primero por el título que copió (los modelos
/// a veces cuentan mal y el número queda corrido); si no lo copió bien, por
/// el número.
fn candidato<'a>(e: &ElegidaIa, candidatos: &'a [Titular]) -> Option<&'a Titular> {
    if let Some(titulo) = e.titulo.as_deref().map(normalizar).filter(|t| t.chars().count() >= 8) {
        let inicio: String = titulo.chars().take(40).collect();
        if let Some(t) = candidatos.iter().find(|t| normalizar(&t.titulo).starts_with(&inicio)) {
            return Some(t);
        }
    }
    e.n.and_then(|n| n.checked_sub(1)).and_then(|i| candidatos.get(i))
}

/// Lee lo que contestó la IA. `None` si no sirve (y se usa el orden sin IA).
/// Tolera texto alrededor del JSON y bloques ```json.
pub fn leer_respuesta(texto: &str, candidatos: &[Titular]) -> Option<Vec<Noticia>> {
    let inicio = texto.find('{')?;
    let fin = texto.rfind('}')?;
    let respuesta: RespuestaIa = serde_json::from_str(texto.get(inicio..=fin)?).ok()?;
    let mut elegidas: Vec<Noticia> = Vec::new();
    for e in respuesta.noticias {
        // Uno inventado (ni el título ni el número existen) se ignora.
        let Some(t) = candidato(&e, candidatos) else { continue };
        if elegidas.iter().any(|n| n.id == t.id) {
            continue;
        }
        let resumen = recortar(e.resumen.trim(), 120);
        elegidas.push(a_noticia(t, resumen));
        if elegidas.len() == MAX_ELEGIDAS {
            break;
        }
    }
    // Con menos de 3 la respuesta no es confiable.
    (elegidas.len() >= 3).then_some(elegidas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noticias::id_de;

    fn ahora() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-11-18T12:00:00-03:00").unwrap()
    }

    fn titular(titulo: &str, fuente: &str, horas: i64) -> Titular {
        let url = format!("https://{fuente}.com/{titulo}");
        Titular {
            id: id_de(&url),
            titulo: titulo.into(),
            bajada: Some(format!("Bajada de {titulo}")),
            fuente: fuente.into(),
            url,
            publicado: Some((ahora() - chrono::Duration::hours(horas)).to_rfc3339()),
        }
    }

    #[test]
    fn candidatos_sin_repetidos_ni_viejos_y_mezclados() {
        let lista = vec![
            titular("Uno", "ole", 1),
            titular("Dos", "ole", 2),
            titular("Tres", "ole", 3),
            titular("UNO", "clarin", 1), // repetido de otra fuente
            titular("Cuatro", "clarin", 5),
            titular("Viejo", "clarin", 50),
        ];
        let c = candidatos(&lista, ahora());
        let titulos: Vec<&str> = c.iter().map(|t| t.titulo.as_str()).collect();
        assert_eq!(titulos, vec!["Uno", "Cuatro", "Dos", "Tres"]);
    }

    #[test]
    fn novedades() {
        let lista = vec![titular("Uno", "ole", 1)];
        assert!(hay_novedades(&[], &lista));
        assert!(!hay_novedades(&[lista[0].id.clone()], &lista));
    }

    #[test]
    fn lee_intereses() {
        assert_eq!(intereses("Boca, Selección Argentina y IA"), vec!["boca", "seleccion argentina", "ia"]);
        assert!(intereses("  ").is_empty());
    }

    #[test]
    fn puntaje_por_palabra_entera_y_sinonimos() {
        let i = intereses("IA, Boca");
        assert!(puntaje(&titular("OpenAI presentó un modelo nuevo", "x", 0), &i, ahora()) >= 2.0);
        // "Italia" no es "IA"
        assert!(puntaje(&titular("Italia goleó a Noruega", "x", 0), &i, ahora()) < 2.0);
        // Dos intereses en el mismo título suman más
        let dos = puntaje(&titular("Boca usa IA para analizar rivales", "x", 0), &i, ahora());
        assert!(dos >= 4.0);
    }

    #[test]
    fn sin_ia_primero_lo_que_interesa_y_pocas_por_fuente() {
        let i = intereses("Boca");
        let lista = vec![
            titular("Boca 1", "ole", 5),
            titular("Boca 2", "ole", 5),
            titular("Boca 3", "ole", 5),
            titular("Boca 4", "ole", 5),
            titular("Otra cosa", "clarin", 0),
            titular("Más cosas", "xataka", 1),
            titular("Y más", "hn", 2),
        ];
        let n = ordenar_sin_ia(&lista, &i, ahora());
        assert_eq!(n.len(), 6);
        assert!(n[0].titulo.starts_with("Boca"));
        assert_eq!(n.iter().filter(|x| x.fuente == "ole").count(), POR_FUENTE);
        assert_eq!(n[0].resumen, "Bajada de Boca 1");
    }

    #[test]
    fn pedido_y_respuesta_de_la_ia() {
        let lista = vec![titular("A", "ole", 1), titular("B", "ole", 1), titular("C", "x", 1), titular("D", "x", 1)];
        let pedido = armar_pedido(&lista, "Boca");
        let Mensaje::Usuario(texto) = &pedido[1] else { panic!() };
        assert!(texto.starts_with("1. [ole] A — Bajada de A"));

        let r = leer_respuesta(
            "Acá van:\n```json\n{\"noticias\":[{\"n\":3,\"resumen\":\" Ce \"},{\"n\":1,\"resumen\":\"A\"},{\"n\":3},{\"n\":99},{\"n\":2,\"resumen\":\"B\"}]}\n```",
            &lista,
        )
        .unwrap();
        let titulos: Vec<&str> = r.iter().map(|n| n.titulo.as_str()).collect();
        assert_eq!(titulos, vec!["C", "A", "B"]);
        assert_eq!(r[0].resumen, "Ce");
        assert!(leer_respuesta("no sé", &lista).is_none());

        // El título manda sobre el número (acá el número está corrido en uno)
        let largos = vec![
            titular("Boca ganó el clásico", "ole", 1),
            titular("La Selección ya tiene rival", "ole", 1),
            titular("Salió un modelo de IA nuevo", "x", 1),
        ];
        let r = leer_respuesta(
            r#"{"noticias":[{"n":1,"titulo":"La Selección ya tiene rival","resumen":"S"},{"n":2,"titulo":"Salió un modelo de IA nuevo","resumen":"I"},{"n":0,"titulo":"Boca ganó el clásico","resumen":"B"}]}"#,
            &largos,
        )
        .unwrap();
        let pares: Vec<(&str, &str)> = r.iter().map(|n| (n.titulo.as_str(), n.resumen.as_str())).collect();
        assert_eq!(pares, vec![("La Selección ya tiene rival", "S"), ("Salió un modelo de IA nuevo", "I"), ("Boca ganó el clásico", "B")]);
        assert!(leer_respuesta("{\"noticias\":[{\"n\":1}]}", &lista).is_none());
    }
}
