//! Secretos (API keys) en el llavero del sistema. En Windows es el
//! Administrador de credenciales: la key queda cifrada con tu usuario de
//! Windows y nunca en un archivo de Nova ni en el repo.
//!
//! El frontend nunca recibe la key: solo sabe si hay una guardada.

use std::sync::Once;

use crate::error::{Error, Resultado};

const SERVICIO: &str = "com.nova.app";

/// Elige el almacén una sola vez (`Once` garantiza que no se repita aunque
/// lo llamen dos hilos a la vez).
fn preparar() {
    static INICIO: Once = Once::new();
    INICIO.call_once(|| {
        #[cfg(windows)]
        match windows_native_keyring_store::Store::new() {
            Ok(almacen) => keyring_core::set_default_store(almacen),
            Err(e) => eprintln!("No se pudo abrir el llavero de Windows: {e}"),
        }
    });
}

fn entrada(cuenta: &str) -> Resultado<keyring_core::Entry> {
    preparar();
    keyring_core::Entry::new(SERVICIO, cuenta).map_err(error_llavero)
}

fn error_llavero(e: keyring_core::Error) -> Error {
    Error::Sistema(format!("No pude usar el llavero del sistema: {e}"))
}

pub fn leer(cuenta: &str) -> Resultado<Option<String>> {
    match entrada(cuenta)?.get_password() {
        Ok(valor) => Ok(Some(valor)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(e) => Err(error_llavero(e)),
    }
}

pub fn guardar(cuenta: &str, valor: &str) -> Resultado<()> {
    entrada(cuenta)?.set_password(valor).map_err(error_llavero)
}

pub fn borrar(cuenta: &str) -> Resultado<()> {
    match entrada(cuenta)?.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(e) => Err(error_llavero(e)),
    }
}
