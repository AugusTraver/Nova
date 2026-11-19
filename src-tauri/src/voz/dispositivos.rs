//! Micrófono y parlantes con `cpal` (en Windows, WASAPI).
//!
//! `cpal` llama a una función nuestra ("callback") desde un hilo de audio
//! de alta prioridad cada pocos milisegundos: ahí no se puede hacer nada
//! lento. Por eso el micrófono solo copia las muestras a un canal, y el
//! parlante solo saca muestras de una cola ya preparada.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::audio::a_mono;

/// Lo que manda el micrófono al motor.
pub enum DelMicrofono {
    /// Muestras en mono, a la frecuencia del micrófono.
    Audio(Vec<f32>),
    Error(String),
}

/// El micrófono abierto. Se cierra al soltarlo (`drop`).
pub struct Microfono {
    _flujo: Stream,
    pub frecuencia: u32,
}

const SIN_MICROFONO: &str =
    "No puedo usar el micrófono. Fijate que esté conectado y que Windows lo deje usar (Configuración > Privacidad > Micrófono).";

/// Los nombres de los micrófonos conectados (para elegir en Ajustes).
pub fn microfonos() -> Vec<String> {
    let Ok(dispositivos) = cpal::default_host().input_devices() else { return Vec::new() };
    let mut nombres: Vec<String> = dispositivos.filter_map(|d| d.description().ok().map(|x| x.name().to_string())).collect();
    nombres.sort();
    nombres.dedup();
    nombres
}

/// El micrófono con ese nombre; si está vacío o no está conectado, el que
/// Windows tenga por defecto. (Windows cambia el predeterminado solo, por
/// ejemplo al conectar un joystick con auriculares.)
fn elegir_microfono(host: &cpal::Host, nombre: &str) -> Option<cpal::Device> {
    if !nombre.is_empty()
        && let Ok(mut dispositivos) = host.input_devices()
        && let Some(d) = dispositivos.find(|d| d.description().is_ok_and(|x| x.name() == nombre))
    {
        return Some(d);
    }
    host.default_input_device()
}

impl Microfono {
    /// Abre el micrófono elegido (`nombre`, "" = el de Windows). `avisar` recibe cada tanda de audio.
    /// `envolver` convierte lo del micrófono en el mensaje del canal del
    /// motor (así el motor escucha un solo canal para todo).
    pub fn abrir<M: Send + 'static>(
        nombre: &str,
        avisar: Sender<M>,
        envolver: fn(DelMicrofono) -> M,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let dispositivo = elegir_microfono(&host, nombre).ok_or(SIN_MICROFONO)?;
        let soportada = dispositivo.default_input_config().map_err(|_| SIN_MICROFONO.to_string())?;
        let formato = soportada.sample_format();
        let config: StreamConfig = soportada.into();
        let canales = usize::from(config.channels);
        let frecuencia = config.sample_rate;

        let avisar_error = avisar.clone();
        let al_fallar = move |e: cpal::Error| {
            // Un "xrun" es un pedacito de audio que se perdió (la compu estuvo
            // ocupada): el micrófono sigue andando, no hay que cerrarlo.
            if e.kind() == cpal::ErrorKind::Xrun {
                return eprintln!("Micrófono: se perdió un pedacito de audio.");
            }
            let _ = avisar_error.send(envolver(DelMicrofono::Error(format!("El micrófono dejó de andar: {e}"))));
        };
        // `move` le pasa el canal al callback; `a_mono` junta los canales.
        let flujo = match formato {
            SampleFormat::F32 => dispositivo.build_input_stream(
                config,
                move |datos: &[f32], _: &_| {
                    let _ = avisar.send(envolver(DelMicrofono::Audio(a_mono(datos, canales))));
                },
                al_fallar,
                None,
            ),
            SampleFormat::I16 => dispositivo.build_input_stream(
                config,
                move |datos: &[i16], _: &_| {
                    let f: Vec<f32> = datos.iter().map(|m| f32::from(*m) / f32::from(i16::MAX)).collect();
                    let _ = avisar.send(envolver(DelMicrofono::Audio(a_mono(&f, canales))));
                },
                al_fallar,
                None,
            ),
            otro => return Err(format!("El micrófono usa un formato que no sé leer ({otro:?}).")),
        }
        .map_err(|e| format!("{SIN_MICROFONO} ({e})"))?;
        flujo.play().map_err(|e| format!("{SIN_MICROFONO} ({e})"))?;
        Ok(Microfono { _flujo: flujo, frecuencia })
    }
}

