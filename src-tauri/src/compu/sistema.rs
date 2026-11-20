//! Lo que le pide a Windows directamente (v0.7.1): las carpetas conocidas
//! (Documentos, Descargas…), abrir cosas, mandar a la Papelera y las teclas
//! de medios.
//!
//! Son llamadas a funciones de Windows con `windows-sys`. Todas son
//! `unsafe` porque Rust no puede verificar lo que hace código de C: los
//! comentarios `SEGURIDAD:` explican por qué cada uso está bien.

use std::path::{Path, PathBuf};

use crate::dominio::compu::{CarpetasDelSistema, sin_prefijo_largo};

/// Windows pide los textos como UTF-16 terminados en un 0 (no como los
/// `String` de Rust, que son UTF-8 y saben su largo).
#[cfg(windows)]
fn ancho(texto: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    texto.encode_wide().chain(std::iter::once(0)).collect()
}

/// Una carpeta conocida de Windows. Se pregunta a Windows (en vez de armar
/// `C:\Users\vos\Documents`) porque OneDrive o el usuario pueden haberla
/// movido a otro lado.
#[cfg(windows)]
fn carpeta_conocida(id: &windows_sys::core::GUID) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::SHGetKnownFolderPath;

    let mut ruta: windows_sys::core::PWSTR = std::ptr::null_mut();
    // SEGURIDAD: `id` apunta a un GUID válido y `ruta` es donde Windows deja
    // un texto que reserva él; lo liberamos con `CoTaskMemFree` como pide.
    let resultado = unsafe { SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut ruta) };
    if resultado != 0 || ruta.is_null() {
        return None;
    }
    // SEGURIDAD: Windows garantiza que el texto termina en 0: contamos hasta ahí.
    let texto = unsafe {
        let mut largo = 0;
        while *ruta.add(largo) != 0 {
            largo += 1;
        }
        std::ffi::OsString::from_wide(std::slice::from_raw_parts(ruta, largo))
    };
    unsafe { CoTaskMemFree(ruta as *const _) };
    Some(PathBuf::from(texto))
}

/// Documentos, Escritorio y Descargas: las carpetas permitidas por defecto.
pub fn carpetas_por_defecto() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads};
        [&FOLDERID_Documents, &FOLDERID_Desktop, &FOLDERID_Downloads].into_iter().filter_map(carpeta_conocida).collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Las carpetas que nunca se pueden permitir, ya resueltas.
pub fn carpetas_del_sistema() -> CarpetasDelSistema {
    let resolver = |p: PathBuf| p.canonicalize().map(|c| sin_prefijo_largo(&c)).unwrap_or(p);
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{
            FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramData, FOLDERID_ProgramFiles,
            FOLDERID_ProgramFilesX86, FOLDERID_RoamingAppData, FOLDERID_Windows,
        };
        let prohibidas = [
            &FOLDERID_Windows,
            &FOLDERID_ProgramFiles,
            &FOLDERID_ProgramFilesX86,
            &FOLDERID_ProgramData,
            &FOLDERID_RoamingAppData,
            &FOLDERID_LocalAppData,
        ]
        .into_iter()
        .filter_map(carpeta_conocida)
        .map(resolver)
        .collect::<Vec<_>>();
        // AppData entera (también LocalLow), no solo Roaming y Local.
        let perfil = carpeta_conocida(&FOLDERID_Profile).map(resolver);
        let mut prohibidas = prohibidas;
        if let Some(p) = &perfil {
            prohibidas.push(p.join("AppData"));
        }
        CarpetasDelSistema { prohibidas, perfil }
    }
    #[cfg(not(windows))]
    {
        let _ = resolver;
        CarpetasDelSistema::default()
    }
}

/// El nombre en castellano de una carpeta por defecto ("Downloads" en el
/// disco es "Descargas" en el Explorador).
pub fn nombre_para_mostrar(carpeta: &Path) -> String {
    let nombre = carpeta.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match nombre.to_lowercase().as_str() {
        "documents" => "Documentos".into(),
        "desktop" => "Escritorio".into(),
        "downloads" => "Descargas".into(),
        "pictures" => "Imágenes".into(),
        "music" => "Música".into(),
        "videos" => "Videos".into(),
        _ => nombre,
    }
}

/// Abre algo con `ShellExecuteW`, lo mismo que hace el Explorador con un
/// doble clic: una dirección en el navegador por defecto, un archivo con su
/// programa, una carpeta en el Explorador o un acceso directo.
///
/// `argumento` (opcional) es UNA ruta que se le pasa al programa, entre
/// comillas. En Windows una ruta no puede tener comillas, así que no hay forma
/// de que "se escape" y agregue otros parámetros.
#[cfg(windows)]
pub fn abrir(objetivo: &std::ffi::OsStr, argumento: Option<&Path>) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let verbo = ancho(std::ffi::OsStr::new("open"));
    let objetivo = ancho(objetivo);
    let parametros = match argumento {
        Some(ruta) => {
            let texto = sin_prefijo_largo(ruta).to_string_lossy().to_string();
            if texto.contains('"') {
                return Err("Esa ruta no se puede pasar.".into());
            }
            Some(ancho(std::ffi::OsStr::new(&format!("\"{texto}\""))))
        }
        None => None,
    };
    // SEGURIDAD: todos los textos son UTF-16 terminados en 0 y viven hasta
    // que termina la llamada. 1 = SW_SHOWNORMAL (mostrar la ventana).
    let resultado = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verbo.as_ptr(),
            objetivo.as_ptr(),
            parametros.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            std::ptr::null(),
            1,
        )
    };
    // Devuelve un número "disfrazado" de puntero: más de 32 es que anduvo.
    if resultado as isize > 32 { Ok(()) } else { Err(format!("Windows no lo pudo abrir (código {}).", resultado as isize)) }
}

