//! Lo que Nova sabe de vos: cómo llamarte, cómo hablarte y un texto libre.
//! Se guarda en preferencias (clave `perfil`) y va en las instrucciones de
//! cada turno, así no depende del historial (que solo trae las últimas 2 h).

use serde::{Deserialize, Serialize};

/// Hasta acá llega "Sobre vos": cada carácter viaja en cada pedido a la IA
/// (y el plan gratis de Groq cuenta tokens por minuto).
pub const LARGO_SOBRE_VOS: usize = 400;
const LARGO_NOMBRE: usize = 40;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tono {
    /// Lo de siempre: rioplatense, con onda y breve.
    #[default]
    ConOnda,
    /// Como un amigo: bien suelto.
    Informal,
    /// Más tranquilo y prolijo, sin perder el voseo.
    Sobrio,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Perfil {
    /// Cómo quiere que lo llamen. Vacío: sin nombre.
    pub como_llamarte: String,
    pub tono: Tono,
    /// Algún chiste corto cada tanto.
    pub chistes: bool,
    /// Texto libre: qué estudia, dónde, lo que quiera que Nova tenga en cuenta.
    pub sobre_vos: String,
}

impl Perfil {
    pub fn validar(self) -> Result<Self, String> {
        let como_llamarte = una_linea(&self.como_llamarte);
        if como_llamarte.chars().count() > LARGO_NOMBRE {
            return Err("El nombre es muy largo.".into());
        }
        let sobre_vos = self.sobre_vos.trim().to_string();
        if sobre_vos.chars().count() > LARGO_SOBRE_VOS {
            return Err(format!("\"Sobre vos\" puede tener hasta {LARGO_SOBRE_VOS} letras."));
        }
        Ok(Perfil { como_llamarte, sobre_vos, ..self })
    }

    /// El pedazo de las instrucciones de sistema. Vacío si no hay nada que decir.
    pub fn instrucciones(&self) -> String {
        let mut lineas: Vec<String> = Vec::new();
        if !self.como_llamarte.is_empty() {
            lineas.push(format!("Llamalo {}.", self.como_llamarte));
        }
        match self.tono {
            Tono::ConOnda => {}
            Tono::Informal => lineas.push("Hablale bien informal, como un amigo.".into()),
            Tono::Sobrio => lineas.push("Hablale tranquilo y prolijo, sin chistes ni exageraciones.".into()),
        }
        if self.chistes && self.tono != Tono::Sobrio {
            lineas.push("Cada tanto tirá algún chiste corto, sin forzarlo y sin que tape la respuesta.".into());
        }
        if !self.sobre_vos.is_empty() {
            // Entre comillas: es lo que escribió el usuario, no una regla más.
            lineas.push(format!("Lo que te contó de sí mismo: «{}»", self.sobre_vos));
        }
        if lineas.is_empty() {
            return String::new();
        }
        format!("\n\nSobre el usuario:\n- {}", lineas.join("\n- "))
    }
}

/// Saca saltos de línea y espacios de más (el nombre es una sola línea).
fn una_linea(texto: &str) -> String {
    texto.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfil(nombre: &str, tono: Tono, chistes: bool, sobre_vos: &str) -> Perfil {
        Perfil { como_llamarte: nombre.into(), tono, chistes, sobre_vos: sobre_vos.into() }
    }

    #[test]
    fn sin_perfil_no_agrega_nada() {
        assert_eq!(Perfil::default().instrucciones(), "");
    }

    #[test]
    fn arma_el_nombre_el_tono_los_chistes_y_lo_que_conto() {
        let texto = perfil("Juli", Tono::Informal, true, "Estudio Diseño en la UBA.").instrucciones();
        assert!(texto.starts_with("\n\nSobre el usuario:\n- Llamalo Juli."));
        assert!(texto.contains("- Hablale bien informal, como un amigo."));
        assert!(texto.contains("- Cada tanto tirá algún chiste corto"));
        assert!(texto.contains("- Lo que te contó de sí mismo: «Estudio Diseño en la UBA.»"));
    }

    #[test]
    fn el_tono_sobrio_no_lleva_chistes() {
        let texto = perfil("", Tono::Sobrio, true, "").instrucciones();
        assert!(texto.contains("sin chistes"));
        assert!(!texto.contains("Cada tanto"));
    }

    #[test]
    fn con_onda_es_lo_de_siempre() {
        assert_eq!(perfil("", Tono::ConOnda, false, "").instrucciones(), "");
    }

    #[test]
    fn validar_limpia_y_pone_limites() {
        let p = perfil("  Juli \n Pérez ", Tono::ConOnda, false, "  hola  ").validar().unwrap();
        assert_eq!(p.como_llamarte, "Juli Pérez");
        assert_eq!(p.sobre_vos, "hola");

        assert!(perfil(&"a".repeat(41), Tono::ConOnda, false, "").validar().is_err());
        assert!(perfil("", Tono::ConOnda, false, &"á".repeat(LARGO_SOBRE_VOS)).validar().is_ok());
        assert!(perfil("", Tono::ConOnda, false, &"á".repeat(LARGO_SOBRE_VOS + 1)).validar().is_err());
    }

    #[test]
    fn lee_lo_guardado_aunque_le_falten_campos() {
        let p: Perfil = serde_json::from_str(r#"{"comoLlamarte":"Juli"}"#).unwrap();
        assert_eq!(p, perfil("Juli", Tono::ConOnda, false, ""));
        let json = serde_json::to_value(perfil("", Tono::Informal, true, "")).unwrap();
        assert_eq!(json["tono"], "informal");
    }
}
