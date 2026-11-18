//! Atajo global de teclado para abrir y cerrar el panel.
//! "Global" quiere decir que funciona aunque Nova no tenga el foco.
//!
//! El atajo se guarda en las preferencias como texto, por ejemplo
//! "Ctrl+Shift+Space" o "Alt+KeyN" (modificadores + código de tecla del
//! navegador, `KeyboardEvent.code`). El plugin sabe leer ese formato.

use tauri::{App, AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::ventana;

/// Convierte el texto en un atajo y verifica que tenga al menos un modificador
/// (un atajo global de una sola tecla taparía esa tecla en todo Windows).
pub fn parsear(texto: &str) -> Result<Shortcut, String> {
    let atajo: Shortcut = texto
        .parse()
        .map_err(|_| format!("No entiendo el atajo \"{texto}\"."))?;
    if atajo.mods.is_empty() {
        return Err("El atajo necesita Ctrl, Alt o Shift.".into());
    }
    Ok(atajo)
}

/// Registra el plugin y el atajo guardado. Se llama una vez, en el `setup`.
///
/// `Box<dyn std::error::Error>` es "cualquier tipo de error": permite usar `?`
/// con errores de distintas bibliotecas sin convertirlos a mano.
pub fn registrar(app: &App, guardado: &str) -> Result<(), Box<dyn std::error::Error>> {
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            // Hay un solo atajo registrado a la vez, así que cualquier atajo
            // que llegue es el de abrir y cerrar. Solo al apretar (no al
            // soltar), si no se ejecutaría dos veces.
            .with_handler(|app, _atajo, evento| {
                if evento.state() == ShortcutState::Pressed {
                    ventana::alternar(app);
                }
            })
            .build(),
    )?;

    // Si el guardado no se entiende, se usa el de fábrica. Si otra app ya tomó
    // el atajo, lo informamos por consola pero no tiramos abajo Nova (se puede
    // abrir desde la bandeja y cambiar el atajo en ajustes).
    let atajo = parsear(guardado)
        .or_else(|_| parsear(crate::repos::preferencias::ATAJO_POR_DEFECTO))
        .map_err(|e| e.to_string())?;
    if let Err(e) = app.global_shortcut().register(atajo) {
        eprintln!("No se pudo registrar el atajo global: {e}");
    }
    Ok(())
}

/// Cambia el atajo. Si el nuevo no se puede registrar (otra app lo usa),
/// vuelve a dejar el anterior y devuelve el error. Devuelve el texto
/// normalizado que conviene guardar.
///
/// `<R: Runtime>` hace que la función sirva para cualquier "motor" de Tauri;
/// en la práctica siempre es el de escritorio.
pub fn cambiar<R: Runtime>(app: &AppHandle<R>, actual: &str, nuevo: &str) -> Result<String, String> {
    let nuevo_atajo = parsear(nuevo)?;
    let accesos = app.global_shortcut();
    let actual_atajo = parsear(actual).ok();

    if actual_atajo == Some(nuevo_atajo) {
        return Ok(nuevo.to_string());
    }
    if let Some(viejo) = actual_atajo {
        let _ = accesos.unregister(viejo);
    }
    if accesos.register(nuevo_atajo).is_err() {
        if let Some(viejo) = actual_atajo {
            let _ = accesos.register(viejo);
        }
        return Err(format!("No se pudo usar \"{nuevo}\": puede que otra app ya lo tenga."));
    }
    Ok(nuevo.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entiende_los_formatos_que_manda_la_interfaz() {
        assert!(parsear("Ctrl+Shift+Space").is_ok());
        assert!(parsear("Alt+KeyN").is_ok());
        assert!(parsear("Ctrl+Alt+Digit1").is_ok());
        assert!(parsear("Ctrl+F5").is_ok());
        assert_eq!(parsear("Ctrl+Shift+Space").unwrap(), parsear("ctrl+shift+space").unwrap());
    }

    #[test]
    fn rechaza_atajos_sin_modificador_o_raros() {
        assert!(parsear("Space").is_err());
        assert!(parsear("Ctrl+Nada").is_err());
        assert!(parsear("").is_err());
    }
}
