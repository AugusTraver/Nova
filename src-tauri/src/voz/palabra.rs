//! "Oye Nova" (v0.6): con el panel cerrado, un modelo chiquito escucha el
//! micrófono y, cuando oye la frase, abre el panel. Todo local: el audio no
//! sale de la compu.
//!
//! El modelo da un puntaje de 0 a 1 cada 80 ms. Con tu voz, "Oye Nova" da
//! entre 0,5 y 0,9 y el silencio o la charla, menos de 0,05. Para no perder
//! las veces que lo decís más rápido o más bajo, el umbral es bajo (0,35) y
//! lo dudoso lo confirma Whisper: si en lo que transcribe no está "nova",
//! no abre.
//!
//! - `Disparador`: decide con los puntajes (puro, con tests).
//! - `confirma`: ¿Whisper entendió "nova"? (puro, con tests).
//! - `Compuerta`: en silencio no corre el modelo (cuida la CPU; puro, con tests).
//! - `Detector`: el modelo + el disparador + los últimos segundos de audio.

use std::collections::VecDeque;
use std::path::Path;

use livekit_wakeword::WakeWordModel;

use super::audio::FRECUENCIA_WHISPER;
use crate::dominio::lenguaje::normalizar;

/// Desde acá puede ser "Oye Nova" (se confirma con Whisper).
pub const UMBRAL: f32 = 0.35;
/// Desde acá es seguro: abre sin preguntarle a Whisper.
pub const SEGURO: f32 = 0.70;
/// El modelo puntúa cada 80 ms.
const MUESTRAS_POR_PASO: usize = 1280;
/// El puntaje sube durante unos pasos: se espera hasta 6 (≈ 0,5 s) para
/// quedarse con el más alto.
const PASOS_DE_SUBIDA: u32 = 6;
/// Después de decidir, 2 s sin volver a disparar (es la misma frase).
const PASOS_DE_DESCANSO: u32 = 25;
/// Lo que se guarda para que Whisper confirme: los últimos 2,5 s.
const MUESTRAS_GUARDADAS: usize = FRECUENCIA_WHISPER as usize * 5 / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Seguro que dijo "Oye Nova".
    Despertar,
    /// Puede ser: que Whisper lo confirme.
    Confirmar,
}

/// Mira los puntajes de a uno. Cuando uno pasa el umbral, espera unos pasos
/// por si sigue subiendo y decide con el más alto.
#[derive(Debug, Default)]
pub struct Disparador {
    /// El más alto de la frase en curso (`None` = no hay ninguna).
    pico: Option<f32>,
    pasos_en_subida: u32,
    descanso: u32,
}

impl Disparador {
    pub fn puntaje(&mut self, p: f32) -> Option<Decision> {
        if self.descanso > 0 {
            self.descanso -= 1;
            return None;
        }
        match self.pico {
            None if p >= UMBRAL => {
                self.pico = Some(p);
                self.pasos_en_subida = 0;
                // Si ya es seguro, no hace falta esperar.
                if p >= SEGURO { self.decidir() } else { None }
            }
            None => None,
            Some(pico) => {
                let pico = pico.max(p);
                self.pico = Some(pico);
                self.pasos_en_subida += 1;
                if pico >= SEGURO || p < UMBRAL || self.pasos_en_subida >= PASOS_DE_SUBIDA {
                    self.decidir()
                } else {
                    None
                }
            }
        }
    }

    fn decidir(&mut self) -> Option<Decision> {
        let pico = self.pico.take()?;
        self.descanso = PASOS_DE_DESCANSO;
        Some(if pico >= SEGURO { Decision::Despertar } else { Decision::Confirmar })
    }

    pub fn reiniciar(&mut self) {
        *self = Disparador::default();
    }
}

