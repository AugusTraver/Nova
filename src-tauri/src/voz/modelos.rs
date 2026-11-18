//! Los programas y modelos de voz: dónde van y cómo se bajan.
//!
//! No van en el repo ni en el instalador (son 1,4 GB): se bajan la primera
//! vez a la carpeta de datos local de la app (`%LOCALAPPDATA%\com.augustraver.nova\voz`).
//!
//! - whisper.cpp (voz → texto), en versión con CUDA para placas NVIDIA y en
//!   versión solo CPU como respaldo.
//! - El modelo large-v3-turbo de Whisper, cuantizado (574 MB).
//! - Piper (texto → voz) y la voz `es_AR-daniela-high`.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Resultado};

const WHISPER_CUDA: &str = "https://github.com/ggml-org/whisper.cpp/releases/download/b5454/whisper-bin-win-cuda-12.4.0-x64.zip";
const WHISPER_CPU: &str = "https://github.com/ggml-org/whisper.cpp/releases/download/b5454/whisper-bin-x64.zip";
const MODELO_WHISPER: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin";
const PIPER: &str = "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip";
const VOZ: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_AR/daniela/high/es_AR-daniela-high.onnx";

/// Las rutas de todo lo de voz, a partir de la carpeta base.
#[derive(Debug, Clone)]
pub struct Rutas {
    pub base: PathBuf,
}

impl Rutas {
    pub fn new(base: PathBuf) -> Self {
        Rutas { base }
    }
    pub fn servidor_whisper(&self, gpu: bool) -> PathBuf {
        self.base.join(if gpu { "whisper-cuda" } else { "whisper-cpu" }).join("whisper-server.exe")
    }
    pub fn modelo_whisper(&self) -> PathBuf {
        self.base.join("modelos").join("ggml-large-v3-turbo-q5_0.bin")
    }
    pub fn piper(&self) -> PathBuf {
        self.base.join("piper").join("piper.exe")
    }
    pub fn voz(&self) -> PathBuf {
        self.base.join("modelos").join("es_AR-daniela-high.onnx")
    }
    /// Donde se anota el número de proceso de Whisper o Piper.
    pub fn archivo_pid(&self, programa: &str) -> PathBuf {
        self.base.join(format!("{programa}.pid"))
    }
    /// Donde Piper deja cada audio que sintetiza.
    pub fn temporales(&self) -> PathBuf {
        self.base.join("temporales")
    }
}

/// Una descarga: qué, de dónde y adónde. Los .zip se descomprimen en una
/// carpeta (sacando la carpeta de adentro del zip: "Release/", "piper/").
#[derive(Debug, Clone, PartialEq)]
pub struct Descarga {
    pub nombre: &'static str,
    pub url: &'static str,
    /// Tamaño aproximado en MB, para la barra de progreso antes de saberlo.
    pub megas: u64,
    pub destino: PathBuf,
    /// `Some(carpeta)` si es un zip a descomprimir ahí.
    pub descomprimir_en: Option<PathBuf>,
    /// El archivo que tiene que existir al final para darla por hecha.
    pub listo_si_existe: PathBuf,
}

