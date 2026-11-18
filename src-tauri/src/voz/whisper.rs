//! Whisper (voz → texto) como un programa aparte: `whisper-server.exe` de
//! whisper.cpp, escuchando solo en esta compu (127.0.0.1) en un puerto libre.
//!
//! Se arranca la primera vez que se abre el panel y queda cargado (el modelo
//! tarda un par de segundos en subir a la placa de video). Si nadie habla en
//! un rato, se cierra para liberar memoria (`Escucha` lo decide).
//!
//! Primero se prueba la versión con CUDA (placa NVIDIA); si no arranca, la de
//! CPU, que anda en cualquier compu pero tarda más.

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::modelos::{Rutas, sin_ventana};
use crate::error::{Error, Resultado};

/// Palabras que ayudan a Whisper a escribir bien nombres propios y el voseo.
const CONTEXTO: &str = "Nova, recordame, anotá, agendá, mañana, pasado mañana, parcial, Análisis, Boca, la Selección.";

pub struct ServidorWhisper {
    proceso: Child,
    puerto: u16,
    pub gpu: bool,
    http: reqwest::Client,
}

impl Drop for ServidorWhisper {
    /// Al soltarlo (cerrar Nova o apagar la voz), se cierra el programa.
    fn drop(&mut self) {
        let _ = self.proceso.kill();
        let _ = self.proceso.wait();
    }
}

/// Un puerto libre: se lo pide al sistema abriendo uno cualquiera (el 0) y
/// cerrándolo enseguida.
fn puerto_libre() -> Resultado<u16> {
    let socket = TcpListener::bind("127.0.0.1:0").map_err(|e| Error::Sistema(format!("Sin puerto libre: {e}")))?;
    Ok(socket.local_addr().map_err(|e| Error::Sistema(e.to_string()))?.port())
}

impl ServidorWhisper {
    /// Arranca el servidor y espera a que responda (hasta 40 s: la primera
    /// vez Windows puede tardar en leer 500 MB de modelo).
    pub async fn arrancar(rutas: &Rutas, preferir_gpu: bool) -> Resultado<Self> {
        let intentos: &[bool] = if preferir_gpu { &[true, false] } else { &[false] };
        let mut ultimo_error = Error::Sistema("No encontré Whisper.".into());
        for &gpu in intentos {
            let programa = rutas.servidor_whisper(gpu);
            if !programa.exists() {
                continue;
            }
            match Self::intentar(rutas, gpu).await {
                Ok(s) => return Ok(s),
                Err(e) => {
                    eprintln!("Whisper ({}) no arrancó: {e}", if gpu { "GPU" } else { "CPU" });
                    ultimo_error = e;
                }
            }
        }
        Err(ultimo_error)
    }

    async fn intentar(rutas: &Rutas, gpu: bool) -> Resultado<Self> {
        let puerto = puerto_libre()?;
        let hilos = std::thread::available_parallelism().map(|n| n.get().clamp(2, 8)).unwrap_or(4);
        let mut comando = Command::new(rutas.servidor_whisper(gpu));
        comando
            .arg("-m")
            .arg(rutas.modelo_whisper())
            .args(["-l", "es", "-nt", "--host", "127.0.0.1", "--port", &puerto.to_string()])
            .args(["-t", &hilos.to_string()])
            // Sin esto, cada pedido carga el contexto anterior y a veces repite frases.
            .args(["-mc", "0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if !gpu {
            comando.arg("-ng");
        }
        // Las DLL (CUDA, ggml) están al lado del programa.
        if let Some(carpeta) = rutas.servidor_whisper(gpu).parent() {
            comando.current_dir(carpeta);
        }
        sin_ventana(&mut comando);
        let proceso = comando.spawn().map_err(|e| Error::Sistema(format!("No pude abrir Whisper: {e}")))?;
        super::modelos::anotar_pid(&rutas.archivo_pid("whisper"), proceso.id());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Sistema(e.to_string()))?;
        let mut servidor = ServidorWhisper { proceso, puerto, gpu, http };

        let limite = Instant::now() + Duration::from_secs(40);
        loop {
            if let Ok(Some(estado)) = servidor.proceso.try_wait() {
                return Err(Error::Sistema(format!("Whisper se cerró al arrancar ({estado}).")));
            }
            if servidor.http.get(format!("http://127.0.0.1:{puerto}/")).send().await.is_ok() {
                return Ok(servidor);
            }
            if Instant::now() > limite {
                return Err(Error::Sistema("Whisper tardó demasiado en arrancar.".into()));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// ¿Sigue vivo el programa?
    pub fn vivo(&mut self) -> bool {
        matches!(self.proceso.try_wait(), Ok(None))
    }

    /// Transcribe un WAV de 16 kHz mono. Devuelve el texto tal cual.
    pub async fn transcribir(&self, wav: Vec<u8>) -> Resultado<String> {
        let parte = reqwest::multipart::Part::bytes(wav)
            .file_name("pedido.wav")
            .mime_str("audio/wav")
            .map_err(|e| Error::Sistema(e.to_string()))?;
        let formulario = reqwest::multipart::Form::new()
            .part("file", parte)
            .text("response_format", "json")
            .text("temperature", "0")
            .text("prompt", CONTEXTO);
        let respuesta = self
            .http
            .post(format!("http://127.0.0.1:{}/inference", self.puerto))
            .multipart(formulario)
            .send()
            .await
            .map_err(|e| Error::Sistema(format!("Whisper no respondió: {e}")))?;
        let json: serde_json::Value =
            respuesta.json().await.map_err(|_| Error::Sistema("Whisper respondió algo que no entiendo.".into()))?;
        Ok(json.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string())
    }
}
