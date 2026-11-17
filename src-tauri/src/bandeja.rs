//! Ícono en la bandeja del sistema (al lado del reloj) con "Abrir" y "Salir".

use tauri::{
    App,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::ventana;

pub fn crear(app: &App) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", "Abrir Nova", true, None::<&str>)?;
    let salir = MenuItem::with_id(app, "salir", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&abrir, &salir])?;

    let mut constructor = TrayIconBuilder::with_id("nova")
        .tooltip("Nova — Ctrl+Shift+Espacio")
        .menu(&menu)
        // El menú se abre con clic derecho; el clic izquierdo abre el panel.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id.as_ref() {
            "abrir" => ventana::mostrar(app),
            "salir" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = evento
            {
                ventana::mostrar(bandeja.app_handle());
            }
        });

    if let Some(icono) = app.default_window_icon() {
        constructor = constructor.icon(icono.clone());
    }

    constructor.build(app)?;
    Ok(())
}
