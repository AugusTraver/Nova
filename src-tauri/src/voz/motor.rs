//! El motor de voz: un hilo propio que es dueño del micrófono, el detector
//! de voz, Whisper y los parlantes. Convertir texto en voz lo hace otro
//! hilo (`sintetizador`), que le devuelve el audio por el mismo canal. Todo le llega por un solo canal
//! (órdenes de la interfaz y audio del micrófono), así que nunca hay dos
//! partes tocando lo mismo a la vez.
//!
//! El ciclo:
//!   escuchando → (hablás) oyendo → (te callás) transcribiendo → la interfaz
//!   le manda el texto al asistente → Nova responde hablando → escuchando.
//!
//! Mientras Nova habla, el micrófono se ignora: si no, se escucharía a sí
//! misma por los parlantes.
//!
//! Con el panel cerrado y "Oye Nova" activado, el micrófono queda abierto y
//! el audio va al detector (`palabra`) en vez de al ciclo de arriba. Cuando
//! lo oye, el motor ya se pone a escuchar (así no se pierde lo que digas
//! enseguida) y pide abrir el panel.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use super::audio::{FRECUENCIA_WHISPER, Remuestreador, envolvente, nivel_visual, remuestrear, rms, wav};
use super::dispositivos::{DelMicrofono, Microfono, Parlante};
use super::modelos::Rutas;
use super::palabra::{self, Decision, Detector};
use super::sintetizador::{Audio, Sintetizador};
use super::texto::{es_eco_del_contexto, limpiar_transcripcion, oraciones, para_leer};
use super::vad::{AjustesVad, EventoVad, MUESTRAS_POR_CUADRO, Vad};
use super::whisper::ServidorWhisper;
use super::AjustesVoz;

pub const EVENTO_ESTADO: &str = "voz:estado";
pub const EVENTO_NIVEL: &str = "voz:nivel";
pub const EVENTO_PARCIAL: &str = "voz:parcial";
pub const EVENTO_FINAL: &str = "voz:final";
/// Oyó "Oye Nova" (para el pulso de la esfera).
pub const EVENTO_DESPERTAR: &str = "voz:despertar";

/// Cada cuánto se avisa el nivel a la esfera (unas 30 veces por segundo).
const CADA_NIVEL: Duration = Duration::from_millis(33);
/// Cada cuánto se transcribe lo que vas diciendo (transcripción en vivo).
const CADA_PARCIAL: Duration = Duration::from_millis(700);
/// Whisper se cierra si nadie lo usa en este tiempo con el panel cerrado.
const WHISPER_OCIOSO: Duration = Duration::from_secs(10 * 60);
/// Después de hablar, un ratito sin escuchar (el eco del final).
const COLA_DEL_ECO: Duration = Duration::from_millis(350);
/// Con el panel cerrado no hay esfera que mover: el ciclo duerme más.
const ESPERA_CERRADO: Duration = Duration::from_millis(500);
/// Cada cuánto se mira si la sesión está bloqueada, la batería y la hora.
const CADA_REVISION: Duration = Duration::from_secs(5);

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
    /// Hacer sonar un audio ya sintetizado (el comparador de voces de Ajustes).
    Sonar(Audio),
    /// Volvió una frase del sintetizador.
    Sintetizado { habla: u64, resultado: crate::error::Resultado<Audio> },
    Callar,
    Configurar(AjustesVoz),
    Microfono(DelMicrofono),
    /// Cambió "no molestar" (puede pausar "Oye Nova").
    ConfigurarAvisos(crate::dominio::avisos::AjustesAvisos),
    /// Nova se cierra: cerrar los programas de voz y avisar por el canal.
    Salir(Sender<()>),
}

fn del_microfono(m: DelMicrofono) -> Orden {
    Orden::Microfono(m)
}

