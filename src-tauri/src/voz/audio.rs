//! Cuentas con audio, sin tocar el hardware (todo puro y con tests).
//!
//! El audio viaja como `f32` de -1 a 1 ("muestras"). Whisper quiere mono a
//! 16 kHz; el micrófono da lo que tenga (48 kHz estéreo, en general) y Piper
//! habla a 22 050 Hz. Acá están las conversiones entre esos formatos.

/// La frecuencia que entiende Whisper.
pub const FRECUENCIA_WHISPER: u32 = 16_000;

/// Junta los canales en uno (promedio). `canales` = 1 la deja igual.
pub fn a_mono(muestras: &[f32], canales: usize) -> Vec<f32> {
    if canales <= 1 {
        return muestras.to_vec();
    }
    muestras.chunks(canales).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect()
}

/// Cambia la frecuencia de muestreo de a pedazos (el micrófono entrega el
/// audio en tandas). Interpolación lineal: para voz alcanza y es barata.
///
/// Guarda la última muestra de la tanda anterior para que no haya un salto
/// entre tandas.
pub struct Remuestreador {
    paso: f64,
    /// Dónde cae la próxima muestra de salida, medido en muestras de entrada
    /// desde `anterior` (que es la posición -1... en la práctica, 0 a 1).
    posicion: f64,
    anterior: f32,
}

impl Remuestreador {
    pub fn new(desde: u32, hacia: u32) -> Self {
        Remuestreador { paso: f64::from(desde) / f64::from(hacia), posicion: 0.0, anterior: 0.0 }
    }

    pub fn procesar(&mut self, entrada: &[f32]) -> Vec<f32> {
        if (self.paso - 1.0).abs() < f64::EPSILON {
            return entrada.to_vec();
        }
        let mut salida = Vec::with_capacity((entrada.len() as f64 / self.paso) as usize + 1);
        // La muestra `i` de la tanda está en la posición i + 1 (la 0 es `anterior`).
        let valor = |i: usize| if i == 0 { self.anterior } else { entrada[i - 1] };
        let largo = entrada.len() as f64;
        while self.posicion < largo {
            let base = self.posicion.floor();
            let fraccion = (self.posicion - base) as f32;
            let i = base as usize;
            let a = valor(i);
            let b = valor(i + 1);
            salida.push(a + (b - a) * fraccion);
            self.posicion += self.paso;
        }
        self.posicion -= largo;
        if let Some(ultima) = entrada.last() {
            self.anterior = *ultima;
        }
        salida
    }
}

/// Remuestrea de una vez (para un audio entero, como lo que devuelve Piper).
pub fn remuestrear(muestras: &[f32], desde: u32, hacia: u32) -> Vec<f32> {
    Remuestreador::new(desde, hacia).procesar(muestras)
}

/// Raíz del promedio de los cuadrados: el "volumen" de un pedazo.
pub fn rms(muestras: &[f32]) -> f32 {
    if muestras.is_empty() {
        return 0.0;
    }
    (muestras.iter().map(|m| m * m).sum::<f32>() / muestras.len() as f32).sqrt()
}

/// El volumen pasado a una escala de 0 a 1 que se ve bien en la esfera
/// (logarítmica, como el oído): -60 dB o menos = 0, -12 dB o más = 1.
pub fn nivel_visual(rms: f32) -> f32 {
    if rms <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db + 60.0) / 48.0).clamp(0.0, 1.0)
}

/// El volumen de cada pedazo de `por_cuadro` muestras, en escala visual.
/// Sirve para animar la esfera al ritmo de un audio que ya se conoce entero.
pub fn envolvente(muestras: &[f32], por_cuadro: usize) -> Vec<f32> {
    muestras.chunks(por_cuadro.max(1)).map(|c| nivel_visual(rms(c))).collect()
}