/// Lo que falta bajar. `gpu`: si hay placa NVIDIA, también la versión con CUDA.
pub fn faltantes(r: &Rutas, gpu: bool) -> Vec<Descarga> {
    let zip = |nombre: &str| r.base.join("temporales").join(nombre);
    let mut todas = vec![
        Descarga {
            nombre: "Whisper (programa)",
            url: WHISPER_CPU,
            megas: 9,
            destino: zip("whisper-cpu.zip"),
            descomprimir_en: Some(r.base.join("whisper-cpu")),
            listo_si_existe: r.servidor_whisper(false),
        },
        Descarga {
            nombre: "Whisper (modelo)",
            url: MODELO_WHISPER,
            megas: 548,
            destino: r.modelo_whisper(),
            descomprimir_en: None,
            listo_si_existe: r.modelo_whisper(),
        },
        Descarga {
            nombre: "Piper (programa)",
            url: PIPER,
            megas: 21,
            destino: zip("piper.zip"),
            descomprimir_en: Some(r.base.join("piper")),
            listo_si_existe: r.piper(),
        },
        Descarga {
            nombre: "Voz en español",
            url: VOZ,
            megas: 109,
            destino: r.voz(),
            descomprimir_en: None,
            listo_si_existe: r.voz(),
        },
        Descarga {
            nombre: "Voz en español (ajustes)",
            url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_AR/daniela/high/es_AR-daniela-high.onnx.json",
            megas: 1,
            destino: r.voz().with_extension("onnx.json"),
            descomprimir_en: None,
            listo_si_existe: r.voz().with_extension("onnx.json"),
        },
    ];
    if gpu {
        todas.insert(
            1,
            Descarga {
                nombre: "Whisper para la placa de video",
                url: WHISPER_CUDA,
                megas: 653,
                destino: zip("whisper-cuda.zip"),
                descomprimir_en: Some(r.base.join("whisper-cuda")),
                listo_si_existe: r.servidor_whisper(true),
            },
        );
    }
    todas.into_iter().filter(|d| !d.listo_si_existe.exists()).collect()
}

/// ¿Hay una placa NVIDIA? (`nvidia-smi` viene con el driver.)
pub fn hay_gpu_nvidia() -> bool {
    let mut comando = std::process::Command::new("nvidia-smi");
    comando.arg("-L");
    sin_ventana(&mut comando);
    comando.output().map(|s| s.status.success() && !s.stdout.is_empty()).unwrap_or(false)
}

/// En Windows, que el programa no abra una consola negra.
pub fn sin_ventana(comando: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = comando;
}

/// Anota el número de proceso (para cerrarlo si Nova se cae).
pub fn anotar_pid(archivo: &Path, pid: u32) {
    let _ = std::fs::write(archivo, pid.to_string());
}

/// Cierra un proceso que quedó de una vez anterior, solo si de verdad es
/// ese programa (el número pudo haberlo reusado otro).
pub fn cerrar_huerfano(archivo: &Path, programa: &str) {
    let Ok(texto) = std::fs::read_to_string(archivo) else { return };
    let _ = std::fs::remove_file(archivo);
    let Ok(pid) = texto.trim().parse::<u32>() else { return };
    let mut comando = std::process::Command::new("taskkill");
    comando.args(["/F", "/PID", &pid.to_string(), "/FI", &format!("IMAGENAME eq {programa}")]);
    sin_ventana(&mut comando);
    let _ = comando.output();
}

/// Progreso de la descarga, para Ajustes (evento `voz:descarga`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgresoDescarga {
    pub nombre: String,
    /// De todas las descargas juntas.
    pub megas_bajados: u64,
    pub megas_totales: u64,
    pub terminado: bool,
    pub error: Option<String>,
}

/// Baja un archivo a `destino` de a pedazos, avisando cada tanto cuántos
/// bytes van. Primero escribe en `.parcial` y al final lo renombra: si se
/// corta a la mitad, no queda un archivo roto que parezca completo.
pub async fn bajar(
    http: &reqwest::Client,
    d: &Descarga,
    mut al_avanzar: impl FnMut(u64),
) -> Resultado<()> {
    let falla = |e: &dyn std::fmt::Display| Error::Sistema(format!("No pude bajar {}: {e}", d.nombre));
    if let Some(carpeta) = d.destino.parent() {
        std::fs::create_dir_all(carpeta).map_err(|e| falla(&e))?;
    }
    let parcial = d.destino.with_extension("parcial");
    let mut respuesta = http.get(d.url).send().await.map_err(|e| falla(&e))?;
    if !respuesta.status().is_success() {
        return Err(falla(&format!("el servidor respondió {}", respuesta.status().as_u16())));
    }
    let mut archivo = std::fs::File::create(&parcial).map_err(|e| falla(&e))?;
    let mut bajados: u64 = 0;
    // `chunk()` va trayendo el cuerpo de a pedazos, sin cargarlo entero en memoria.
    while let Some(pedazo) = respuesta.chunk().await.map_err(|e| falla(&e))? {
        archivo.write_all(&pedazo).map_err(|e| falla(&e))?;
        bajados += pedazo.len() as u64;
        al_avanzar(bajados);
    }
    archivo.flush().map_err(|e| falla(&e))?;
    drop(archivo);
    std::fs::rename(&parcial, &d.destino).map_err(|e| falla(&e))?;
    if let Some(carpeta) = &d.descomprimir_en {
        descomprimir(&d.destino, carpeta).map_err(|e| falla(&e))?;
        let _ = std::fs::remove_file(&d.destino);
    }
    Ok(())
}

