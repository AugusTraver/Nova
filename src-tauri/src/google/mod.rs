//! Google Calendar (v0.7.2): el permiso (`oauth`), la API (`calendario`) y
//! la sincronización en los dos sentidos (`sincronizar`). La traducción de
//! eventos y las reglas puras están en `dominio::google`.

pub mod calendario;
pub mod oauth;
pub mod sincronizar;

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Resultado;
use crate::llavero;

/// Lo que Tauri guarda de Google (con `app.manage`): el cliente de la API
/// (con el permiso corto en memoria) y si hay una conexión en curso.
pub struct Google {
    pub api: calendario::ClienteCalendar,
    conectando: AtomicBool,
    cancelar: AtomicBool,
}

impl Google {
    pub fn nuevo() -> Resultado<Self> {
        Ok(Google { api: calendario::ClienteCalendar::nuevo()?, conectando: AtomicBool::new(false), cancelar: AtomicBool::new(false) })
    }

    /// Marca que empezó una conexión. `false` si ya había una.
    pub fn empezar_conexion(&self) -> bool {
        self.cancelar.store(false, Ordering::SeqCst);
        !self.conectando.swap(true, Ordering::SeqCst)
    }

    pub fn terminar_conexion(&self) {
        self.conectando.store(false, Ordering::SeqCst);
    }

    pub fn conectando(&self) -> bool {
        self.conectando.load(Ordering::SeqCst)
    }

    pub fn cancelar_conexion(&self) {
        self.cancelar.store(true, Ordering::SeqCst);
    }

    pub fn cancelado(&self) -> bool {
        self.cancelar.load(Ordering::SeqCst)
    }
}

/// ¿Hay un permiso de Google guardado?
pub fn conectado() -> bool {
    llavero::leer(oauth::CUENTA_PERMISO).ok().flatten().is_some()
}