#[cfg(not(windows))]
pub fn abrir(_objetivo: &std::ffi::OsStr, _argumento: Option<&Path>) -> Result<(), String> {
    Err("Esto solo anda en Windows.".into())
}

/// Manda un archivo a la Papelera (se puede recuperar desde ahí).
#[cfg(windows)]
pub fn a_la_papelera(ruta: &Path) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW, SHFileOperationW,
    };
    // Esta función pide la lista de archivos terminada en DOS ceros.
    let mut desde = ancho(sin_prefijo_largo(ruta).as_os_str());
    desde.push(0);
    let mut operacion = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE,
        pFrom: desde.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };
    // SEGURIDAD: la estructura está completa y `desde` vive durante la llamada.
    let resultado = unsafe { SHFileOperationW(&mut operacion) };
    if resultado == 0 && operacion.fAnyOperationsAborted == 0 {
        Ok(())
    } else {
        Err(format!("No lo pude mandar a la Papelera (código {resultado})."))
    }
}

#[cfg(not(windows))]
pub fn a_la_papelera(ruta: &Path) -> Result<(), String> {
    std::fs::remove_file(ruta).map_err(|e| e.to_string())
}

/// Las teclas de medios del teclado. Las entienden Spotify, YouTube en el
/// navegador y casi cualquier reproductor, y el volumen es el de Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeclaMedios {
    PausarOSeguir,
    Siguiente,
    Anterior,
    SubirVolumen,
    BajarVolumen,
    Silenciar,
}

#[cfg(windows)]
pub fn apretar(tecla: TeclaMedios, veces: u8) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_MEDIA_NEXT_TRACK,
        VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    };
    let codigo = match tecla {
        TeclaMedios::PausarOSeguir => VK_MEDIA_PLAY_PAUSE,
        TeclaMedios::Siguiente => VK_MEDIA_NEXT_TRACK,
        TeclaMedios::Anterior => VK_MEDIA_PREV_TRACK,
        TeclaMedios::SubirVolumen => VK_VOLUME_UP,
        TeclaMedios::BajarVolumen => VK_VOLUME_DOWN,
        TeclaMedios::Silenciar => VK_VOLUME_MUTE,
    };
    let tecla = |soltar: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: codigo,
                wScan: 0,
                dwFlags: if soltar { KEYEVENTF_KEYUP } else { 0 as KEYBD_EVENT_FLAGS },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    // Cada toque de volumen sube o baja 2 puntos: "subí el volumen" son 5.
    let entradas: Vec<INPUT> = (0..veces.max(1)).flat_map(|_| [tecla(false), tecla(true)]).collect();
    // SEGURIDAD: `entradas` es un arreglo de INPUT bien armados y le decimos
    // su largo y el tamaño de cada uno, como pide la función.
    let enviadas = unsafe { SendInput(entradas.len() as u32, entradas.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if enviadas as usize == entradas.len() { Ok(()) } else { Err("Windows no aceptó la tecla.".into()) }
}

#[cfg(not(windows))]
pub fn apretar(_tecla: TeclaMedios, _veces: u8) -> Result<(), String> {
    Err("Esto solo anda en Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn encuentra_las_carpetas_de_windows() {
        let carpetas = carpetas_por_defecto();
        assert_eq!(carpetas.len(), 3, "{carpetas:?}");
        assert!(carpetas.iter().all(|c| c.is_dir()));
        let sistema = carpetas_del_sistema();
        assert!(sistema.prohibidas.iter().any(|p| p.to_string_lossy().to_lowercase().ends_with("windows")));
        assert!(sistema.perfil.is_some());
        // Las de por defecto no están prohibidas.
        for c in &carpetas {
            let resuelta = sin_prefijo_largo(&c.canonicalize().unwrap());
            assert!(crate::dominio::compu::carpeta_prohibida(&resuelta, &sistema).is_none(), "{resuelta:?}");
        }
    }

    #[test]
    fn nombres_en_castellano() {
        assert_eq!(nombre_para_mostrar(Path::new(r"C:\Users\A\Downloads")), "Descargas");
        assert_eq!(nombre_para_mostrar(Path::new(r"D:\Facu")), "Facu");
    }

    /// Abre la calculadora de verdad.
    #[test]
    #[ignore]
    fn abre_la_calculadora() {
        abrir(std::ffi::OsStr::new("calc.exe"), None).unwrap();
    }
}
