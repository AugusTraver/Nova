//! Cuánto cupo le queda a cada proveedor de IA (v0.7.3).
//!
//! Groq manda en cada respuesta cuántos tokens le quedan en el minuto
//! (`x-ratelimit-remaining-tokens`) y cuándo se renuevan
//! (`x-ratelimit-reset-tokens`). Cuando alguno contesta 429 ("te pasaste"),
//! dice cuánto esperar. Con eso, la cadena de proveedores (`cadena.rs`) sabe
//! si un pedido va a entrar antes de mandarlo, y si no, lo manda al respaldo.
//!
//! La decisión es pura (`Cupo::alcanza`, con tests). Lo que se recuerda vive
//! en un `static` con un `Mutex`: se comparte entre los pedidos de todo el
//! programa (cada turno arma su cliente, pero el cupo es del proveedor).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use super::proveedor::TipoProveedor;

/// Lo que se sabe del cupo de un proveedor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cupo {
    /// Dijo 429 (o falló el servidor): no se le pide nada hasta esta hora.
    pub pausado_hasta: Option<Instant>,
    /// Tokens que le quedaban en el minuto, y hasta cuándo vale ese dato.
    pub quedan: Option<(u64, Instant)>,
}

impl Cupo {
    /// ¿Entra un pedido de `estimado` tokens ahora?
    pub fn alcanza(&self, estimado: u64, ahora: Instant) -> bool {
        if self.pausado_hasta.is_some_and(|hasta| ahora < hasta) {
            return false;
        }
        match self.quedan {
            Some((tokens, hasta)) if ahora < hasta => tokens >= estimado,
            _ => true,
        }
    }

    pub fn en_pausa(&self, ahora: Instant) -> bool {
        self.pausado_hasta.is_some_and(|hasta| ahora < hasta)
    }
}

static CUPOS: LazyLock<Mutex<HashMap<TipoProveedor, Cupo>>> = LazyLock::new(Mutex::default);

fn con_cupos<T>(hacer: impl FnOnce(&mut HashMap<TipoProveedor, Cupo>) -> T) -> T {
    // Si otro hilo entró en pánico con el candado tomado, el dato sigue
    // sirviendo (`into_inner`): es solo una estimación.
    hacer(&mut CUPOS.lock().unwrap_or_else(|e| e.into_inner()))
}

pub fn de(tipo: TipoProveedor) -> Cupo {
    con_cupos(|c| c.get(&tipo).copied().unwrap_or_default())
}

/// Lo que dijeron los encabezados de una respuesta que salió bien.
pub fn anotar_quedan(tipo: TipoProveedor, tokens: u64, se_renueva_en: Duration) {
    let ahora = Instant::now();
    con_cupos(|c| {
        let cupo = c.entry(tipo).or_default();
        cupo.quedan = Some((tokens, ahora + se_renueva_en));
        cupo.pausado_hasta = None;
    });
}

/// Dijo que se pasó (429) o falló el servidor: no pedirle por un rato.
pub fn pausar(tipo: TipoProveedor, por: Duration) {
    let hasta = Instant::now() + por;
    con_cupos(|c| {
        let cupo = c.entry(tipo).or_default();
        // Si ya estaba pausado por más tiempo (el límite del día), queda eso.
        if cupo.pausado_hasta.is_none_or(|antes| antes < hasta) {
            cupo.pausado_hasta = Some(hasta);
        }
    });
}

/// "7.66s", "1m2.5s", "2h3m": el formato de los encabezados de Groq.
pub fn leer_duracion(texto: &str) -> Option<Duration> {
    let mut total = 0.0;
    let mut numero = String::new();
    let mut alguna = false;
    let mut chars = texto.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '0'..='9' | '.' => numero.push(c),
            'h' | 'm' | 's' => {
                // "ms" son milisegundos.
                let (factor, salto) = match (c, chars.peek()) {
                    ('m', Some('s')) => (0.001, true),
                    ('h', _) => (3600.0, false),
                    ('m', _) => (60.0, false),
                    _ => (1.0, false),
                };
                if salto {
                    chars.next();
                }
                total += numero.parse::<f64>().ok()? * factor;
                numero.clear();
                alguna = true;
            }
            _ => return None,
        }
    }
    if !numero.is_empty() {
        // Un número suelto son segundos (como `retry-after: 12`).
        total += numero.parse::<f64>().ok()?;
        alguna = true;
    }
    alguna.then(|| Duration::from_secs_f64(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_si_entra() {
        let ahora = Instant::now();
        let en = |s| ahora + Duration::from_secs(s);
        assert!(Cupo::default().alcanza(5000, ahora), "sin datos, se intenta");
        let poco = Cupo { quedan: Some((1800, en(20))), ..Default::default() };
        assert!(!poco.alcanza(2500, ahora));
        assert!(poco.alcanza(1500, ahora));
        // El dato venció: el minuto se renovó.
        assert!(poco.alcanza(2500, en(21)));
        let pausado = Cupo { pausado_hasta: Some(en(9)), quedan: Some((8000, en(60))) };
        assert!(!pausado.alcanza(10, ahora));
        assert!(pausado.en_pausa(ahora));
        assert!(pausado.alcanza(10, en(10)));
        assert!(!pausado.en_pausa(en(10)));
    }

    #[test]
    fn lee_las_duraciones_de_groq() {
        assert_eq!(leer_duracion("7.66s"), Some(Duration::from_secs_f64(7.66)));
        assert_eq!(leer_duracion("1m2.5s"), Some(Duration::from_secs_f64(62.5)));
        assert_eq!(leer_duracion("2h3m"), Some(Duration::from_secs(7380)));
        assert_eq!(leer_duracion("120ms"), Some(Duration::from_millis(120)));
        assert_eq!(leer_duracion("12"), Some(Duration::from_secs(12)));
        assert_eq!(leer_duracion(""), None);
        assert_eq!(leer_duracion("pronto"), None);
    }

    #[test]
    fn recuerda_por_proveedor() {
        // Ollama no tiene límites: ningún otro test lo usa, así que no se pisan.
        anotar_quedan(TipoProveedor::Ollama, 100, Duration::from_secs(30));
        assert!(!de(TipoProveedor::Ollama).alcanza(500, Instant::now()));
        pausar(TipoProveedor::Ollama, Duration::from_secs(600));
        pausar(TipoProveedor::Ollama, Duration::from_secs(5));
        let cupo = de(TipoProveedor::Ollama);
        assert!(cupo.pausado_hasta.unwrap() > Instant::now() + Duration::from_secs(500), "queda la pausa más larga");
        anotar_quedan(TipoProveedor::Ollama, 9000, Duration::from_secs(30));
        assert!(de(TipoProveedor::Ollama).alcanza(500, Instant::now()), "una respuesta buena saca la pausa");
    }
}
