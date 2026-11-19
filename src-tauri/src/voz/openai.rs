//! La voz de OpenAI (`gpt-4o-mini-tts`). Lo bueno: se le puede pedir cómo
//! hablar con instrucciones en palabras (acento, tono, ritmo). Un pedido
//! HTTPS con la key del usuario (en el llavero) y vuelve PCM crudo a
//! 24 kHz. Viaja solo el texto que dice Nova, nunca el micrófono.

use serde_json::json;

use super::audio::leer_pcm16;
use super::nube::{cliente_http, error_de_red, explicar_respuesta};
use crate::error::{Error, Resultado};

pub const CUENTA_LLAVERO: &str = "openai-tts-key";
const FRECUENCIA: u32 = 24_000;
const MODELO: &str = "gpt-4o-mini-tts";
pub const VOZ_POR_DEFECTO: &str = "coral";
/// Las voces que acepta el modelo.
pub const VOCES: &[&str] = &["alloy", "ash", "ballad", "cedar", "coral", "echo", "fable", "marin", "nova", "onyx", "sage", "shimmer", "verse"];

/// Cómo tiene que sonar Nova. El modelo no acepta velocidad junto con
/// instrucciones, así que el ritmo también va acá.
pub fn instrucciones(velocidad: f32) -> String {
    let ritmo = if velocidad >= 1.15 {
        "Hablá bastante rápido, con energía."
    } else if velocidad >= 1.05 {
        "Hablá un poco más rápido de lo normal."
    } else if velocidad <= 0.85 {
        "Hablá despacio y con calma."
    } else if velocidad <= 0.95 {
        "Hablá un poco más despacio de lo normal."
    } else {
        "Hablá a un ritmo natural de charla."
    };
    format!(
        "Acento: español rioplatense de Buenos Aires, Argentina, con voseo y la «ll» y la «y» pronunciadas como «sh». \
         Tono: cálido, cercano y relajado, como una amiga que te ayuda a organizar el día. \
         Natural y conversacional, sin sonar a locutora ni exagerar. {ritmo}"
    )
}

pub fn voz_valida(voz: &str) -> bool {
    VOCES.contains(&voz)
}

pub fn cuerpo(texto: &str, voz: &str, velocidad: f32) -> serde_json::Value {
    json!({
        "model": MODELO,
        "input": texto,
        "voice": voz,
        "instructions": instrucciones(velocidad),
        "response_format": "pcm",
    })
}

#[derive(Clone)]
pub struct OpenAi {
    http: reqwest::Client,
    key: String,
    pub voz: String,
    velocidad: f32,
}

impl OpenAi {
    pub fn new(key: String, voz: String, velocidad: f32) -> Resultado<Self> {
        Ok(OpenAi { http: cliente_http()?, key, voz, velocidad })
    }

    pub async fn sintetizar(&self, texto: &str) -> Resultado<(u32, Vec<f32>)> {
        let respuesta = self
            .http
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.key)
            .json(&cuerpo(texto, &self.voz, self.velocidad))
            .send()
            .await
            .map_err(|e| error_de_red("OpenAI", &e))?;
        let respuesta = explicar_respuesta("OpenAI", respuesta).await?;
        let bytes = respuesta.bytes().await.map_err(|e| Error::Sistema(format!("Se cortó la voz de OpenAI: {e}")))?;
        Ok((FRECUENCIA, leer_pcm16(&bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_el_pedido() {
        let c = cuerpo("Listo, te lo anoto.", "coral", 1.0);
        assert_eq!(c["model"], "gpt-4o-mini-tts");
        assert_eq!(c["voice"], "coral");
        assert_eq!(c["response_format"], "pcm");
        assert!(c["instructions"].as_str().unwrap().contains("rioplatense"));
    }

    #[test]
    fn el_ritmo_va_en_las_instrucciones() {
        assert!(instrucciones(1.0).contains("ritmo natural"));
        assert!(instrucciones(1.3).contains("bastante rápido"));
        assert!(instrucciones(0.7).contains("despacio y con calma"));
    }

    #[test]
    fn valida_la_voz() {
        assert!(voz_valida("coral") && voz_valida("marin"));
        assert!(!voz_valida("Coral") && !voz_valida(""));
    }

    /// Contra OpenAI de verdad, con la key del llavero:
    /// `cargo test -- --ignored openai`.
    #[test]
    #[ignore]
    fn openai_de_verdad() {
        let key = crate::llavero::leer(CUENTA_LLAVERO).unwrap().expect("falta la key de OpenAI en el llavero");
        let cliente = OpenAi::new(key, VOZ_POR_DEFECTO.into(), 1.0).unwrap();
        let (frecuencia, audio) = tauri::async_runtime::block_on(cliente.sintetizar("Hola, soy Nova.")).unwrap();
        assert_eq!(frecuencia, 24_000);
        assert!(audio.len() > 12_000, "muy corto: {}", audio.len());
    }
}
