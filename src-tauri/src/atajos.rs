//! Atajo global de teclado para abrir y cerrar el panel.
//! "Global" quiere decir que funciona aunque Nova no tenga el foco.

use tauri::App;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::ventana;

/// Atajo por defecto: Ctrl+Shift+Espacio. En v0.2 se vuelve configurable.
fn atajo_por_defecto() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)
}

/// Registra el plugin y el atajo. Se llama una vez, en el `setup` de la app.
///
/// `Box<dyn std::error::Error>` es "cualquier tipo de error": permite usar `?`
/// con errores de distintas bibliotecas sin convertirlos a mano.
pub fn registrar(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let atajo = atajo_por_defecto();

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            // `move` hace que el closure se quede con su propia copia de `atajo`.
            .with_handler(move |app, presionado, evento| {
                // Solo al apretar (no al soltar), si no se ejecutaría dos veces.
                if presionado == &atajo && evento.state() == ShortcutState::Pressed {
                    ventana::alternar(app);
                }
            })
            .build(),
    )?;

    // Si otra app ya tomó este atajo, el registro falla: lo informamos por
    // consola pero no tiramos abajo Nova (se puede abrir desde la bandeja).
    if let Err(e) = app.global_shortcut().register(atajo) {
        eprintln!("No se pudo registrar el atajo global: {e}");
    }

    Ok(())
}
