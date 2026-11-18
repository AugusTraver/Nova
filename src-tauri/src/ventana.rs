//! Manejo de la ventana del panel: mostrarla, ocultarla y darle el efecto de vidrio.
//!
//! La ventana se crea UNA sola vez al arrancar (oculta) y después solo se
//! muestra u oculta. Crear una ventana nueva cada vez tardaría demasiado para
//! el objetivo de abrir en menos de 300 ms.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Etiqueta de la ventana principal (definida en tauri.conf.json).
pub const VENTANA_PANEL: &str = "main";

/// Eventos que se mandan al frontend (ver src/app/puenteTauri.ts).
const EVENTO_ABRIR: &str = "panel:abrir";
const EVENTO_CERRAR: &str = "panel:cerrar";

fn panel(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(VENTANA_PANEL)
}

/// Aplica el desenfoque del fondo de Windows (acrylic) a la ventana.
/// Si el sistema no lo soporta, prueba con el "blur" clásico; si tampoco,
/// sigue sin efecto (el panel igual tiene su velo oscuro semitransparente).
pub fn aplicar_vidrio(ventana: &WebviewWindow) {
    // `Some((r, g, b, a))` es el tinte del vidrio: negro neutro y suave, para
    // que el cielo translúcido del panel (que ya pone el color) no se apague.
    let tinte = Some((8, 8, 9, 50));
    if window_vibrancy::apply_acrylic(ventana, tinte).is_err() {
        let _ = window_vibrancy::apply_blur(ventana, tinte);
    }
}

/// Ubica la ventana cubriendo todo el monitor donde está el mouse.
fn cubrir_monitor_actual(ventana: &WebviewWindow) {
    // `monitor_from_point` busca el monitor que contiene al cursor; si falla
    // (por ejemplo, no se pudo leer el cursor), se usa el monitor principal.
    let monitor = ventana
        .cursor_position()
        .ok()
        .and_then(|p| ventana.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| ventana.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let pos = monitor.position();
        let tam = monitor.size();
        let _ = ventana.set_position(PhysicalPosition::new(pos.x, pos.y));
        let _ = ventana.set_size(PhysicalSize::new(tam.width, tam.height));
    }
}

/// Muestra el panel y le avisa al frontend que arranque la animación de entrada.
pub fn mostrar(app: &AppHandle) {
    let Some(ventana) = panel(app) else { return };
    cubrir_monitor_actual(&ventana);
    let _ = ventana.show();
    let _ = ventana.set_focus();
    let _ = ventana.emit(EVENTO_ABRIR, ());
}

/// Le pide al frontend que cierre. El frontend hace la animación de salida
/// y recién al final llama al comando `ocultar_panel`.
pub fn pedir_cierre(app: &AppHandle) {
    if let Some(ventana) = panel(app) {
        let _ = ventana.emit(EVENTO_CERRAR, ());
    }
}

/// ¿Está el panel a la vista? (la tarea de fondo consulta más seguido).
pub fn abierto(app: &AppHandle) -> bool {
    panel(app).and_then(|v| v.is_visible().ok()).unwrap_or(false)
}

/// Abre si está cerrado, cierra si está abierto (lo usa el atajo global).
pub fn alternar(app: &AppHandle) {
    if abierto(app) {
        pedir_cierre(app);
    } else {
        mostrar(app);
    }
}

/// Comando llamado desde el frontend cuando terminó la animación de salida.
#[tauri::command]
pub fn ocultar_panel(app: AppHandle) -> Result<(), String> {
    match panel(&app) {
        Some(ventana) => ventana.hide().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}
