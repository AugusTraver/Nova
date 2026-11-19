//! El motor de voz: un hilo propio que es dueño del micrófono, el detector
//! de voz, Whisper, Piper y los parlantes. Todo le llega por un solo canal
//! (órdenes de la interfaz y audio del micrófono), así que nunca hay dos
//! partes tocando lo mismo a la vez.
//!
//! El ciclo:
//!   escuchando → (hablás) oyendo → (te callás) transcribiendo → la interfaz
//!   le manda el texto al asistente → Nova responde hablando → escuchando.
//!
//! Mientras Nova habla, el micrófono se ignora: si no, se escucharía a sí
//! misma por los parlantes.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use super::audio::{FRECUENCIA_WHISPER, Remuestreador, envolvente, nivel_visual, remuestrear, rms, wav};
use super::dispositivos::{DelMicrofono, Microfono, Parlante};
use super::modelos::Rutas;
use super::piper::Piper;
use super::texto::{es_eco_del_contexto, limpiar_transcripcion, oraciones, para_leer};
use super::vad::{AjustesVad, EventoVad, MUESTRAS_POR_CUADRO, Vad};
use super::whisper::ServidorWhisper;
use super::AjustesVoz;

pub const EVENTO_ESTADO: &str = "voz:estado";
pub const EVENTO_NIVEL: &str = "voz:nivel";
pub const EVENTO_PARCIAL: &str = "voz:parcial";
pub const EVENTO_FINAL: &str = "voz:final";

/// Cada cuánto se avisa el nivel a la esfera (unas 30 veces por segundo).
const CADA_NIVEL: Duration = Duration::from_millis(33);
/// Cada cuánto se transcribe lo que vas diciendo (transcripción en vivo).
const CADA_PARCIAL: Duration = Duration::from_millis(700);
/// Whisper se cierra si nadie lo usa en este tiempo con el panel cerrado.
const WHISPER_OCIOSO: Duration = Duration::from_secs(10 * 60);
/// Después de hablar, un ratito sin escuchar (el eco del final).
const COLA_DEL_ECO: Duration = Duration::from_millis(350);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    /// Panel cerrado o voz apagada.
    Apagada,
    /// Arrancando Whisper.
    Preparando,
    Escuchando,
    /// Detectó que estás hablando.
    Oyendo,
    Transcribiendo,
    /// Esperando que el asistente conteste.
    Pensando,
    Hablando,
    /// Estás escribiendo: no escucha.
    Pausada,
    /// Faltan los modelos (se bajan desde Ajustes).
    SinModelos,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvisoEstado {
    pub estado: Estado,
    pub mensaje: Option<String>,
}

pub enum Orden {
    /// Se abrió el panel: escuchar (si la voz está activada).
    Activar,
    /// Se cerró el panel: soltar el micrófono y callarse.
    Desactivar,
    Pausar,
    Reanudar,
    /// El asistente está pensando: no escuchar hasta que conteste.
    Esperar,
    Hablar(String),
    Callar,
    Configurar(AjustesVoz),
    Microfono(DelMicrofono),
    /// Nova se cierra: cerrar los programas de voz y avisar por el canal.
    Salir(Sender<()>),
}

fn del_microfono(m: DelMicrofono) -> Orden {
    Orden::Microfono(m)
}

struct Habla {
    parlante: Parlante,
    /// El nivel de cada 1/30 s de lo encolado, para la esfera.
    niveles: Vec<f32>,
    /// Las frases que faltan sintetizar. Se sintetizan de a una, cuando a
    /// la cola del parlante le queda poco: así un "callate" llega enseguida.
    frases: VecDeque<String>,
}