/// Arma un WAV de 16 bits mono, como el que espera Whisper.
pub fn wav(muestras: &[f32], frecuencia: u32) -> Vec<u8> {
    let datos = (muestras.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + datos as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + datos).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // largo del bloque fmt
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&frecuencia.to_le_bytes());
    b.extend_from_slice(&(frecuencia * 2).to_le_bytes()); // bytes por segundo
    b.extend_from_slice(&2u16.to_le_bytes()); // bytes por muestra
    b.extend_from_slice(&16u16.to_le_bytes()); // bits
    b.extend_from_slice(b"data");
    b.extend_from_slice(&datos.to_le_bytes());
    for m in muestras {
        let v = (m.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// Lee un WAV PCM de 16 bits (lo que escribe Piper). Devuelve la frecuencia
/// y las muestras en mono. Busca el bloque `data` (puede haber otros antes).
pub fn leer_wav(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    let error = || "El audio de la voz salió mal.".to_string();
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(error());
    }
    let u16_en = |i: usize| bytes.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
    let u32_en = |i: usize| bytes.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    let mut i = 12;
    let (mut frecuencia, mut canales, mut bits) = (0u32, 1usize, 16u16);
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let largo = u32_en(i + 4).ok_or_else(error)? as usize;
        let cuerpo = i + 8;
        if id == b"fmt " {
            canales = usize::from(u16_en(cuerpo + 2).ok_or_else(error)?);
            frecuencia = u32_en(cuerpo + 4).ok_or_else(error)?;
            bits = u16_en(cuerpo + 14).ok_or_else(error)?;
        } else if id == b"data" {
            if bits != 16 || frecuencia == 0 {
                return Err(error());
            }
            let fin = (cuerpo + largo).min(bytes.len());
            let muestras: Vec<f32> = bytes[cuerpo..fin]
                .chunks_exact(2)
                .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / f32::from(i16::MAX))
                .collect();
            return Ok((frecuencia, a_mono(&muestras, canales)));
        }
        // Los bloques de largo impar llevan un byte de relleno.
        i = cuerpo + largo + (largo & 1);
    }
    Err(error())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(frecuencia: f32, muestreo: u32, segundos: f32, amplitud: f32) -> Vec<f32> {
        let n = (muestreo as f32 * segundos) as usize;
        (0..n).map(|i| (i as f32 / muestreo as f32 * frecuencia * std::f32::consts::TAU).sin() * amplitud).collect()
    }

    #[test]
    fn mono_promedia_canales() {
        assert_eq!(a_mono(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
        assert_eq!(a_mono(&[0.3, 0.1], 1), vec![0.3, 0.1]);
    }

    #[test]
    fn remuestrea_a_16k_por_tandas_sin_perder_muestras() {
        let entrada = seno(440.0, 48_000, 1.0, 0.5);
        let mut r = Remuestreador::new(48_000, 16_000);
        // En tandas de tamaños raros, como las del micrófono.
        let salida: Vec<f32> = entrada.chunks(441).flat_map(|c| r.procesar(c)).collect();
        assert!((salida.len() as i64 - 16_000).abs() <= 1, "{}", salida.len());
        // Sigue siendo el mismo tono: el volumen se conserva.
        assert!((rms(&salida) - rms(&entrada)).abs() < 0.01);
        // De una vez da lo mismo que por tandas.
        let junta = remuestrear(&entrada, 48_000, 16_000);
        assert_eq!(junta.len(), salida.len());
        // Subir también anda (Piper 22 050 → parlantes 48 000).
        assert!((remuestrear(&seno(200.0, 22_050, 1.0, 0.5), 22_050, 48_000).len() as i64 - 48_000).abs() <= 1);
    }

    #[test]
    fn niveles() {
        assert_eq!(rms(&[]), 0.0);
        assert!((rms(&[0.5, -0.5]) - 0.5).abs() < 1e-6);
        assert_eq!(nivel_visual(0.0), 0.0);
        assert_eq!(nivel_visual(0.0005), 0.0); // -66 dB: silencio
        assert_eq!(nivel_visual(0.5), 1.0); // -6 dB: fuerte
        let medio = nivel_visual(0.01); // -40 dB
        assert!(medio > 0.3 && medio < 0.5);
        assert_eq!(envolvente(&[0.5; 10], 4).len(), 3);
    }

    #[test]
    fn wav_de_ida_y_vuelta() {
        let original = seno(300.0, 16_000, 0.1, 0.4);
        let bytes = wav(&original, 16_000);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + original.len() * 2);
        let (frecuencia, leidas) = leer_wav(&bytes).unwrap();
        assert_eq!(frecuencia, 16_000);
        assert_eq!(leidas.len(), original.len());
        assert!(leidas.iter().zip(&original).all(|(a, b)| (a - b).abs() < 0.001));
        assert!(leer_wav(b"no es un wav").is_err());
    }
}