struct Habla {
    /// El número que le dio el sintetizador (lo que vuelve con otro número
    /// es de una respuesta que se cortó).
    id: u64,
    /// El nivel de cada 1/30 s de lo encolado, para la esfera.
    niveles: Vec<f32>,
    /// Cuántas frases faltan que vuelvan del sintetizador.
    faltan: usize,
    /// Lo que llevaba sonado el parlante al empezar (el parlante queda
    /// abierto entre respuestas y cuenta desde que se abrió).
    sonadas_al_empezar: u64,
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
    /// Qué hacer al oír "Oye Nova" (en la app, abrir el panel).
    al_despertar: Option<Box<dyn Fn() + Send>>,
    /// Se carga la primera vez que hace falta esperar la palabra.
    detector: Option<Detector>,
    /// Avisa si está esperando "Oye Nova" o no (para la bandeja).
    al_cambiar_espera: Option<Box<dyn Fn(bool) + Send>>,
    /// Lo último que se avisó con `al_cambiar_espera`.
    esperaba_palabra: bool,
    /// Sesión bloqueada, a batería o en "no molestar": no escucha la palabra.
    palabra_en_pausa: bool,
    ultima_revision: Instant,
    avisos: crate::dominio::avisos::AjustesAvisos,
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
    /// Se arranca la primera vez que hay que hablar.
    sintetizador: Option<Sintetizador>,
    /// Queda abierto mientras el panel está abierto: abrirlo en cada
    /// respuesta demoraba el arranque y a veces hacía un "click".
    parlante: Option<Parlante>,
    habla: Option<Habla>,
    /// Hasta cuándo se ignora el micrófono (eco de lo que acaba de decir).
    sordo_hasta: Instant,
    estado: Estado,
}

impl Motor {
    pub fn new(avisar_evento: Avisador, rutas: Rutas, ajustes: AjustesVoz, canal: Sender<Orden>) -> Self {
        Motor {
            avisar_evento,
            al_despertar: None,
            detector: None,
            al_cambiar_espera: None,
            esperaba_palabra: false,
            palabra_en_pausa: false,
            ultima_revision: Instant::now(),
            avisos: Default::default(),
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
            sintetizador: None,
            parlante: None,
            habla: None,
            sordo_hasta: Instant::now(),
            estado: Estado::Apagada,
        }
    }

    pub fn al_despertar(mut self, f: Box<dyn Fn() + Send>) -> Self {
        self.al_despertar = Some(f);
        self
    }

