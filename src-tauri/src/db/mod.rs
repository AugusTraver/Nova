//! Base de datos local (SQLite). Todos los datos personales viven acá, en un
//! archivo dentro de la carpeta de datos de la app (nunca en el repo).
//!
//! Hay una sola conexión, compartida por todos los comandos y protegida con un
//! `Mutex`: un candado que garantiza que solo un comando la use a la vez.
//! SQLite es tan rápido para estos volúmenes que esperar el candado no se nota.

mod migraciones;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::Error;

pub use migraciones::migrar;

/// Estado que Tauri guarda y le pasa a los comandos (`State<'_, Db>`).
pub struct Db(Mutex<Connection>);

impl Db {
    /// Abre (o crea) el archivo de la base y aplica las migraciones pendientes.
    pub fn abrir(ruta: &Path) -> Result<Self, Error> {
        let mut conexion = Connection::open(ruta)?;
        configurar(&conexion)?;
        migrar(&mut conexion)?;
        Ok(Db(Mutex::new(conexion)))
    }

    /// Toma el candado y devuelve la conexión. Si un comando anterior entró en
    /// pánico con el candado tomado, el `Mutex` queda "envenenado"; igual la
    /// recuperamos (`into_inner`), porque cada escritura va en una transacción
    /// y SQLite no queda a medias.
    pub fn conexion(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Ajustes de cada conexión. `foreign_keys` viene apagado por defecto en
/// SQLite: sin esto no funcionarían los `ON DELETE SET NULL` del esquema.
fn configurar(conexion: &Connection) -> Result<(), Error> {
    conexion.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )?;
    Ok(())
}

#[cfg(test)]
impl Db {
    /// Envuelve una conexión (en memoria) para los tests que usan `Db`.
    pub fn de_conexion(conexion: Connection) -> Self {
        Db(Mutex::new(conexion))
    }
}

/// Base en memoria, ya migrada. Solo para los tests.
#[cfg(test)]
pub fn en_memoria() -> Connection {
    let mut conexion = Connection::open_in_memory().expect("abrir base en memoria");
    configurar(&conexion).expect("configurar");
    migrar(&mut conexion).expect("migrar");
    conexion
}