/// Descomprime un zip en `carpeta`, salteando la primera carpeta de adentro
/// ("Release/whisper-server.exe" queda como "carpeta/whisper-server.exe").
/// Rechaza rutas que intenten salirse de la carpeta ("../../algo").
pub fn descomprimir(zip: &Path, carpeta: &Path) -> Result<(), String> {
    let archivo = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut lector = zip::ZipArchive::new(archivo).map_err(|e| e.to_string())?;
    for i in 0..lector.len() {
        let mut entrada = lector.by_index(i).map_err(|e| e.to_string())?;
        // `enclosed_name` devuelve None si la ruta es peligrosa.
        let Some(ruta) = entrada.enclosed_name() else { continue };
        let Some(relativa) = sin_primera_carpeta(&ruta) else { continue };
        let final_ = carpeta.join(relativa);
        if entrada.is_dir() {
            std::fs::create_dir_all(&final_).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(padre) = final_.parent() {
            std::fs::create_dir_all(padre).map_err(|e| e.to_string())?;
        }
        let mut salida = std::fs::File::create(&final_).map_err(|e| e.to_string())?;
        std::io::copy(&mut entrada, &mut salida).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// "Release/x/y.dll" → "x/y.dll"; "suelto.txt" → "suelto.txt".
fn sin_primera_carpeta(ruta: &Path) -> Option<PathBuf> {
    let partes: Vec<_> = ruta.components().collect();
    match partes.len() {
        0 => None,
        1 => Some(PathBuf::from(partes[0].as_os_str())),
        _ => Some(partes[1..].iter().collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rutas_y_faltantes() {
        let base = std::env::temp_dir().join("nova-test-voz-que-no-existe");
        let r = Rutas::new(base.clone());
        assert!(r.servidor_whisper(true).ends_with("whisper-cuda/whisper-server.exe"));
        let sin_gpu = faltantes(&r, false);
        let con_gpu = faltantes(&r, true);
        assert_eq!(sin_gpu.len(), 5);
        assert_eq!(con_gpu.len(), 6);
        assert!(con_gpu.iter().any(|d| d.url.contains("cuda")));
        // Total aproximado que se le muestra al usuario.
        let total: u64 = con_gpu.iter().map(|d| d.megas).sum();
        assert!((1300..1400).contains(&total), "{total}");
    }

    #[test]
    fn saca_la_carpeta_de_adentro_del_zip() {
        assert_eq!(sin_primera_carpeta(Path::new("Release/whisper.dll")), Some(PathBuf::from("whisper.dll")));
        assert_eq!(
            sin_primera_carpeta(Path::new("piper/espeak-ng-data/es_dict")),
            Some(PathBuf::from("espeak-ng-data/es_dict"))
        );
        assert_eq!(sin_primera_carpeta(Path::new("suelto.txt")), Some(PathBuf::from("suelto.txt")));
    }

    #[test]
    fn descomprime_un_zip() {
        use zip::write::SimpleFileOptions;
        let carpeta = std::env::temp_dir().join(format!("nova-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&carpeta);
        std::fs::create_dir_all(&carpeta).unwrap();
        let ruta_zip = carpeta.join("prueba.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&ruta_zip).unwrap());
            z.start_file("Release/programa.exe", SimpleFileOptions::default()).unwrap();
            z.write_all(b"hola").unwrap();
            z.start_file("Release/datos/x.txt", SimpleFileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        let destino = carpeta.join("sale");
        descomprimir(&ruta_zip, &destino).unwrap();
        assert_eq!(std::fs::read(destino.join("programa.exe")).unwrap(), b"hola");
        assert!(destino.join("datos").join("x.txt").exists());
        let _ = std::fs::remove_dir_all(&carpeta);
    }
}