/// ¿Lo que entendió Whisper tiene "nova"? Whisper a veces lo escribe "Noba"
/// o lo pega a la coma ("Oye,Nova").
pub fn confirma(texto: &str) -> bool {
    normalizar(texto)
        .split(|c: char| !c.is_alphanumeric())
        .any(|p| matches!(p, "nova" | "noba" | "novah"))
}

/// Debajo de esto es silencio (−54 dB: la pieza callada). Un ventilador
/// fuerte o la tele lo pasan, y ahí el modelo corre siempre.
const PISO_DE_SILENCIO: f32 = 0.002;
/// Después de 3 s de silencio el modelo descansa. Con menos (1,2 s) se perdía
/// alguno dicho entre pausas cortas: al arrancar de cero puntúa un poco menos.
const PASOS_PARA_DORMIR: u32 = 38;
/// Al volver el sonido, el modelo arranca de cero con los últimos 2 s.
const MUESTRAS_DE_ARRANQUE: usize = FRECUENCIA_WHISPER as usize * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paso {
    /// Correr el modelo con este paso.
    Correr,
    /// Silencio largo: no correrlo.
    Dormir,
    /// Volvió el sonido: reiniciar el modelo, darle el último segundo y seguir.
    Despertar,
}

/// Decide, por el volumen de cada paso de 80 ms, si vale correr el modelo.
#[derive(Debug, Default)]
pub struct Compuerta {
    pasos_en_silencio: u32,
}

impl Compuerta {
    pub fn paso(&mut self, rms: f32) -> Paso {
        if rms >= PISO_DE_SILENCIO {
            let dormido = self.pasos_en_silencio > PASOS_PARA_DORMIR;
            self.pasos_en_silencio = 0;
            return if dormido { Paso::Despertar } else { Paso::Correr };
        }
        self.pasos_en_silencio = self.pasos_en_silencio.saturating_add(1);
        if self.pasos_en_silencio > PASOS_PARA_DORMIR { Paso::Dormir } else { Paso::Correr }
    }
}

/// El modelo con su disparador y el audio reciente.
pub struct Detector {
    modelo: WakeWordModel,
    disparador: Disparador,
    reciente: VecDeque<f32>,
    /// Para convertir a enteros de a pasos enteros.
    pendiente: Vec<i16>,
    compuerta: Compuerta,
    /// Cuántos pasos llegaron y en cuántos corrió el modelo (para medir).
    pub pasos: u64,
    pub pasos_corridos: u64,
}

impl Detector {
    /// Carga el modelo (`oye_nova.onnx`, ~7 MB; tarda ~0,3 s).
    pub fn cargar(ruta: &Path) -> Result<Self, String> {
        let modelo = WakeWordModel::new(&[ruta], FRECUENCIA_WHISPER)
            .map_err(|e| format!("No pude cargar el modelo de \"Oye Nova\": {e}"))?;
        Ok(Detector {
            modelo,
            disparador: Disparador::default(),
            reciente: VecDeque::with_capacity(MUESTRAS_GUARDADAS),
            pendiente: Vec::with_capacity(MUESTRAS_POR_PASO),
            compuerta: Compuerta::default(),
            pasos: 0,
            pasos_corridos: 0,
        })
    }

    /// Audio a 16 kHz, de cualquier largo. Devuelve una decisión si la hubo.
    pub fn oir(&mut self, muestras: &[f32]) -> Option<Decision> {
        for &m in muestras {
            if self.reciente.len() == MUESTRAS_GUARDADAS {
                self.reciente.pop_front();
            }
            self.reciente.push_back(m);
            self.pendiente.push((m.clamp(-1.0, 1.0) * 32767.0) as i16);
        }
        let mut decision = None;
        // De a un paso por vez: así cada puntaje pasa por el disparador.
        while self.pendiente.len() >= MUESTRAS_POR_PASO {
            let paso: Vec<i16> = self.pendiente.drain(..MUESTRAS_POR_PASO).collect();
            self.pasos += 1;
            let rms = (paso.iter().map(|&m| (f32::from(m) / 32768.0).powi(2)).sum::<f32>() / paso.len() as f32).sqrt();
            match self.compuerta.paso(rms) {
                Paso::Dormir => {
                    // En silencio el puntaje es ~0: no hace falta calcularlo.
                    self.disparador.puntaje(0.0);
                    continue;
                }
                Paso::Despertar => self.arrancar_de_cero(),
                Paso::Correr => {}
            }
            self.pasos_corridos += 1;
            let puntaje = self.puntuar(&paso);
            if let Some(d) = self.disparador.puntaje(puntaje) {
                decision = Some(d);
            }
        }
        decision
    }