/// Cómo avisa el motor a la interfaz: en la app, eventos de Tauri; en los
/// tests, una función que los anota.
pub type Avisador = Box<dyn Fn(&'static str, Value) + Send>;

pub fn avisador_de_tauri(app: AppHandle) -> Avisador {
    Box::new(move |evento, valor| {
        let _ = app.emit(evento, valor);
    })
}

pub struct Motor {
    avisar_evento: Avisador,
    /// En los tests no hay micrófono: el audio llega por `Orden::Microfono`
    /// a esta frecuencia.
    microfono_de_prueba: Option<u32>,
    rutas: Rutas,
    ajustes: AjustesVoz,
    canal: Sender<Orden>,
    /// El panel está abierto y la voz activada.
    activo: bool,
    pausado: bool,
    /// Ya mandó un pedido y espera la respuesta (no escucha).
    esperando: bool,
    microfono: Option<Microfono>,
    remuestreador: Option<Remuestreador>,
    pendiente: Vec<f32>,
    vad: Vad,
    nivel_mic: f32,
    whisper: Option<ServidorWhisper>,
    ultimo_uso_whisper: Instant,
    ultimo_parcial: Instant,
    piper: Option<Piper>,
    habla: Option<Habla>,
    /// Hasta cuándo se ignora el micrófono (eco de lo que acaba de decir).
    sordo_hasta: Instant,
    estado: Estado,
}

impl Motor {
    pub fn new(avisar_evento: Avisador, rutas: Rutas, ajustes: AjustesVoz, canal: Sender<Orden>) -> Self {
        Motor {
            avisar_evento,
            microfono_de_prueba: None,
            rutas,
            ajustes,
            canal,
            activo: false,
            pausado: false,
            esperando: false,
            microfono: None,
            remuestreador: None,
            pendiente: Vec::new(),
            vad: Vad::new(AjustesVad::default()),
            nivel_mic: 0.0,
            whisper: None,
            ultimo_uso_whisper: Instant::now(),
            ultimo_parcial: Instant::now(),
            piper: None,
            habla: None,
            sordo_hasta: Instant::now(),
            estado: Estado::Apagada,
        }
    }

    /// Para los tests: sin micrófono de verdad (el audio se manda a mano).
    #[cfg(test)]
    pub fn sin_microfono(mut self, frecuencia: u32) -> Self {
        self.microfono_de_prueba = Some(frecuencia);
        self
    }

    /// El ciclo del hilo: espera órdenes (o el próximo cuadro de la esfera).
    pub fn correr(mut self, ordenes: Receiver<Orden>) {
        let mut ultimo_nivel = Instant::now();
        loop {
            match ordenes.recv_timeout(CADA_NIVEL) {
                Ok(Orden::Salir(listo)) => {
                    // Al soltarlos se cierran los procesos (ver `Drop`).
                    self.callar();
                    self.cerrar_microfono();
                    self.whisper = None;
                    self.piper = None;
                    let _ = listo.send(());
                    return;
                }
                Ok(orden) => self.atender(orden),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            if ultimo_nivel.elapsed() >= CADA_NIVEL {
                ultimo_nivel = Instant::now();
                self.cada_cuadro();
            }
        }
    }

    fn avisar(&mut self, estado: Estado, mensaje: Option<String>) {
        if estado == self.estado && mensaje.is_none() {
            return;
        }
        self.estado = estado;
        (self.avisar_evento)(EVENTO_ESTADO, json!(AvisoEstado { estado, mensaje }));
    }

    fn error(&mut self, mensaje: impl Into<String>) {
        let mensaje = mensaje.into();
        eprintln!("Voz: {mensaje}");
        self.avisar(Estado::Error, Some(mensaje));
    }

    fn modelos_listos(&self) -> bool {
        self.rutas.modelo_whisper().exists()
            && (self.rutas.servidor_whisper(true).exists() || self.rutas.servidor_whisper(false).exists())
            && self.rutas.piper().exists()
            && self.rutas.voz().exists()
    }

    fn atender(&mut self, orden: Orden) {
        match orden {
            Orden::Activar => self.activar(),
            Orden::Desactivar => {
                self.activo = false;
                self.esperando = false;
                self.callar();
                self.cerrar_microfono();
                self.avisar(Estado::Apagada, None);
            }
            Orden::Pausar => {
                self.pausado = true;
                self.vad.reiniciar();
                if self.activo && self.habla.is_none() {
                    self.avisar(Estado::Pausada, None);
                }
            }
            Orden::Reanudar => {
                self.pausado = false;
                self.esperando = false;
                self.volver_a_escuchar();
            }
            Orden::Esperar => {
                self.esperando = true;
                self.vad.reiniciar();
                if self.activo {
                    self.avisar(Estado::Pensando, None);
                }
            }
            Orden::Hablar(texto) => {
                // Llegó la respuesta: ya no espera. Al terminar de hablar
                // vuelve a escuchar (antes quedaba sordo para siempre).
                self.esperando = false;
                self.hablar(&texto);
            }
            Orden::Callar => {
                self.callar();
                self.volver_a_escuchar();
            }
            Orden::Configurar(ajustes) => {
                let cambio_velocidad = (ajustes.velocidad - self.ajustes.velocidad).abs() > f32::EPSILON;
                let cambio_gpu = ajustes.usar_gpu != self.ajustes.usar_gpu;
                self.ajustes = ajustes;
                if cambio_velocidad {
                    self.piper = None; // se reabre con la velocidad nueva
                }
                if cambio_gpu {
                    self.whisper = None;
                }
                if !self.ajustes.activada {
                    self.cerrar_microfono();
                    self.callar();
                    self.avisar(Estado::Apagada, None);
                } else if self.activo {
                    self.activar();
                }
            }
            Orden::Microfono(DelMicrofono::Audio(muestras)) => self.oir(&muestras),
            Orden::Microfono(DelMicrofono::Error(e)) => {
                self.cerrar_microfono();
                self.error(e);
            }
            Orden::Salir(listo) => {
                let _ = listo.send(());
            }
        }
    }

    fn activar(&mut self) {
        self.activo = true;
        self.esperando = false;
        if !self.ajustes.activada {
            return self.avisar(Estado::Apagada, None);
        }
        if !self.modelos_listos() {
            return self.avisar(Estado::SinModelos, None);
        }
        if self.whisper.as_mut().is_none_or(|w| !w.vivo()) {
            self.avisar(Estado::Preparando, None);
            // Arrancar Whisper es asíncrono (espera a que responda); acá se
            // bloquea este hilo hasta que esté. Mientras, el audio del
            // micrófono se va juntando en el canal.
            let rutas = self.rutas.clone();
            let gpu = self.ajustes.usar_gpu;
            match tauri::async_runtime::block_on(ServidorWhisper::arrancar(&rutas, gpu)) {
                Ok(servidor) => self.whisper = Some(servidor),
                Err(e) => return self.error(e.to_string()),
            }
        }
        self.ultimo_uso_whisper = Instant::now();
        if let Some(frecuencia) = self.microfono_de_prueba {
            self.remuestreador.get_or_insert_with(|| Remuestreador::new(frecuencia, FRECUENCIA_WHISPER));
        } else if self.microfono.is_none() {
            match Microfono::abrir(self.canal.clone(), del_microfono) {
                Ok(m) => {
                    self.remuestreador = Some(Remuestreador::new(m.frecuencia, FRECUENCIA_WHISPER));
                    self.microfono = Some(m);
                }
                Err(e) => return self.error(e),
            }
        }
        self.vad.reiniciar();
        if self.habla.is_none() {
            self.avisar(if self.pausado { Estado::Pausada } else { Estado::Escuchando }, None);
        }
    }

    fn cerrar_microfono(&mut self) {
        self.microfono = None;
        self.remuestreador = None;
        self.pendiente.clear();
        self.vad.reiniciar();
        self.nivel_mic = 0.0;
    }

    fn escuchando(&self) -> bool {
        self.activo && !self.pausado && !self.esperando && self.habla.is_none() && Instant::now() >= self.sordo_hasta
    }

    fn volver_a_escuchar(&mut self) {
        if !self.activo || !self.ajustes.activada || self.habla.is_some() {
            return;
        }
        if self.microfono.is_none() && self.microfono_de_prueba.is_none() {
            return self.activar();
        }
        self.vad.reiniciar();
        self.avisar(if self.pausado { Estado::Pausada } else { Estado::Escuchando }, None);
    }

    /// Llega audio del micrófono: a 16 kHz, en cuadros de 30 ms, al detector.
    fn oir(&mut self, muestras: &[f32]) {
        let Some(remuestreador) = self.remuestreador.as_mut() else { return };
        let a_16k = remuestreador.procesar(muestras);
        if !self.escuchando() {
            self.pendiente.clear();
            self.nivel_mic = 0.0;
            return;
        }
        self.pendiente.extend(a_16k);
        while self.pendiente.len() >= MUESTRAS_POR_CUADRO {
            let cuadro: Vec<f32> = self.pendiente.drain(..MUESTRAS_POR_CUADRO).collect();
            // El nivel sube rápido y baja despacio: la esfera no tiembla.
            let nivel = nivel_visual(rms(&cuadro));
            self.nivel_mic = if nivel > self.nivel_mic { nivel } else { self.nivel_mic * 0.85 + nivel * 0.15 };
            match self.vad.procesar(&cuadro) {
                Some(EventoVad::Empezo) => {
                    self.ultimo_parcial = Instant::now();
                    self.avisar(Estado::Oyendo, None);
                }
                Some(EventoVad::Termino(audio)) => {
                    self.transcribir_final(audio);
                    // Lo que quedó en `pendiente` ya no corresponde.
                    self.pendiente.clear();
                    return;
                }
                Some(EventoVad::Descartado) => self.avisar(Estado::Escuchando, None),
                None => {}
            }
        }
        if self.vad.hablando() && self.ultimo_parcial.elapsed() >= CADA_PARCIAL {
            self.transcribir_parcial();
        }
    }

    fn transcribir(&mut self, audio: &[f32]) -> Option<String> {
        let whisper = self.whisper.as_ref()?;
        self.ultimo_uso_whisper = Instant::now();
        match tauri::async_runtime::block_on(whisper.transcribir(wav(audio, FRECUENCIA_WHISPER))) {
            Ok(texto) => limpiar_transcripcion(&texto).filter(|t| !es_eco_del_contexto(t, super::whisper::CONTEXTO)),
            Err(e) => {
                eprintln!("Voz: {e}");
                None
            }
        }
    }

    /// Lo que vas diciendo, para mostrarlo mientras hablás. Con CPU es muy
    /// lento para hacerlo seguido, así que solo con la placa de video.
    fn transcribir_parcial(&mut self) {
        self.ultimo_parcial = Instant::now();
        if !self.whisper.as_ref().is_some_and(|w| w.gpu) {
            return;
        }
        let Some(audio) = self.vad.audio_actual().map(<[f32]>::to_vec) else { return };
        if audio.len() < FRECUENCIA_WHISPER as usize / 2 {
            return;
        }
        if let Some(texto) = self.transcribir(&audio) {
            (self.avisar_evento)(EVENTO_PARCIAL, json!(texto));
        }
    }

    fn transcribir_final(&mut self, audio: Vec<f32>) {
        self.avisar(Estado::Transcribiendo, None);
        match self.transcribir(&audio) {
            Some(texto) => {
                // Queda esperando la respuesta del asistente (la interfaz se
                // la manda y después pide `Hablar` o `Reanudar`).
                self.esperando = true;
                self.avisar(Estado::Pensando, None);
                (self.avisar_evento)(EVENTO_FINAL, json!(texto));
            }
            None => self.avisar(Estado::Escuchando, None),
        }
    }

    fn hablar(&mut self, texto: &str) {
        if !self.ajustes.activada || !self.modelos_listos() {
            return;
        }
        let frases = oraciones(&para_leer(texto));
        if frases.is_empty() {
            return self.volver_a_escuchar();
        }
        self.callar();
        self.vad.reiniciar();
        if self.piper.as_mut().is_none_or(|p| !p.vivo()) {
            match Piper::abrir(&self.rutas, self.ajustes.velocidad) {
                Ok(p) => self.piper = Some(p),
                Err(e) => return self.error(e.to_string()),
            }
        }
        let parlante = match Parlante::abrir() {
            Ok(p) => p,
            Err(e) => return self.error(e),
        };
        self.habla = Some(Habla { parlante, niveles: Vec::new(), frases: frases.into() });
        self.avisar(Estado::Hablando, None);
        self.sintetizar_siguiente();
    }

    /// Sintetiza la próxima frase y la pone a sonar después de lo que ya hay.
    fn sintetizar_siguiente(&mut self) {
        let Some(frase) = self.habla.as_mut().and_then(|h| h.frases.pop_front()) else { return };
        let Some(piper) = self.piper.as_mut() else { return };
        let resultado = piper.sintetizar(&frase);
        let Some(habla) = self.habla.as_mut() else { return };
        match resultado {
            Ok((frecuencia, audio)) => {
                let salida = remuestrear(&audio, frecuencia, habla.parlante.frecuencia);
                let por_cuadro = (habla.parlante.frecuencia / 30) as usize;
                habla.niveles.extend(envolvente(&salida, por_cuadro));
                habla.parlante.encolar(&salida);
            }
            Err(e) => {
                // Lo que ya sonó queda; el resto se pierde y Piper se reabre la próxima.
                habla.frases.clear();
                self.piper = None;
                eprintln!("Voz: {e}");
            }
        }
    }

    fn callar(&mut self) {
        if let Some(habla) = self.habla.take() {
            habla.parlante.vaciar();
            self.sordo_hasta = Instant::now() + COLA_DEL_ECO;
        }
    }

    /// 30 veces por segundo: el nivel para la esfera, si terminó de hablar
    /// y si conviene cerrar Whisper.
    fn cada_cuadro(&mut self) {
        // Le queda menos de un segundo por sonar: se prepara la frase siguiente.
        if self.habla.as_ref().is_some_and(|h| !h.frases.is_empty() && h.parlante.pendientes() < h.parlante.frecuencia as usize) {
            self.sintetizar_siguiente();
        }
        let mut termino = false;
        let nivel = if let Some(habla) = &self.habla {
            let cuadro = (habla.parlante.sonadas() / u64::from(habla.parlante.frecuencia / 30).max(1)) as usize;
            termino = habla.frases.is_empty() && habla.parlante.pendientes() == 0;
            habla.niveles.get(cuadro).copied().unwrap_or(0.0)
        } else if self.escuchando() {
            self.nivel_mic
        } else {
            0.0
        };
        if self.activo || self.habla.is_some() {
            (self.avisar_evento)(EVENTO_NIVEL, json!(nivel));
        }
        if termino {
            self.habla = None;
            self.sordo_hasta = Instant::now() + COLA_DEL_ECO;
            if self.activo {
                self.avisar(if self.pausado { Estado::Pausada } else { Estado::Escuchando }, None);
                self.vad.reiniciar();
            } else {
                self.avisar(Estado::Apagada, None);
            }
        }
        if !self.activo && self.whisper.is_some() && self.ultimo_uso_whisper.elapsed() > WHISPER_OCIOSO {
            self.whisper = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motor_sin_modelos() -> Motor {
        let (tx, _rx) = std::sync::mpsc::channel();
        let rutas = Rutas::new(std::env::temp_dir().join("nova-voz-que-no-existe"));
        let mut motor = Motor::new(Box::new(|_, _| {}), rutas, AjustesVoz::default(), tx).sin_microfono(48_000);
        motor.activo = true;
        motor.remuestreador = Some(Remuestreador::new(48_000, FRECUENCIA_WHISPER));
        motor
    }

    #[test]
    fn la_respuesta_hablada_deja_volver_a_escuchar() {
        let mut motor = motor_sin_modelos();
        // Así queda después de transcribir un pedido.
        motor.esperando = true;
        assert!(!motor.escuchando());
        motor.atender(Orden::Hablar("Listo, te lo anoto.".into()));
        assert!(!motor.esperando);
        assert!(motor.escuchando());
    }

    #[test]
    fn reanudar_tambien_vuelve_a_escuchar() {
        let mut motor = motor_sin_modelos();
        motor.esperando = true;
        motor.atender(Orden::Reanudar);
        assert!(motor.escuchando());
    }
}
