//! Migraciones versionadas.
//!
//! SQLite guarda un número entero libre en el archivo (`PRAGMA user_version`).
//! Lo usamos como "versión del esquema": si vale 0, falta aplicar todo; si vale
//! 1, ya se aplicó la primera, etc. Para cambiar el esquema se AGREGA un
//! archivo nuevo a la lista; nunca se edita uno que ya corrió.

use rusqlite::Connection;

use crate::error::Error;

/// `include_str!` copia el contenido del .sql adentro del ejecutable al
/// compilar, así no hay que distribuir los archivos aparte.
const MIGRACIONES: &[&str] = &[
    include_str!("../../migraciones/001_inicial.sql"),
    include_str!("../../migraciones/002_mensajes_datos.sql"),
    include_str!("../../migraciones/003_cache_externa.sql"),
    include_str!("../../migraciones/004_avisos.sql"),
    include_str!("../../migraciones/005_cambios_archivos.sql"),
];

pub fn migrar(conexion: &mut Connection) -> Result<(), Error> {
    // SQLite guarda enteros con signo (i64); lo pasamos a usize para indexar.
    let actual: i64 = conexion.pragma_query_value(None, "user_version", |fila| fila.get(0))?;
    let actual = usize::try_from(actual).unwrap_or(0);

    for (indice, sql) in MIGRACIONES.iter().enumerate().skip(actual) {
        // Cada migración en su transacción: si falla, no queda a medias.
        let tx = conexion.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (indice + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aplica_todo_y_es_idempotente() {
        let mut conexion = Connection::open_in_memory().unwrap();
        migrar(&mut conexion).unwrap();
        migrar(&mut conexion).unwrap(); // la segunda vez no hace nada

        let version: i64 = conexion
            .pragma_query_value(None, "user_version", |f| f.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRACIONES.len());

        let tablas: i64 = conexion
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN
                 ('materias','eventos','eventos_excepciones','responsabilidades','mensajes','preferencias',
                  'cache_externa','pedidos_por_dia')",
                [],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(tablas, 8);
    }
}