/// Los parlantes abiertos, con su cola de muestras (mono, a la frecuencia
/// del dispositivo). Se cierran al soltarlos.
pub struct Parlante {
    _flujo: Stream,
    pub frecuencia: u32,
    cola: Arc<Mutex<VecDeque<f32>>>,
    /// Cuántas muestras ya sonaron (para saber por dónde va y animar la esfera).
    sonadas: Arc<AtomicU64>,
}

impl Parlante {
    pub fn abrir() -> Result<Self, String> {
        let sin_parlante = "No encuentro por dónde sonar (¿parlantes o auriculares?).";
        let host = cpal::default_host();
        let dispositivo = host.default_output_device().ok_or(sin_parlante)?;
        let soportada = dispositivo.default_output_config().map_err(|_| sin_parlante.to_string())?;
        let formato = soportada.sample_format();
        let config: StreamConfig = soportada.into();
        let canales = usize::from(config.channels);
        let frecuencia = config.sample_rate;
        let cola: Arc<Mutex<VecDeque<f32>>> = Arc::default();
        let sonadas: Arc<AtomicU64> = Arc::default();

        // Llena cada cuadro (todos los canales con la misma muestra). Si la
        // cola se vacía, sale silencio.
        fn llenar<T: Copy>(salida: &mut [T], canales: usize, cola: &Mutex<VecDeque<f32>>, sonadas: &AtomicU64, convertir: fn(f32) -> T) {
            let Ok(mut cola) = cola.try_lock() else { return };
            let mut usadas = 0u64;
            for cuadro in salida.chunks_mut(canales) {
                let muestra = match cola.pop_front() {
                    Some(m) => {
                        usadas += 1;
                        m
                    }
                    None => 0.0,
                };
                for canal in cuadro.iter_mut() {
                    *canal = convertir(muestra);
                }
            }
            sonadas.fetch_add(usadas, Ordering::Relaxed);
        }

        let (c, s) = (cola.clone(), sonadas.clone());
        let al_fallar = |e: cpal::Error| eprintln!("Parlante: {e}");
        let flujo = match formato {
            SampleFormat::F32 => dispositivo.build_output_stream(
                config,
                move |datos: &mut [f32], _: &_| llenar(datos, canales, &c, &s, |m| m),
                al_fallar,
                None,
            ),
            SampleFormat::I16 => dispositivo.build_output_stream(
                config,
                move |datos: &mut [i16], _: &_| {
                    llenar(datos, canales, &c, &s, |m| (m.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
                },
                al_fallar,
                None,
            ),
            otro => return Err(format!("Los parlantes usan un formato que no sé escribir ({otro:?}).")),
        }
        .map_err(|e| format!("{sin_parlante} ({e})"))?;
        flujo.play().map_err(|e| format!("{sin_parlante} ({e})"))?;
        Ok(Parlante { _flujo: flujo, frecuencia, cola, sonadas })
    }

    /// Agrega audio al final (ya a la frecuencia del parlante).
    pub fn encolar(&self, muestras: &[f32]) {
        if let Ok(mut cola) = self.cola.lock() {
            cola.extend(muestras);
        }
    }

    pub fn pendientes(&self) -> usize {
        self.cola.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn sonadas(&self) -> u64 {
        self.sonadas.load(Ordering::Relaxed)
    }

    /// Corta en seco.
    pub fn vaciar(&self) {
        if let Ok(mut cola) = self.cola.lock() {
            cola.clear();
        }
    }
}
