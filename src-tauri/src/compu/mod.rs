//! Nova usa la compu (v0.7.1, SPEC 5.9): abrir páginas, videos, programas y
//! archivos, y modificar archivos de texto con confirmación.
//!
//! - `dominio::compu`: las reglas puras (direcciones, rutas, antes y después).
//! - `sistema`: lo que se le pide a Windows.
//! - `programas`, `archivos`, `youtube`: cada cosa que se puede abrir o tocar.
//! - `herramientas`: lo que ve el modelo.
//!
//! El modelo nunca arma un comando ni una ruta que se ejecute: cada acción es
//! una herramienta con argumentos que Rust valida.

pub mod archivos;
pub mod herramientas;
pub mod programas;
pub mod sistema;
pub mod youtube;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::dominio::compu::{CarpetasDelSistema, carpeta_prohibida, sin_prefijo_largo, validar_direccion};
use crate::dominio::lenguaje::normalizar;
use programas::Programa;
use sistema::TeclaMedios;

/// Una página que el usuario guardó con un nombre ("facultad" → campus).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginaGuardada {
    pub nombre: String,
    pub direccion: String,
}

/// Lo que se configura en Ajustes (preferencia `compu`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesCompu {
    /// El interruptor general: apagado, no se manda ninguna herramienta.
    pub activa: bool,
    /// Las carpetas donde Nova puede abrir, leer y cambiar archivos.
    pub carpetas: Vec<String>,
    pub paginas: Vec<PaginaGuardada>,
}

impl Default for AjustesCompu {
    fn default() -> Self {
        AjustesCompu { activa: true, carpetas: Vec::new(), paginas: Vec::new() }
    }
}

const MAXIMO_CARPETAS: usize = 20;
const MAXIMO_PAGINAS: usize = 50;

impl AjustesCompu {
    /// Si nunca se guardó: Documentos, Escritorio y Descargas.
    pub fn por_defecto() -> Self {
        AjustesCompu {
            carpetas: sistema::carpetas_por_defecto().iter().map(|c| c.display().to_string()).collect(),
            ..Default::default()
        }
    }

    /// Valida lo que llega de Ajustes. Las carpetas se resuelven (tienen que
    /// existir) y no pueden ser del sistema.
    pub fn validar(self, sistema: &CarpetasDelSistema) -> Result<Self, String> {
        let mut carpetas: Vec<String> = Vec::new();
        for c in self.carpetas.iter().map(|c| c.trim()).filter(|c| !c.is_empty()) {
            let real = Path::new(c).canonicalize().map_err(|_| format!("No encontré la carpeta «{c}»."))?;
            if !real.is_dir() {
                return Err(format!("«{c}» no es una carpeta."));
            }
            let real = sin_prefijo_largo(&real);
            if let Some(motivo) = carpeta_prohibida(&real, sistema) {
                return Err(format!("{}: {motivo}", real.display()));
            }
            let texto = real.display().to_string();
            if !carpetas.iter().any(|x| x.eq_ignore_ascii_case(&texto)) {
                carpetas.push(texto);
            }
        }
        if carpetas.len() > MAXIMO_CARPETAS {
            return Err(format!("Como mucho {MAXIMO_CARPETAS} carpetas."));
        }
        let mut paginas: Vec<PaginaGuardada> = Vec::new();
        for p in self.paginas {
            let nombre = p.nombre.trim().to_string();
            if nombre.is_empty() && p.direccion.trim().is_empty() {
                continue;
            }
            if nombre.is_empty() || nombre.chars().count() > 60 {
                return Err("Cada página necesita un nombre corto.".into());
            }
            let direccion = validar_direccion(&p.direccion).map_err(|e| format!("{nombre}: {e}"))?;
            if paginas.iter().any(|x| normalizar(&x.nombre) == normalizar(&nombre)) {
                return Err(format!("Hay dos páginas que se llaman «{nombre}»."));
            }
            paginas.push(PaginaGuardada { nombre, direccion });
        }
        if paginas.len() > MAXIMO_PAGINAS {
            return Err(format!("Como mucho {MAXIMO_PAGINAS} páginas."));
        }
        Ok(AjustesCompu { activa: self.activa, carpetas, paginas })
    }
}

/// Lo que de verdad hace cosas en la compu. Es un "trait" (una interfaz)
/// para que los tests usen uno que solo anota lo que le pidieron.
pub trait Manos: Send + Sync {
    fn abrir(&self, objetivo: &OsStr, argumento: Option<&Path>) -> Result<(), String>;
    fn apretar(&self, tecla: TeclaMedios, veces: u8) -> Result<(), String>;
}

pub struct ManosReales;

impl Manos for ManosReales {
    fn abrir(&self, objetivo: &OsStr, argumento: Option<&Path>) -> Result<(), String> {
        sistema::abrir(objetivo, argumento)
    }