    pub fn al_cambiar_espera(mut self, f: Box<dyn Fn(bool) + Send>) -> Self {
        self.al_cambiar_espera = Some(f);
        self
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
            let espera = if self.activo || self.habla.is_some() { CADA_NIVEL } else { ESPERA_CERRADO };
            match ordenes.recv_timeout(espera) {
                Ok(Orden::Salir(listo)) => {
                    // Al soltarlos se cierran los procesos (ver `Drop`).
                    self.callar();
                    self.cerrar_microfono();
                    self.whisper = None;
                    self.sintetizador = None;
                    self.parlante = None;
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
                self.parlante = None;
                self.avisar(Estado::Apagada, None);
                // Con "Oye Nova", el micrófono se vuelve a abrir para esperarla.
                self.preparar_palabra();
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
            Orden::Sonar(audio) => self.sonar(audio),
            Orden::Sintetizado { habla, resultado } => self.llego_audio(habla, resultado),
            Orden::Callar => {
                self.callar();
                self.volver_a_escuchar();
            }
            Orden::Configurar(ajustes) => {
                let cambio_gpu = ajustes.usar_gpu != self.ajustes.usar_gpu;
                if let Some(s) = &self.sintetizador {
                    s.configurar(ajustes.clone());
                }
                self.ajustes = ajustes;
                if cambio_gpu {
                    self.whisper = None;
                }
                if !self.ajustes.activada {
                    self.cerrar_microfono();
                    self.callar();
                    self.avisar(Estado::Apagada, None);
                } else if self.activo {
                    self.activar();
                } else {
                    // Panel cerrado: abrir o soltar el micrófono según "Oye Nova".
                    self.preparar_palabra();
                }
            }
            Orden::ConfigurarAvisos(avisos) => {
                self.avisos = avisos;
                self.revisar_pausa();
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
        // Si "Oye Nova" ya lo puso a escuchar, el `Activar` que llega al
        // abrirse el panel no tiene que cortar lo que estás diciendo.
        let ya_escuchaba = self.activo
            && (self.microfono.is_some() || self.microfono_de_prueba.is_some())
            && matches!(self.estado, Estado::Escuchando | Estado::Oyendo | Estado::Transcribiendo | Estado::Pensando);
        if ya_escuchaba && self.ajustes.activada && self.whisper.is_some() {
            return;
        }
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

    /// ¿Con el panel cerrado hay que esperar "Oye Nova"?
    fn espera_palabra(&self) -> bool {
        !self.activo
            && !self.palabra_en_pausa
            && self.ajustes.activada
            && self.ajustes.palabra_clave
            && self.modelos_listos()
            && self.rutas.modelo_palabra().exists()
    }

    /// Panel cerrado: si hay que esperar "Oye Nova", deja listos Whisper (para
    /// confirmar), el detector y el micrófono. Si no, suelta el micrófono.
    fn preparar_palabra(&mut self) {
        if self.activo {
            return;
        }
        if !self.espera_palabra() {
            self.cerrar_microfono();
            return;
        }
        if self.whisper.as_mut().is_none_or(|w| !w.vivo()) {
            let rutas = self.rutas.clone();
            match tauri::async_runtime::block_on(ServidorWhisper::arrancar(&rutas, self.ajustes.usar_gpu)) {
                Ok(servidor) => self.whisper = Some(servidor),
                Err(e) => return eprintln!("Voz: sin Whisper no espero \"Oye Nova\": {e}"),
            }
        }
        if self.detector.is_none() {
            match Detector::cargar(&self.rutas.modelo_palabra()) {
                Ok(d) => self.detector = Some(d),
                Err(e) => return eprintln!("Voz: {e}"),
            }
        }
        if let Some(d) = self.detector.as_mut() {
            d.reiniciar();
        }
        if let Some(frecuencia) = self.microfono_de_prueba {
            self.remuestreador.get_or_insert_with(|| Remuestreador::new(frecuencia, FRECUENCIA_WHISPER));
        } else if self.microfono.is_none() {
            match Microfono::abrir(self.canal.clone(), del_microfono) {
                Ok(m) => {
                    self.remuestreador = Some(Remuestreador::new(m.frecuencia, FRECUENCIA_WHISPER));
                    self.microfono = Some(m);
                }
                Err(e) => eprintln!("Voz: no pude abrir el micrófono para \"Oye Nova\": {e}"),
            }
        }
    }

    /// ¿Hay que pausar "Oye Nova"? Sesión bloqueada, desenchufada (si así se
    /// eligió) o en "no molestar" (ídem). Al cambiar, suelta o vuelve a abrir
    /// el micrófono.
    fn revisar_pausa(&mut self) {
        self.ultima_revision = Instant::now();
        let ahora = chrono::Local::now().fixed_offset();
        let pausa = super::sistema::sesion_bloqueada()
            || (self.ajustes.palabra_pausa_bateria && super::sistema::a_bateria())
            || (self.avisos.no_molestar_pausa_palabra && self.avisos.en_no_molestar(ahora));
        if pausa == self.palabra_en_pausa {
            return;
        }
        self.palabra_en_pausa = pausa;
        if !self.activo {
            // `preparar_palabra` ya mira la pausa: abre o suelta el micrófono.
            self.preparar_palabra();
        }
    }

    /// ¿Está esperando "Oye Nova" ahora mismo? (micrófono abierto con el
    /// panel cerrado). Se avisa solo cuando cambia.
    fn avisar_espera(&mut self) {
        let espera = !self.activo && self.detector.is_some() && self.microfono.is_some() && !self.palabra_en_pausa;
        if espera != self.esperaba_palabra {
            self.esperaba_palabra = espera;
            if let Some(f) = &self.al_cambiar_espera {
                f(espera);
            }
        }
    }

    /// Panel cerrado: el audio va al detector.
    fn esperar_palabra(&mut self, a_16k: &[f32]) {
        let Some(detector) = self.detector.as_mut() else { return };
        let Some(decision) = detector.oir(a_16k) else { return };
        if decision == Decision::Confirmar {
            let audio = detector.audio_reciente();
            let texto = self.transcribir(&audio).unwrap_or_default();
            if !palabra::confirma(&texto) {
                eprintln!("Voz: parecía \"Oye Nova\" pero Whisper entendió «{texto}»");
                return;
            }
        }
        self.despertar();
    }

    /// Oyó "Oye Nova": ya se pone a escuchar y pide abrir el panel.
    fn despertar(&mut self) {
        if let Some(d) = self.detector.as_mut() {
            d.reiniciar();
        }
        self.activo = true;
        self.esperando = false;
        self.pausado = false;
        self.pendiente.clear();
        self.vad.reiniciar();
        self.ultimo_uso_whisper = Instant::now();
        self.avisar(Estado::Escuchando, None);
        (self.avisar_evento)(EVENTO_DESPERTAR, json!(null));
        if let Some(abrir) = &self.al_despertar {
            abrir();
        }
    }

    /// Llega audio del micrófono: a 16 kHz, en cuadros de 30 ms, al detector.
    fn oir(&mut self, muestras: &[f32]) {
        let Some(remuestreador) = self.remuestreador.as_mut() else { return };
        let a_16k = remuestreador.procesar(muestras);
        if !self.activo {
            if self.habla.is_none() {
                self.esperar_palabra(&a_16k);
            }
            return;
        }
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
        if !self.ajustes.activada {
            return;
        }
        let frases = oraciones(&para_leer(texto));
        if frases.is_empty() {
            return self.volver_a_escuchar();
        }
        if self.sintetizador.is_none() {
            let canal = self.canal.clone();
            let al_sintetizar = Box::new(move |habla, resultado| {
                let _ = canal.send(Orden::Sintetizado { habla, resultado });
            });
            match Sintetizador::arrancar(self.rutas.clone(), self.ajustes.clone(), al_sintetizar) {
                Ok(s) => self.sintetizador = Some(s),
                Err(e) => return self.error(e.to_string()),
            }
        }
        // Primero se calla lo anterior (eso cancela su número en el
        // sintetizador) y después se mandan las frases nuevas.
        self.callar();
        let faltan = frases.len();
        let Some(id) = self.sintetizador.as_ref().map(|s| s.decir(frases)) else { return };
        self.empezar_a_hablar(id, faltan);
    }

    /// Prepara el parlante y deja todo listo para que vaya llegando el audio.
    fn empezar_a_hablar(&mut self, id: u64, faltan: usize) -> bool {
        self.callar();
        self.vad.reiniciar();
        if self.parlante.is_none() {
            match Parlante::abrir() {
                Ok(p) => self.parlante = Some(p),
                Err(e) => {
                    self.error(e);
                    return false;
                }
            }
        }
        let sonadas_al_empezar = self.parlante.as_ref().map_or(0, Parlante::sonadas);
        self.habla = Some(Habla { id, niveles: Vec::new(), faltan, sonadas_al_empezar });
        self.avisar(Estado::Hablando, None);
        true
    }

    /// Un audio que ya viene hecho (el comparador de Ajustes).
    fn sonar(&mut self, audio: Audio) {
        // El 0 nunca lo usa el sintetizador: nada suyo se mezcla con esto.
        if self.empezar_a_hablar(0, 0) {
            self.encolar(audio);
        }
    }

    fn llego_audio(&mut self, id: u64, resultado: crate::error::Resultado<Audio>) {
        let Some(habla) = self.habla.as_mut().filter(|h| h.id == id) else { return };
        match resultado {
            Ok(audio) => {
                habla.faltan = habla.faltan.saturating_sub(1);
                self.encolar(audio);
            }
            Err(e) => {
                // Lo que ya sonó queda; el resto no se dice.
                eprintln!("Voz: {e}");
                habla.faltan = 0;
                if let Some(s) = &self.sintetizador {
                    s.cancelar();
                }
            }
        }
    }

    /// Pone a sonar un audio después de lo que ya hay.
    fn encolar(&mut self, (frecuencia, audio): Audio) {
        let (Some(habla), Some(parlante)) = (self.habla.as_mut(), self.parlante.as_ref()) else { return };
        let salida = remuestrear(&audio, frecuencia, parlante.frecuencia);
        let por_cuadro = (parlante.frecuencia / 30) as usize;
        habla.niveles.extend(envolvente(&salida, por_cuadro));
        parlante.encolar(&salida);
    }

    fn callar(&mut self) {
        if self.habla.take().is_some() {
            if let Some(p) = &self.parlante {
                p.vaciar();
            }
            if let Some(s) = &self.sintetizador {
                s.cancelar();
            }
            self.sordo_hasta = Instant::now() + COLA_DEL_ECO;
        }
    }

    /// 30 veces por segundo: el nivel para la esfera, si terminó de hablar
    /// y si conviene cerrar Whisper.
    fn cada_cuadro(&mut self) {
        if self.ultima_revision.elapsed() >= CADA_REVISION {
            self.revisar_pausa();
        }
        self.avisar_espera();
        let mut termino = false;
        let nivel = if let (Some(habla), Some(parlante)) = (&self.habla, &self.parlante) {
            let sonadas = parlante.sonadas().saturating_sub(habla.sonadas_al_empezar);
            let cuadro = (sonadas / u64::from(parlante.frecuencia / 30).max(1)) as usize;
            termino = habla.faltan == 0 && parlante.pendientes() == 0;
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
                self.parlante = None;
                self.avisar(Estado::Apagada, None);
            }
        }
        // Con "Oye Nova" Whisper queda abierto: confirma la palabra.
        if !self.activo && self.whisper.is_some() && self.ultimo_uso_whisper.elapsed() > WHISPER_OCIOSO && !self.espera_palabra() {
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
        // Una respuesta sin nada para leer: no abre el parlante (los tests
        // no tocan dispositivos) pero tiene que dejar de esperar igual.
        motor.atender(Orden::Hablar("**".into()));
        assert!(!motor.esperando);
        assert!(motor.escuchando());
    }

    #[test]
    fn descarta_el_audio_de_una_respuesta_cortada() {
        let mut motor = motor_sin_modelos();
        motor.habla = Some(Habla { id: 5, niveles: Vec::new(), faltan: 2, sonadas_al_empezar: 0 });
        // Llega una frase de la respuesta anterior (la 4): no cuenta.
        motor.atender(Orden::Sintetizado { habla: 4, resultado: Ok((24_000, vec![0.0; 100])) });
        assert_eq!(motor.habla.as_ref().unwrap().faltan, 2);
        motor.atender(Orden::Sintetizado { habla: 5, resultado: Ok((24_000, vec![0.0; 100])) });
        assert_eq!(motor.habla.as_ref().unwrap().faltan, 1);
        // Si una frase falla, el resto no se dice.
        motor.atender(Orden::Sintetizado { habla: 5, resultado: Err(crate::error::Error::Sistema("sin red".into())) });
        assert_eq!(motor.habla.as_ref().unwrap().faltan, 0);
    }

    #[test]
    fn reanudar_tambien_vuelve_a_escuchar() {
        let mut motor = motor_sin_modelos();
        motor.esperando = true;
        motor.atender(Orden::Reanudar);
        assert!(motor.escuchando());
    }
}
