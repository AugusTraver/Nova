//! Voz (v0.5, SPEC 5.1): Nova escucha sola al abrir el panel y contesta
//! hablando. Todo local: el audio nunca sale de la compu.
//!
//! - `audio`, `vad`, `texto`: cuentas puras con tests (remuestrear, detectar
//!   cuándo hablás, limpiar lo que entiende Whisper, partir en oraciones).
//! - `dispositivos`: micrófono y parlantes (cpal).
//! - `whisper`, `piper`: los programas de voz → texto y texto → voz.
//! - `modelos`: dónde están y cómo se bajan.
//! - `motor`: el hilo que junta todo.

pub mod audio;
pub mod dispositivos;
pub mod modelos;
pub mod motor;
pub mod piper;
pub mod texto;
pub mod vad;
pub mod whisper;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use modelos::Rutas;
use motor::{Motor, Orden};

/// Lo que se guarda en preferencias (clave `voz`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesVoz {
    /// Escuchar al abrir el panel y contestar hablando.
    pub activada: bool,
    /// 1 = normal; de 0.7 a 1.4.
    pub velocidad: f32,
    /// Leer el resumen del día la primera vez que se abre el panel en el día.
    pub leer_resumen: bool,
    /// Usar la placa de video para Whisper (si hay una NVIDIA).
    pub usar_gpu: bool,
}

impl Default for AjustesVoz {
    fn default() -> Self {
        AjustesVoz { activada: true, velocidad: 1.0, leer_resumen: true, usar_gpu: true }
    }
}

impl AjustesVoz {
    pub fn validar(self) -> Result<Self, String> {
        if !(0.7..=1.4).contains(&self.velocidad) {
            return Err("La velocidad tiene que estar entre 0,7 y 1,4.".into());
        }
        Ok(self)
    }
}

/// El estado que guarda Tauri (`app.manage`): el canal para mandarle
/// órdenes al motor. El motor arranca la primera vez que se lo necesita.
pub struct Voz {
    canal: Mutex<Option<Sender<Orden>>>,
    pub rutas: Rutas,
}

impl Voz {
    pub fn new(rutas: Rutas) -> Self {
        // Si Nova se cerró de golpe la vez anterior, pueden haber quedado
        // Whisper o Piper abiertos: se cierran.
        modelos::cerrar_huerfano(&rutas.archivo_pid("whisper"), "whisper-server.exe");
        modelos::cerrar_huerfano(&rutas.archivo_pid("piper"), "piper.exe");
        Voz { canal: Mutex::new(None), rutas }
    }

    /// Al cerrar Nova: el motor cierra Whisper y Piper (espera hasta 2 s).
    pub fn apagar(&self) {
        let Ok(mut canal) = self.canal.lock() else { return };
        if let Some(tx) = canal.take() {
            let (listo, esperar) = channel();
            if tx.send(Orden::Salir(listo)).is_ok() {
                let _ = esperar.recv_timeout(std::time::Duration::from_secs(2));
            }
        }
    }

    /// Manda una orden; si el motor no existe, lo crea en un hilo nuevo.
    pub fn mandar(&self, app: &AppHandle, ajustes: &AjustesVoz, orden: Orden) {
        let Ok(mut canal) = self.canal.lock() else { return };
        if canal.is_none() {
            let (tx, rx) = channel();
            let motor = Motor::new(motor::avisador_de_tauri(app.clone()), self.rutas.clone(), ajustes.clone(), tx.clone());
            // Un hilo común (no async): el motor bloquea esperando audio y
            // hablando con los programas de voz.
            let creado = std::thread::Builder::new().name("nova-voz".into()).spawn(move || motor.correr(rx));
            if creado.is_err() {
                eprintln!("No pude arrancar el hilo de voz.");
                return;
            }
            *canal = Some(tx);
        }
        if let Some(tx) = canal.as_ref()
            && tx.send(orden).is_err()
        {
            // El motor se cayó: la próxima orden lo vuelve a crear.
            *canal = None;
        }
    }

