//! Ícono en la bandeja del sistema (al lado del reloj) con "Abrir", "Oye
//! Nova" y "Salir". Mientras espera "Oye Nova" con el panel cerrado, el ícono
//! lleva un puntito y el texto lo dice: así sabés que el micrófono está abierto.

use tauri::{
    App, AppHandle, Manager, Wry,
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::db::Db;
use crate::repos::preferencias;
use crate::ventana;
use crate::voz::Voz;
use crate::voz::motor::Orden;

const ID: &str = "nova";
const TEXTO: &str = "Nova — Ctrl+Shift+Espacio";
const TEXTO_ESCUCHANDO: &str = "Nova — escuchando «Oye Nova»";

/// El tilde del menú y los dos íconos (Tauri los guarda en el estado).
pub struct Bandeja {
    palabra: CheckMenuItem<Wry>,
    icono: Option<Image<'static>>,
    icono_escuchando: Option<Image<'static>>,
}

pub fn crear(app: &App) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", "Abrir Nova", true, None::<&str>)?;
    let activa = {
        let db = app.state::<Db>();
        let conexion = db.conexion();
        preferencias::voz(&conexion).map(|v| v.activada && v.palabra_clave).unwrap_or(false)
    };
    let palabra = CheckMenuItem::with_id(app, "palabra", "Escuchar «Oye Nova»", true, activa, None::<&str>)?;
    let salir = MenuItem::with_id(app, "salir", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&abrir, &palabra, &salir])?;

    let mut constructor = TrayIconBuilder::with_id(ID)
        .tooltip(TEXTO)
        .menu(&menu)
        // El menú se abre con clic derecho; el clic izquierdo abre el panel.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id.as_ref() {
            "abrir" => ventana::mostrar(app),
            "palabra" => alternar_palabra(app),
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

    let icono = app.default_window_icon().map(|i| i.clone().to_owned());
    if let Some(i) = &icono {
        constructor = constructor.icon(i.clone());
    }
    constructor.build(app)?;
    let icono_escuchando = icono.as_ref().map(con_puntito);
    app.manage(Bandeja { palabra, icono, icono_escuchando });
    Ok(())
}

/// El tilde del menú: prende o apaga "Oye Nova" (y lo guarda).
fn alternar_palabra(app: &AppHandle) {
    let ajustes = {
        let db = app.state::<Db>();
        let conexion = db.conexion();
        let Ok(mut ajustes) = preferencias::voz(&conexion) else { return };
        ajustes.palabra_clave = !ajustes.palabra_clave;
        if let Err(e) = preferencias::guardar(&conexion, preferencias::VOZ, &ajustes) {
            return eprintln!("No pude guardar «Oye Nova»: {e}");
        }
        ajustes
    };
    if let Some(b) = app.try_state::<Bandeja>() {
        let _ = b.palabra.set_checked(ajustes.activada && ajustes.palabra_clave);
    }
    app.state::<Voz>().mandar(app, &ajustes, Orden::Configurar(ajustes.clone()));
}

/// Al guardar Ajustes: que el tilde del menú coincida.
pub fn marcar_tilde(app: &AppHandle, activa: bool) {
    if let Some(b) = app.try_state::<Bandeja>() {
        let _ = b.palabra.set_checked(activa);
    }
}

/// Lo llama el motor de voz cuando empieza o deja de esperar "Oye Nova".
pub fn marcar_escucha(app: &AppHandle, escuchando: bool) {
    let Some(b) = app.try_state::<Bandeja>() else { return };
    let Some(bandeja) = app.tray_by_id(ID) else { return };
    let icono = if escuchando { &b.icono_escuchando } else { &b.icono };
    if let Some(i) = icono {
        let _ = bandeja.set_icon(Some(i.clone()));
    }
    let _ = bandeja.set_tooltip(Some(if escuchando { TEXTO_ESCUCHANDO } else { TEXTO }));
}

/// El mismo ícono con un punto claro abajo a la derecha.
fn con_puntito(icono: &Image<'_>) -> Image<'static> {
    let (ancho, alto) = (icono.width(), icono.height());
    let mut rgba = icono.rgba().to_vec();
    let radio = ancho.min(alto) as f32 * 0.2;
    let (cx, cy) = (ancho as f32 - radio - 1.0, alto as f32 - radio - 1.0);
    for y in 0..alto {
        for x in 0..ancho {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let i = ((y * ancho + x) * 4) as usize;
            if d <= radio {
                // Borde oscuro para que se vea sobre cualquier ícono.
                let color: [u8; 4] = if d > radio - 1.5 { [20, 24, 40, 255] } else { [140, 230, 200, 255] };
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }
    }
    Image::new_owned(rgba, ancho, alto)
}
