//! Detección de actividad de voz (VAD): decide cuándo empezás y cuándo
//! terminás de hablar, para mandarle a Whisper solo el pedido.
//!
//! Es por energía, con un "piso de ruido" que se adapta: el ruido de la
//! habitación es lo más bajo que sonó en los últimos segundos (al hablar
//! siempre hay pausas; un ventilador, en cambio, suena parejo y termina
//! siendo el piso). Voz es lo que suena bastante más fuerte que eso. Sin modelos ni dependencias, y se testea con señales
//! inventadas. Trabaja en cuadros de 30 ms a 16 kHz.

use super::audio::rms;

/// 30 ms a 16 kHz.
pub const MUESTRAS_POR_CUADRO: usize = 480;
const MS_POR_CUADRO: u32 = 30;
/// Cuántos cuadros se miran para estimar el ruido (5 segundos).
const CUADROS_DE_RUIDO: usize = 167;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AjustesVad {
    /// Cuántas veces más fuerte que el ruido tiene que sonar para ser voz.
    pub factor_inicio: f32,
    /// Por debajo de este factor, vuelve a ser silencio (histéresis: así una
    /// sílaba más baja no corta la frase).
    pub factor_fin: f32,
    /// Piso mínimo absoluto (un micrófono muy limpio no dispara con un roce).
    pub rms_minimo: f32,
    /// Hace falta este tanto de voz seguida para arrancar.
    pub ms_para_empezar: u32,
    /// Este tanto de silencio seguido termina el pedido.
    pub ms_de_silencio_final: u32,
    /// Un pedido más corto que esto se descarta (un golpe, una tos).
    pub ms_minimos: u32,
    /// Un pedido no puede durar más que esto (se corta y se manda).
    pub ms_maximos: u32,
    /// Audio de antes del arranque que se agrega (la primera sílaba suele
    /// empezar bajito y no llegar al umbral).
    pub ms_previos: u32,
}