    pub fn modelos_listos(&self) -> bool {
        modelos::faltantes(&self.rutas, false).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajustes_por_defecto_y_validacion() {
        let a = AjustesVoz::default();
        assert!(a.activada && a.leer_resumen && a.usar_gpu);
        assert!(AjustesVoz { velocidad: 2.0, ..a.clone() }.validar().is_err());
        let leidos: AjustesVoz = serde_json::from_str(r#"{"velocidad":1.2}"#).unwrap();
        assert_eq!(leidos.velocidad, 1.2);
        assert!(leidos.activada);
    }
}

/// Pruebas con los programas de verdad. Necesitan los modelos bajados, así
/// que no corren con `cargo test`: `cargo test voz::real -- --ignored`.
#[cfg(test)]
mod real {
    use super::audio::{FRECUENCIA_WHISPER, remuestrear, wav};
    use super::modelos::Rutas;
    use super::piper::Piper;
    use super::whisper::ServidorWhisper;

    fn rutas() -> Rutas {
        let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA");
        Rutas::new(std::path::PathBuf::from(local).join("com.augustraver.nova").join("voz"))
    }

    /// El motor entero: le llega "por el micrófono" (a 48 kHz, con ruido de
    /// habitación) un pedido hablado, y tiene que avisar el texto final.
    #[test]
    #[ignore]
    fn el_motor_escucha_un_pedido() {
        use super::motor::{EVENTO_ESTADO, EVENTO_FINAL, Motor, Orden};
        use super::dispositivos::DelMicrofono;
        use std::sync::{Arc, Mutex};

        let r = rutas();
        let (frecuencia, voz) = {
            let mut piper = Piper::abrir(&r, 1.0).unwrap();
            piper.sintetizar("Recordame comprar el regalo de Juan para el viernes.").unwrap()
        };
        let voz = remuestrear(&voz, frecuencia, 48_000);
        let ruido = |segundos: f32| -> Vec<f32> {
            (0..(48_000.0 * segundos) as usize).map(|i| ((i * 7919 % 1000) as f32 / 1000.0 - 0.5) * 0.004).collect()
        };
        let audio: Vec<f32> = [ruido(2.0), voz, ruido(2.0)].concat();

        let eventos: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
        let anotar = eventos.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let motor = Motor::new(
            Box::new(move |e, v| anotar.lock().unwrap().push((e.to_string(), v))),
            r,
            super::AjustesVoz::default(),
            tx.clone(),
        )
        .sin_microfono(48_000);
        let hilo = std::thread::spawn(move || motor.correr(rx));
        tx.send(Orden::Activar).unwrap();
        // De a 10 ms, como un micrófono.
        for tanda in audio.chunks(480) {
            tx.send(Orden::Microfono(DelMicrofono::Audio(tanda.to_vec()))).unwrap();
        }
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let final_ = loop {
            let encontrado = eventos.lock().unwrap().iter().find(|(e, _)| e == EVENTO_FINAL).map(|(_, v)| v.clone());
            if let Some(v) = encontrado {
                break v;
            }
            assert!(std::time::Instant::now() < limite, "no llegó el final");
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        let estados: Vec<String> = eventos
            .lock()
            .unwrap()
            .iter()
            .filter(|(e, _)| e == EVENTO_ESTADO)
            .map(|(_, v)| v["estado"].as_str().unwrap_or("").to_string())
            .collect();
        let (listo, esperar) = std::sync::mpsc::channel();
        tx.send(Orden::Salir(listo)).unwrap();
        esperar.recv().unwrap();
        hilo.join().unwrap();

        let texto = crate::dominio::lenguaje::normalizar(final_.as_str().unwrap());
        assert!(texto.contains("regalo de juan") && texto.contains("viernes"), "{texto}");
        // Pasó por todos los estados, en orden.
        let orden: Vec<&str> = ["preparando", "escuchando", "oyendo", "transcribiendo", "pensando"].to_vec();
        let mut i = 0;
        for e in &estados {
            if i < orden.len() && e == orden[i] {
                i += 1;
            }
        }
        assert_eq!(i, orden.len(), "{estados:?}");
    }

    #[test]
    #[ignore]
    fn piper_habla_y_whisper_entiende() {
        let r = rutas();
        let mut piper = Piper::abrir(&r, 1.0).unwrap();
        let (frecuencia, audio) = piper.sintetizar("Tengo parcial de Análisis el jueves a las nueve.").unwrap();
        assert!(audio.len() > frecuencia as usize);
        // Una segunda frase con el mismo proceso (sin volver a cargar la voz).
        let inicio = std::time::Instant::now();
        piper.sintetizar("Dale.").unwrap();
        assert!(inicio.elapsed().as_millis() < 3000, "{:?}", inicio.elapsed());

        let a_16k = remuestrear(&audio, frecuencia, FRECUENCIA_WHISPER);
        if let Ok(destino) = std::env::var("NOVA_GUARDAR_WAV") {
            std::fs::write(destino, wav(&a_16k, FRECUENCIA_WHISPER)).unwrap();
        }
        let texto = tauri::async_runtime::block_on(async {
            let servidor = ServidorWhisper::arrancar(&r, true).await.unwrap();
            assert!(servidor.gpu);
            servidor.transcribir(wav(&a_16k, FRECUENCIA_WHISPER)).await.unwrap()
        });
        let normal = crate::dominio::lenguaje::normalizar(&texto);
        // Piper cambia un poco cada vez que habla y a veces Whisper le
        // erra a una palabra: se piden las importantes.
        assert!(normal.contains("analisis") && normal.contains("jueves"), "{texto}");
    }
}

/// El micrófono y los parlantes de verdad (`cargo test voz::dispositivos_reales -- --ignored`).
/// El parlante suena en silencio: solo se comprueba que avance.
#[cfg(test)]
mod dispositivos_reales {
    use super::dispositivos::{DelMicrofono, Microfono, Parlante};
    use std::time::Duration;

    #[test]
    #[ignore]
    fn el_microfono_entrega_audio() {
        let (tx, rx) = std::sync::mpsc::channel::<DelMicrofono>();
        let mic = Microfono::abrir(tx, |m| m).unwrap();
        assert!(mic.frecuencia >= 16_000);
        let mut muestras = 0;
        let limite = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < limite {
            if let Ok(DelMicrofono::Audio(a)) = rx.recv_timeout(Duration::from_millis(100)) {
                muestras += a.len();
            }
        }
        // En 2 s tienen que haber llegado al menos 1,5 s de audio.
        assert!(muestras as f32 > mic.frecuencia as f32 * 1.5, "{muestras}");
    }

    #[test]
    #[ignore]
    fn el_parlante_avanza() {
        let p = Parlante::abrir().unwrap();
        p.encolar(&vec![0.0; p.frecuencia as usize / 2]);
        std::thread::sleep(Duration::from_millis(900));
        assert!(p.sonadas() >= u64::from(p.frecuencia) / 2 - 2000, "{}", p.sonadas());
        assert_eq!(p.pendientes(), 0);
    }
}
