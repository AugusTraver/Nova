//! La voz de ElevenLabs: la más natural de las que probamos, y en su
//! biblioteca hay voces argentinas. Un pedido HTTPS con la key del usuario
//! (en el llavero) y vuelve PCM crudo a 24 kHz. Viaja solo el texto que dice
//! Nova, nunca el micrófono.
//!
//! Las voces se eligen de las que el usuario tiene en su cuenta ("Mis
//! voces"): las de la biblioteca hay que agregarlas ahí primero.

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::audio::leer_pcm16;
use super::nube::{cliente_http, error_de_red, explicar_respuesta};
use crate::error::{Error, Resultado};

pub const CUENTA_LLAVERO: &str = "elevenlabs-key";
const FRECUENCIA: u32 = 24_000;
/// El más rápido (unos 75 ms del lado de ElevenLabs).
pub const MODELO_RAPIDO: &str = "eleven_flash_v2_5";
/// Más natural y expresivo, pero tarda más en empezar.
pub const MODELO_NATURAL: &str = "eleven_multilingual_v2";
pub const MODELO_POR_DEFECTO: &str = MODELO_RAPIDO;

/// Los ID de voz son letras y números ("21m00Tcm4TlvDq8ikWAM").
pub fn voz_valida(voz: &str) -> bool {
    (10..=40).contains(&voz.len()) && voz.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn modelo_valido(modelo: &str) -> bool {
    [MODELO_RAPIDO, MODELO_NATURAL].contains(&modelo)
}

/// El cuerpo del pedido. ElevenLabs acepta velocidades entre 0,7 y 1,2.
/// Solo los modelos Flash y Turbo aceptan fijar el idioma; con los otros,
/// mandarlo es un error.
pub fn cuerpo(texto: &str, modelo: &str, velocidad: f32) -> serde_json::Value {
    let mut c = json!({
        "text": texto,
        "model_id": modelo,
        "voice_settings": { "stability": 0.45, "similarity_boost": 0.8, "speed": velocidad.clamp(0.7, 1.2) },
    });
    if modelo == MODELO_RAPIDO {
        c["language_code"] = json!("es");
    }
    c
}

/// Una voz de la cuenta, para elegir en Ajustes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VozDeCuenta {
    pub id: String,
    pub nombre: String,
    /// "argentine, female" o lo que diga ElevenLabs (puede venir vacío).
    pub detalle: String,
}

#[derive(Deserialize)]
struct ListaDeVoces {
    #[serde(rename = "voices")]
    voces: Vec<VozCruda>,
}

#[derive(Deserialize)]
struct VozCruda {
    voice_id: String,
    name: String,
    #[serde(default)]
    labels: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Lee la lista de voces que devuelve `GET /v1/voices`.
pub fn leer_voces(json: &str) -> Resultado<Vec<VozDeCuenta>> {
    let lista: ListaDeVoces = serde_json::from_str(json).map_err(|_| Error::Sistema("ElevenLabs mandó una lista de voces rara.".into()))?;
    Ok(lista
        .voces
        .into_iter()
        .map(|v| {
            let detalle = ["accent", "gender", "description"]
                .iter()
                .filter_map(|k| v.labels.get(*k).and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(str::to_string))
                .collect::<Vec<_>>()
                .join(", ");
            VozDeCuenta { id: v.voice_id, nombre: v.name, detalle }
        })
        .collect())
}

#[derive(Clone)]
pub struct ElevenLabs {
    http: reqwest::Client,
    key: String,
    pub voz: String,
    modelo: String,
    velocidad: f32,
}

impl ElevenLabs {
    pub fn new(key: String, voz: String, modelo: String, velocidad: f32) -> Resultado<Self> {
        Ok(ElevenLabs { http: cliente_http()?, key, voz, modelo, velocidad })
    }

    pub async fn sintetizar(&self, texto: &str) -> Resultado<(u32, Vec<f32>)> {
        let respuesta = self
            .http
            .post(format!("https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=pcm_24000", self.voz))
            .header("xi-api-key", &self.key)
            .json(&cuerpo(texto, &self.modelo, self.velocidad))
            .send()
            .await
            .map_err(|e| error_de_red("ElevenLabs", &e))?;
        let respuesta = explicar_respuesta("ElevenLabs", respuesta).await?;
        let bytes = respuesta.bytes().await.map_err(|e| Error::Sistema(format!("Se cortó la voz de ElevenLabs: {e}")))?;
        Ok((FRECUENCIA, leer_pcm16(&bytes)))
    }

    /// Las voces de la cuenta (las propias y las agregadas de la biblioteca).
    pub async fn voces(key: &str) -> Resultado<Vec<VozDeCuenta>> {
        let respuesta = cliente_http()?
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", key)
            .send()
            .await
            .map_err(|e| error_de_red("ElevenLabs", &e))?;
        let respuesta = explicar_respuesta("ElevenLabs", respuesta).await?;
        let texto = respuesta.text().await.map_err(|e| Error::Sistema(e.to_string()))?;
        leer_voces(&texto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_el_pedido() {
        let c = cuerpo("Listo, te lo anoto.", MODELO_RAPIDO, 1.0);
        assert_eq!(c["text"], "Listo, te lo anoto.");
        assert_eq!(c["model_id"], "eleven_flash_v2_5");
        assert_eq!(c["language_code"], "es");
        assert_eq!(c["voice_settings"]["speed"], 1.0);
        // El modelo natural no acepta el idioma, y la velocidad se recorta.
        let c = cuerpo("hola", MODELO_NATURAL, 1.4);
        assert!(c.get("language_code").is_none());
        assert!((c["voice_settings"]["speed"].as_f64().unwrap() - 1.2).abs() < 1e-6);
    }

    #[test]
    fn valida_voz_y_modelo() {
        assert!(voz_valida("21m00Tcm4TlvDq8ikWAM"));
        assert!(!voz_valida("") && !voz_valida("../../v1/user") && !voz_valida("abc"));
        assert!(modelo_valido("eleven_flash_v2_5") && !modelo_valido("eleven_otro"));
    }

    #[test]
    fn lee_las_voces_de_la_cuenta() {
        let json = r#"{"voices":[
            {"voice_id":"abc123abc123","name":"Valentina","labels":{"accent":"argentine","gender":"female"},"category":"professional"},
            {"voice_id":"xyz789xyz789","name":"Roger"}
        ]}"#;
        let voces = leer_voces(json).unwrap();
        assert_eq!(voces.len(), 2);
        assert_eq!(voces[0].nombre, "Valentina");
        assert_eq!(voces[0].detalle, "argentine, female");
        assert_eq!(voces[1].detalle, "");
        assert!(leer_voces("no").is_err());
    }

    /// Contra ElevenLabs de verdad, con la key del llavero:
    /// `cargo test -- --ignored elevenlabs`.
    #[test]
    #[ignore]
    fn elevenlabs_de_verdad() {
        let key = crate::llavero::leer(CUENTA_LLAVERO).unwrap().expect("falta la key de ElevenLabs en el llavero");
        let voces = tauri::async_runtime::block_on(ElevenLabs::voces(&key)).unwrap();
        let voz = voces.first().expect("la cuenta no tiene voces").id.clone();
        let cliente = ElevenLabs::new(key, voz, MODELO_RAPIDO.into(), 1.0).unwrap();
        let (frecuencia, audio) = tauri::async_runtime::block_on(cliente.sintetizar("Hola, soy Nova.")).unwrap();
        assert_eq!(frecuencia, 24_000);
        assert!(audio.len() > 12_000, "muy corto: {}", audio.len());
    }
}