    fn apretar(&self, tecla: TeclaMedios, veces: u8) -> Result<(), String> {
        sistema::apretar(tecla, veces)
    }
}

/// El estado de la compu dentro del asistente.
pub struct Compu {
    manos: Box<dyn Manos>,
    /// La lista de programas se arma la primera vez que se necesita y se
    /// rehace si no encuentra uno (puede que lo hayas instalado recién).
    programas: Mutex<Option<Arc<Vec<Programa>>>>,
    carpetas_inicio: Vec<PathBuf>,
    pub carpeta_copias: PathBuf,
    /// La key de YouTube (del llavero; en los tests, ninguna).
    pub leer_key_youtube: fn() -> Option<String>,
}

impl Compu {
    pub fn real(carpeta_copias: PathBuf) -> Self {
        Compu {
            manos: Box::new(ManosReales),
            programas: Mutex::default(),
            carpetas_inicio: programas::carpetas_del_menu_inicio(),
            carpeta_copias,
            leer_key_youtube: || crate::llavero::leer(youtube::CUENTA_LLAVERO).ok().flatten(),
        }
    }

    pub fn programas(&self, rehacer: bool) -> Arc<Vec<Programa>> {
        let mut guardados = self.programas.lock().unwrap_or_else(|e| e.into_inner());
        if rehacer || guardados.is_none() {
            *guardados = Some(Arc::new(programas::indexar(&self.carpetas_inicio)));
        }
        guardados.clone().unwrap_or_default()
    }

    pub fn manos(&self) -> &dyn Manos {
        self.manos.as_ref()
    }
}

#[cfg(test)]
pub mod prueba {
    use super::*;

    /// Anota lo que se le pide en vez de hacerlo.
    #[derive(Clone, Default)]
    pub struct ManosDePrueba(pub Arc<Mutex<Vec<String>>>);

    impl Manos for ManosDePrueba {
        fn abrir(&self, objetivo: &OsStr, argumento: Option<&Path>) -> Result<(), String> {
            let con = argumento.map(|a| format!(" con {}", a.display())).unwrap_or_default();
            self.0.lock().unwrap().push(format!("abrir {}{con}", objetivo.to_string_lossy()));
            Ok(())
        }

        fn apretar(&self, tecla: TeclaMedios, veces: u8) -> Result<(), String> {
            self.0.lock().unwrap().push(format!("tecla {tecla:?} x{veces}"));
            Ok(())
        }
    }

    impl Compu {
        pub fn de_prueba(programas: Vec<Programa>, copias: PathBuf) -> (Self, ManosDePrueba) {
            let manos = ManosDePrueba::default();
            let compu = Compu {
                manos: Box::new(manos.clone()),
                programas: Mutex::new(Some(Arc::new(programas))),
                carpetas_inicio: Vec::new(),
                carpeta_copias: copias,
                leer_key_youtube: || None,
            };
            (compu, manos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_los_ajustes() {
        let raiz = std::env::temp_dir().join(format!("nova-ajustes-compu-{}", std::process::id()));
        std::fs::create_dir_all(raiz.join("Facu")).unwrap();
        let facu = raiz.join("Facu").display().to_string();
        let sistema = CarpetasDelSistema { prohibidas: vec![PathBuf::from(r"C:\Windows")], perfil: None };

        let bien = AjustesCompu {
            activa: true,
            carpetas: vec![facu.clone(), format!("  {facu}  "), String::new()],
            paginas: vec![
                PaginaGuardada { nombre: " Facultad ".into(), direccion: "frba.utn.edu.ar".into() },
                PaginaGuardada { nombre: "".into(), direccion: "".into() },
            ],
        }
        .validar(&sistema)
        .unwrap();
        assert_eq!(bien.carpetas.len(), 1);
        assert!(!bien.carpetas[0].starts_with(r"\\?\"));
        assert_eq!(bien.paginas, [PaginaGuardada { nombre: "Facultad".into(), direccion: "https://frba.utn.edu.ar/".into() }]);

        let con = |carpetas: Vec<String>, paginas| AjustesCompu { activa: true, carpetas, paginas }.validar(&sistema);
        assert!(con(vec![r"C:\Windows\System32".into()], vec![]).is_err());
        assert!(con(vec![r"C:\".into()], vec![]).is_err());
        assert!(con(vec![raiz.join("no-existe").display().to_string()], vec![]).is_err());
        let pagina = |d: &str| vec![PaginaGuardada { nombre: "x".into(), direccion: d.into() }];
        assert!(con(vec![], pagina("javascript:alert(1)")).is_err());
        assert!(con(vec![], pagina("file:///C:/")).is_err());
        std::fs::remove_dir_all(&raiz).unwrap();
    }

    #[test]
    fn por_defecto_esta_activa() {
        let a: AjustesCompu = serde_json::from_str("{}").unwrap();
        assert!(a.activa && a.carpetas.is_empty());
    }
}
