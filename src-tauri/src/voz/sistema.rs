//! Lo que pregunta el motor a Windows para pausar "Oye Nova": si la sesión
//! está bloqueada y si la compu anda a batería.

/// Con la sesión bloqueada (Win+L) el escritorio de entrada no es el tuyo y
/// Windows no deja abrirlo: así se sabe sin registrar ventanas ni avisos.
#[cfg(windows)]
pub fn sesion_bloqueada() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, DESKTOP_SWITCHDESKTOP, OpenInputDesktop};
    // SAFETY: llamadas simples de Windows; el escritorio que se abre se cierra enseguida.
    unsafe {
        let escritorio = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
        if escritorio.is_null() {
            return true;
        }
        CloseDesktop(escritorio);
        false
    }
}

/// ¿Está desenchufada? En una de escritorio siempre da que no.
#[cfg(windows)]
pub fn a_bateria() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    // SAFETY: Windows llena la estructura que le pasamos.
    unsafe {
        let mut estado: SYSTEM_POWER_STATUS = std::mem::zeroed();
        // 0 = a batería, 1 = enchufada, 255 = no se sabe.
        GetSystemPowerStatus(&mut estado) != 0 && estado.ACLineStatus == 0
    }
}

#[cfg(not(windows))]
pub fn sesion_bloqueada() -> bool {
    false
}

#[cfg(not(windows))]
pub fn a_bateria() -> bool {
    false
}
