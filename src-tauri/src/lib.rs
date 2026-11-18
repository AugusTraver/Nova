//! Punto de entrada de la app. `main.rs` solo llama a `run()`.

mod atajos;
mod bandeja;
mod comandos;
mod db;
mod dominio;
mod error;
mod repos;
mod ventana;

use tauri::{Manager, WindowEvent};

use crate::db::Db;
use crate::repos::{Ahora, automaticas, preferencias};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            ventana::ocultar_panel,
            comandos::listar_materias,
            comandos::crear_materia,
            comandos::editar_materia,
            comandos::borrar_materia,
            comandos::listar_ocurrencias,
            comandos::crear_evento,
            comandos::editar_evento,
            comandos::borrar_evento,
            comandos::saltear_fecha,
            comandos::listar_responsabilidades,
            comandos::crear_responsabilidad,
            comandos::editar_responsabilidad,
            comandos::marcar_responsabilidad,
            comandos::borrar_responsabilidad,
            comandos::sincronizar_automaticas,
            comandos::obtener_ajustes,
            comandos::guardar_ajustes,
        ])
        .setup(|app| {
            // La base va en la carpeta de datos de la app
            // (%APPDATA%\com.nova.app\ o similar), nunca dentro del repo.
            let carpeta = app.path().app_data_dir()?;
            std::fs::create_dir_all(&carpeta)?;
            let db = Db::abrir(&carpeta.join("nova.db"))?;

            let atajo = {
                let mut conexion = db.conexion();
                // Al arrancar: los cumpleaños que ya pasaron saltan al año siguiente.
                if let Err(e) = automaticas::sincronizar_todo(&mut conexion, &Ahora::del_sistema()) {
                    eprintln!("No se pudieron sincronizar las automáticas: {e}");
                }
                preferencias::atajo(&conexion).unwrap_or_else(|_| preferencias::ATAJO_POR_DEFECTO.into())
            }; // acá se suelta el candado de la conexión
            app.manage(db);

            if let Some(panel) = app.get_webview_window(ventana::VENTANA_PANEL) {
                ventana::aplicar_vidrio(&panel);
            }
            bandeja::crear(app)?;
            atajos::registrar(app, &atajo)?;
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
