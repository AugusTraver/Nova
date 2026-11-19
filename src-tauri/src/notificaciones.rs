//! Las notificaciones de Windows (v0.6). Tocar una abre el panel.
//!
//! Windows solo muestra avisos de programas que tienen un "AppUserModelID"
//! registrado, y eso lo hace el instalador. Mientras se desarrolla (`tauri
//! dev`) Nova no está instalada, así que se usa el de PowerShell: el aviso
//! sale igual, pero dice "Windows PowerShell" arriba.

use tauri::AppHandle;

use crate::dominio::avisos::Aviso;

#[cfg(windows)]
pub fn mostrar(app: &AppHandle, aviso: &Aviso) {
    use tauri_winrt_notification::{Sound, Toast};

    let id = if cfg!(debug_assertions) { Toast::POWERSHELL_APP_ID.to_string() } else { app.config().identifier.clone() };
    let abrir = app.clone();
    let resultado = Toast::new(&id)
        .title(&aviso.titulo)
        .text1(&aviso.cuerpo)
        .sound(Some(Sound::Default))
        .on_activated(move |_| {
            crate::ventana::mostrar(&abrir);
            Ok(())
        })
        .show();
    if let Err(e) = resultado {
        eprintln!("No pude mostrar el aviso «{}»: {e}", aviso.titulo);
    }
}

#[cfg(not(windows))]
pub fn mostrar(_app: &AppHandle, aviso: &Aviso) {
    eprintln!("Aviso: {} — {}", aviso.titulo, aviso.cuerpo);
}
