//! Dónde dejó el usuario cada widget del panel (v0.7.1).
//!
//! Se guarda cuánto se corrió cada uno de su lugar de siempre, como fracción
//! del panel (x: 0.1 = un décimo del ancho), así se ve igual si cambia la
//! pantalla. Que no se pisen ni tapen la esfera lo resuelve la interfaz al
//! soltarlos: acá solo se valida lo que llega.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Los widgets que se pueden mover.
pub const WIDGETS: [&str; 6] = ["resumen", "calendario", "facu", "futbol", "responsabilidades", "noticias"];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Corrimiento {
    pub x: f64,
    pub y: f64,
}

/// Widget → corrimiento. `BTreeMap` es un diccionario con las claves
/// ordenadas: el JSON guardado sale siempre igual.
pub type Disposicion = BTreeMap<String, Corrimiento>;

/// Rechaza widgets desconocidos y números inválidos (`NaN`, infinito), y no
/// deja que un widget se vaya más de un panel entero de su lugar. Los que
/// quedaron en su lugar no se guardan.
pub fn validar(disposicion: Disposicion) -> Result<Disposicion, String> {
    let mut valida = Disposicion::new();
    for (widget, c) in disposicion {
        if !WIDGETS.contains(&widget.as_str()) {
            return Err(format!("No conozco el widget \"{widget}\"."));
        }
        if !c.x.is_finite() || !c.y.is_finite() {
            return Err("La posición no es un número.".into());
        }
        let c = Corrimiento { x: c.x.clamp(-1.0, 1.0), y: c.y.clamp(-1.0, 1.0) };
        if c.x.abs() > 1e-4 || c.y.abs() > 1e-4 {
            valida.insert(widget, c);
        }
    }
    Ok(valida)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn una(widget: &str, x: f64, y: f64) -> Disposicion {
        Disposicion::from([(widget.to_string(), Corrimiento { x, y })])
    }

    #[test]
    fn acepta_los_widgets_conocidos() {
        assert_eq!(validar(una("facu", 0.2, -0.1)), Ok(una("facu", 0.2, -0.1)));
    }

    #[test]
    fn rechaza_un_widget_desconocido() {
        assert!(validar(una("spotify", 0.1, 0.1)).is_err());
    }

    #[test]
    fn rechaza_lo_que_no_es_numero() {
        assert!(validar(una("facu", f64::NAN, 0.0)).is_err());
        assert!(validar(una("facu", 0.0, f64::INFINITY)).is_err());
    }

    #[test]
    fn no_deja_irse_mas_de_un_panel() {
        assert_eq!(validar(una("futbol", 5.0, -3.0)), Ok(una("futbol", 1.0, -1.0)));
    }

    #[test]
    fn los_que_estan_en_su_lugar_no_se_guardan() {
        assert_eq!(validar(una("noticias", 0.0, 0.0)), Ok(Disposicion::new()));
    }
}