impl Default for AjustesVad {
    fn default() -> Self {
        AjustesVad {
            factor_inicio: 3.2,
            factor_fin: 1.8,
            rms_minimo: 0.008,
            ms_para_empezar: 120,
            ms_de_silencio_final: 900,
            ms_minimos: 350,
            ms_maximos: 20_000,
            ms_previos: 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventoVad {
    /// Empezaste a hablar.
    Empezo,
    /// Terminaste: el audio del pedido (con lo previo incluido).
    Termino(Vec<f32>),
    /// Era muy corto: no era un pedido.
    Descartado,
}

pub struct Vad {
    ajustes: AjustesVad,
    /// Estimación del ruido de fondo (RMS).
    ruido: f32,
    /// La energía de los últimos cuadros (para el piso de ruido).
    energias: std::collections::VecDeque<f32>,
    /// Cuadros recientes en silencio (para agregar lo previo al arrancar).
    previos: std::collections::VecDeque<Vec<f32>>,
    /// Cuadros de voz seguidos mientras todavía no arrancó.
    candidatos: Vec<Vec<f32>>,
    /// El pedido en curso, si se está hablando.
    pedido: Option<Vec<f32>>,
    cuadros_de_voz: u32,
    cuadros_en_silencio: u32,
}

impl Vad {
    pub fn new(ajustes: AjustesVad) -> Self {
        Vad {
            ajustes,
            ruido: ajustes.rms_minimo / 2.0,
            energias: Default::default(),
            previos: Default::default(),
            candidatos: Vec::new(),
            pedido: None,
            cuadros_de_voz: 0,
            cuadros_en_silencio: 0,
        }
    }

    pub fn hablando(&self) -> bool {
        self.pedido.is_some()
    }

    /// El audio del pedido en curso (para la transcripción en vivo).
    pub fn audio_actual(&self) -> Option<&[f32]> {
        self.pedido.as_deref()
    }

    /// Vuelve a empezar (por ejemplo, después de que Nova habló: lo que se
    /// grabó mientras tanto era su propia voz).
    pub fn reiniciar(&mut self) {
        self.previos.clear();
        self.candidatos.clear();
        self.pedido = None;
        self.cuadros_de_voz = 0;
        self.cuadros_en_silencio = 0;
    }

    fn cuadros(&self, ms: u32) -> u32 {
        ms.div_ceil(MS_POR_CUADRO)
    }

    /// Procesa un cuadro de `MUESTRAS_POR_CUADRO` muestras a 16 kHz.
    pub fn procesar(&mut self, cuadro: &[f32]) -> Option<EventoVad> {
        let energia = rms(cuadro);
        self.energias.push_back(energia);
        if self.energias.len() > CUADROS_DE_RUIDO {
            self.energias.pop_front();
        }
        // El piso es el mínimo reciente, pero se mueve suave (sin saltos).
        let piso = self.energias.iter().copied().fold(f32::MAX, f32::min);
        self.ruido += (piso - self.ruido) * 0.05;
        let umbral_inicio = (self.ruido * self.ajustes.factor_inicio).max(self.ajustes.rms_minimo);
        let umbral_fin = (self.ruido * self.ajustes.factor_fin).max(self.ajustes.rms_minimo * 0.6);

        if let Some(pedido) = self.pedido.as_mut() {
            pedido.extend_from_slice(cuadro);
            if energia < umbral_fin {
                self.cuadros_en_silencio += 1;
            } else {
                self.cuadros_en_silencio = 0;
            }
            let largo_ms = (pedido.len() / (MUESTRAS_POR_CUADRO / MS_POR_CUADRO as usize)) as u32;
            let callado = self.cuadros_en_silencio >= self.cuadros(self.ajustes.ms_de_silencio_final);
            if callado || largo_ms >= self.ajustes.ms_maximos {
                let mut audio = self.pedido.take().unwrap_or_default();
                // Se saca casi todo el silencio del final (deja 200 ms).
                let sobra = (self.cuadros_en_silencio.saturating_sub(self.cuadros(200)) as usize) * MUESTRAS_POR_CUADRO;
                audio.truncate(audio.len().saturating_sub(sobra));
                let con_voz_ms = self.cuadros_de_voz * MS_POR_CUADRO;
                self.cuadros_de_voz = 0;
                self.cuadros_en_silencio = 0;
                return Some(if con_voz_ms < self.ajustes.ms_minimos { EventoVad::Descartado } else { EventoVad::Termino(audio) });
            }
            if energia >= umbral_fin {
                self.cuadros_de_voz += 1;
            }
            return None;
        }

        if energia >= umbral_inicio {
            self.candidatos.push(cuadro.to_vec());
            if self.candidatos.len() as u32 >= self.cuadros(self.ajustes.ms_para_empezar) {
                let mut pedido: Vec<f32> = self.previos.drain(..).flatten().collect();
                self.cuadros_de_voz = self.candidatos.len() as u32;
                pedido.extend(self.candidatos.drain(..).flatten());
                self.pedido = Some(pedido);
                self.cuadros_en_silencio = 0;
                return Some(EventoVad::Empezo);
            }
            return None;
        }

        // Silencio: lo que había de "casi voz" pasa a ser previo.
        for c in self.candidatos.drain(..) {
            self.previos.push_back(c);
        }
        self.previos.push_back(cuadro.to_vec());
        while self.previos.len() as u32 > self.cuadros(self.ajustes.ms_previos) {
            self.previos.pop_front();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un cuadro de "voz" (un tono) o de "ruido" (de volumen fijo).
    fn cuadro(amplitud: f32) -> Vec<f32> {
        (0..MUESTRAS_POR_CUADRO).map(|i| (i as f32 * 0.3).sin() * amplitud).collect()
    }

    fn pasar(vad: &mut Vad, amplitud: f32, ms: u32) -> Vec<EventoVad> {
        (0..ms / MS_POR_CUADRO).filter_map(|_| vad.procesar(&cuadro(amplitud))).collect()
    }

    #[test]
    fn detecta_un_pedido_y_su_final() {
        let mut vad = Vad::new(AjustesVad::default());
        assert!(pasar(&mut vad, 0.003, 1500).is_empty()); // habitación tranquila
        let inicio = pasar(&mut vad, 0.2, 1500);
        assert_eq!(inicio, vec![EventoVad::Empezo]);
        assert!(vad.hablando());
        let fin = pasar(&mut vad, 0.003, 1200);
        let [EventoVad::Termino(audio)] = fin.as_slice() else { panic!("{fin:?}") };
        // 1,5 s de voz, lo previo (300 ms) y 200 ms de cola: unos 2 s.
        let segundos = audio.len() as f32 / 16_000.0;
        assert!((1.8..2.2).contains(&segundos), "{segundos}");
        assert!(!vad.hablando());
    }

    #[test]
    fn una_pausa_corta_no_corta_la_frase() {
        let mut vad = Vad::new(AjustesVad::default());
        pasar(&mut vad, 0.003, 900);
        pasar(&mut vad, 0.2, 600);
        assert!(pasar(&mut vad, 0.003, 450).is_empty()); // respira
        assert!(pasar(&mut vad, 0.2, 600).is_empty());
        assert!(vad.hablando());
    }

    #[test]
    fn un_golpe_se_descarta() {
        let mut vad = Vad::new(AjustesVad::default());
        pasar(&mut vad, 0.003, 900);
        let eventos: Vec<EventoVad> = [pasar(&mut vad, 0.3, 150), pasar(&mut vad, 0.003, 1200)].concat();
        assert_eq!(eventos, vec![EventoVad::Empezo, EventoVad::Descartado]);
    }

    #[test]
    fn se_adapta_a_una_habitacion_ruidosa() {
        let mut vad = Vad::new(AjustesVad::default());
        // Un ventilador constante: al principio puede confundirlo, pero al
        // rato ya es "el ruido de la habitación" y el falso pedido se cierra.
        let al_principio = pasar(&mut vad, 0.02, 8000);
        assert!(al_principio.len() <= 2, "{al_principio:?}");
        assert!(!vad.hablando());
        assert!(pasar(&mut vad, 0.02, 3000).is_empty());
        // Y la voz por encima del ventilador se sigue detectando.
        assert_eq!(pasar(&mut vad, 0.2, 600), vec![EventoVad::Empezo]);
    }

    #[test]
    fn con_una_voz_de_verdad() {
        use crate::voz::audio::{leer_wav, remuestrear};
        // "Recordame comprar el regalo de Juan para el viernes", con un poco
        // de ruido de habitación antes y después.
        let (frecuencia, voz) = leer_wav(include_bytes!("ejemplos/pedido.wav")).unwrap();
        let voz = remuestrear(&voz, frecuencia, 16_000);
        let ruido = |segundos: f32| -> Vec<f32> {
            (0..(16_000.0 * segundos) as usize).map(|i| ((i * 7919 % 1000) as f32 / 1000.0 - 0.5) * 0.004).collect()
        };
        let audio: Vec<f32> = [ruido(1.5), voz.clone(), ruido(1.5)].concat();
        let mut vad = Vad::new(AjustesVad::default());
        let eventos: Vec<EventoVad> = audio.chunks_exact(MUESTRAS_POR_CUADRO).filter_map(|c| vad.procesar(c)).collect();
        let [EventoVad::Empezo, EventoVad::Termino(pedido)] = eventos.as_slice() else { panic!("{:?}", eventos.len()) };
        // El pedido entero, sin cortar la frase en las pausas entre palabras.
        let (largo, original) = (pedido.len() as f32 / 16_000.0, voz.len() as f32 / 16_000.0);
        assert!(largo > original * 0.85 && largo < original + 0.8, "{largo} vs {original}");
    }

    #[test]
    fn un_pedido_eterno_se_corta() {
        let ajustes = AjustesVad { ms_maximos: 3000, ..Default::default() };
        let mut vad = Vad::new(ajustes);
        pasar(&mut vad, 0.003, 900);
        let eventos = pasar(&mut vad, 0.2, 5000);
        assert_eq!(eventos.len(), 3); // empezó, se cortó a los 3 s, y volvió a empezar
        assert!(matches!(eventos[1], EventoVad::Termino(_)));
    }
}
