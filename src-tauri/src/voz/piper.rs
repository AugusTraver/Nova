//! Piper (texto → voz) como un programa aparte que queda abierto: carga la
//! voz una vez (tarda un segundo) y después sintetiza cada frase en una
//! fracción de segundo.
//!
//! Se le habla por la entrada estándar con una línea JSON por frase
//! (`{"text": "..."}`); Piper escribe un WAV en `temporales/` y responde con
//! su ruta en una línea. Esto es bloqueante: se usa desde el hilo que habla.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use super::audio::leer_wav;
use super::modelos::{Rutas, sin_ventana};
use crate::error::{Error, Resultado};

pub struct Piper {
    proceso: Child,
    entrada: ChildStdin,
    salida: BufReader<ChildStdout>,
}

impl Drop for Piper {
    fn drop(&mut self) {
        let _ = self.proceso.kill();
        let _ = self.proceso.wait();
    }
}

impl Piper {
    /// `velocidad`: 1 = normal, 1.2 = más rápido. Piper lo toma al revés
    /// (`length_scale`: cuánto dura cada sonido).
    pub fn abrir(rutas: &Rutas, velocidad: f32) -> Resultado<Self> {
        let temporales = rutas.temporales();
        std::fs::create_dir_all(&temporales).map_err(|e| Error::Sistema(e.to_string()))?;
        let largo = (1.0 / velocidad.clamp(0.6, 1.6)).to_string();
        let mut comando = Command::new(rutas.piper());
        comando
            .arg("-m")
            .arg(rutas.voz())
            .arg("--json-input")
            .arg("-q")
            .arg("-d")
            .arg(&temporales)
            .args(["--length_scale", &largo, "--sentence_silence", "0.15"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(carpeta) = rutas.piper().parent() {
            comando.current_dir(carpeta);
        }
        sin_ventana(&mut comando);
        let mut proceso = comando.spawn().map_err(|e| Error::Sistema(format!("No pude abrir Piper: {e}")))?;
        super::modelos::anotar_pid(&rutas.archivo_pid("piper"), proceso.id());
        // `take()` saca los tubos del proceso para usarlos por separado.
        let entrada = proceso.stdin.take().ok_or_else(|| Error::Sistema("Piper sin entrada.".into()))?;
        let salida = BufReader::new(proceso.stdout.take().ok_or_else(|| Error::Sistema("Piper sin salida.".into()))?);
        Ok(Piper { proceso, entrada, salida })
    }

    /// Sintetiza una frase. Devuelve la frecuencia y el audio en mono.
    pub fn sintetizar(&mut self, texto: &str) -> Resultado<(u32, Vec<f32>)> {
        let falla = |e: &dyn std::fmt::Display| Error::Sistema(format!("Piper falló: {e}"));
        let linea = serde_json::json!({ "text": texto }).to_string();
        writeln!(self.entrada, "{linea}").map_err(|e| falla(&e))?;
        self.entrada.flush().map_err(|e| falla(&e))?;
        let mut ruta = String::new();
        let leidos = self.salida.read_line(&mut ruta).map_err(|e| falla(&e))?;
        if leidos == 0 {
            return Err(falla(&"se cerró"));
        }
        let ruta = ruta.trim();
        let bytes = std::fs::read(ruta).map_err(|e| falla(&e))?;
        let _ = std::fs::remove_file(ruta);
        leer_wav(&bytes).map_err(|e| falla(&e))
    }

    pub fn vivo(&mut self) -> bool {
        matches!(self.proceso.try_wait(), Ok(None))
    }
}