    fn puntuar(&mut self, paso: &[i16]) -> f32 {
        match self.modelo.predict(paso) {
            Ok(p) => p.values().copied().fold(0.0, f32::max),
            Err(e) => {
                eprintln!("Voz: el detector de \"Oye Nova\" falló: {e}");
                0.0
            }
        }
    }

    /// Volvió el sonido después de un silencio: el modelo arranca limpio con
    /// los 2 s anteriores al paso actual (silencio, que es lo que esperaba).
    fn arrancar_de_cero(&mut self) {
        let _ = self.modelo.reset();
        // Lo que quedó en `pendiente` es posterior al paso actual: no va.
        let hasta = self.reciente.len().saturating_sub(self.pendiente.len() + MUESTRAS_POR_PASO);
        let desde = hasta.saturating_sub(MUESTRAS_DE_ARRANQUE);
        let previo: Vec<i16> =
            self.reciente.range(desde..hasta).map(|&m| (m.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
        self.puntuar(&previo);
    }

    /// Los últimos 2,5 s, para que Whisper confirme.
    pub fn audio_reciente(&self) -> Vec<f32> {
        self.reciente.iter().copied().collect()
    }

    /// Después de despertar (o al volver a esperar): empezar de cero.
    pub fn reiniciar(&mut self) {
        let _ = self.modelo.reset();
        self.disparador.reiniciar();
        self.reciente.clear();
        self.pendiente.clear();
        self.compuerta = Compuerta::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasar(d: &mut Disparador, puntajes: &[f32]) -> Vec<(usize, Decision)> {
        puntajes.iter().enumerate().filter_map(|(i, &p)| d.puntaje(p).map(|x| (i, x))).collect()
    }

    #[test]
    fn en_silencio_el_modelo_descansa() {
        let mut c = Compuerta::default();
        let silencio = PISO_DE_SILENCIO / 10.0;
        // El primer rato de silencio todavía corre (el puntaje llega tarde).
        for _ in 0..PASOS_PARA_DORMIR {
            assert_eq!(c.paso(silencio), Paso::Correr);
        }
        assert_eq!(c.paso(silencio), Paso::Dormir);
        assert_eq!(c.paso(silencio), Paso::Dormir);
        // Vuelve la voz: arranca de cero y sigue corriendo.
        assert_eq!(c.paso(0.02), Paso::Despertar);
        assert_eq!(c.paso(0.02), Paso::Correr);
        assert_eq!(c.paso(silencio), Paso::Correr);
    }

    #[test]
    fn un_oye_nova_claro_despierta_enseguida() {
        // Así subía con tu voz: 0,003 → 0,415 → 0,857.
        let mut d = Disparador::default();
        assert_eq!(pasar(&mut d, &[0.003, 0.415, 0.857, 0.278]), vec![(2, Decision::Despertar)]);
    }

    #[test]
    fn uno_dudoso_lo_confirma_whisper() {
        // El de 0,485 → 0,385 de la prueba: no llega a seguro.
        let mut d = Disparador::default();
        assert_eq!(pasar(&mut d, &[0.022, 0.485, 0.385, 0.002]), vec![(3, Decision::Confirmar)]);
    }

    #[test]
    fn la_charla_no_dispara() {
        let mut d = Disparador::default();
        assert!(pasar(&mut d, &[0.001, 0.05, 0.14, 0.3, 0.02, 0.0]).is_empty());
    }

    #[test]
    fn la_misma_frase_no_dispara_dos_veces() {
        // 0,789 y 0,755 seguidos (dos pasos de la misma frase): una sola vez.
        let mut d = Disparador::default();
        assert_eq!(pasar(&mut d, &[0.789, 0.755, 0.6, 0.2]).len(), 1);
        // Pasado el descanso, otra frase sí.
        let silencio = vec![0.0; PASOS_DE_DESCANSO as usize];
        pasar(&mut d, &silencio);
        assert_eq!(pasar(&mut d, &[0.8]), vec![(0, Decision::Despertar)]);
    }

    #[test]
    fn si_se_queda_arriba_del_umbral_igual_decide() {
        let mut d = Disparador::default();
        let largo = vec![0.5; 10];
        assert_eq!(pasar(&mut d, &largo), vec![(PASOS_DE_SUBIDA as usize, Decision::Confirmar)]);
    }

    /// Con el modelo de verdad y una grabación tuya (WAV mono a 16 kHz):
    /// `NOVA_GRABACION_PALABRA=ruta.wav NOVA_VECES_PALABRA=11 cargo test -- --ignored palabra`.
    /// Cuenta cuántas veces decide (seguras + a confirmar).
    #[test]
    #[ignore]
    fn detecta_en_una_grabacion() {
        let Ok(ruta) = std::env::var("NOVA_GRABACION_PALABRA") else { return };
        let veces: usize = std::env::var("NOVA_VECES_PALABRA").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
        let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA");
        let modelo = std::path::PathBuf::from(local).join("com.augustraver.nova/voz/modelos/oye_nova.onnx");
        let mut detector = Detector::cargar(&modelo).unwrap();
        let bytes = std::fs::read(&ruta).unwrap();
        // Encabezado WAV simple de 44 bytes, 16 bits.
        let muestras: Vec<f32> =
            bytes[44..].chunks_exact(2).map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0).collect();
        let mut decisiones = Vec::new();
        let reloj = std::time::Instant::now();
        for (i, trozo) in muestras.chunks(480).enumerate() {
            if let Some(d) = detector.oir(trozo) {
                decisiones.push((i * 480 / 16, d));
            }
        }
        println!("decisiones (ms, cuál): {decisiones:?}");
        println!("el modelo corrió en {} de {} pasos", detector.pasos_corridos, detector.pasos);
        let segundos = muestras.len() as f64 / 16_000.0;
        println!("con voz: {:.2} % de un núcleo", reloj.elapsed().as_secs_f64() * 100.0 / segundos);
        // Un minuto de pieza callada (ruido muy bajo): casi no tiene que gastar.
        let mut semilla: u32 = 7;
        let callado: Vec<f32> = (0..16_000 * 60)
            .map(|_| {
                semilla = semilla.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((semilla >> 16) as f32 / 65_536.0 - 0.5) * 0.001
            })
            .collect();
        let reloj = std::time::Instant::now();
        for trozo in callado.chunks(480) {
            detector.oir(trozo);
        }
        println!("en silencio: {:.2} % de un núcleo", reloj.elapsed().as_secs_f64() * 100.0 / 60.0);
        assert!(decisiones.len() >= veces * 8 / 10, "solo {} de {veces}", decisiones.len());
        assert!(decisiones.len() <= veces, "de más: {}", decisiones.len());
    }

    #[test]
    fn whisper_tiene_que_haber_entendido_nova() {
        assert!(confirma("Oye Nova."));
        assert!(confirma("oye, nova, ¿qué tengo hoy?"));
        assert!(confirma("Oye,Noba"));
        assert!(!confirma("Oye, nueva temporada"));
        assert!(!confirma("Novak Djokovic ganó"));
        assert!(!confirma(""));
    }
}
