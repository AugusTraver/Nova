//! Punto de entrada de la app. `main.rs` solo llama a `run()`.

mod atajos;
mod bandeja;
mod ventana;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![ventana::ocultar_panel])
        .setup(|app| {
            if let Some(panel) = app.get_webview_window(ventana::VENTANA_PANEL) {
                ventana::aplicar_vidrio(&panel);
            }
            bandeja::crear(app)?;
            atajos::registrar(app)?;
            Ok(())
        })
        .on_window_event(|ventana_evento, evento| {
            // Alt+F4 o similar: no cerrar la app, solo ocultar el panel.
            // Nova sigue viva en la bandeja (para notificaciones y el atajo).
            if let WindowEvent::CloseRequested { api, .. } = evento {
                api.prevent_close();
                ventana::pedir_cierre(ventana_evento.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar Nova");
}
