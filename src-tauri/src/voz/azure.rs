//! La voz de Microsoft (Azure AI Speech): mucho más natural que Piper.
//!
//! Es el servicio oficial: un pedido HTTPS con la key del usuario (que vive
//! en el llavero) y un texto en SSML, y vuelve un WAV listo para sonar. Lo
//! que viaja es solo el texto que dice Nova; el micrófono no sale de la compu.
//! Si no hay internet o la key falla, el motor vuelve a Piper.

use super::audio::leer_wav;
use super::nube::{cliente_http, error_de_red};
use crate::error::{Error, Resultado};

/// Cuenta en el llavero para la key de Azure.
pub const CUENTA_LLAVERO: &str = "azure-speech-key";
/// La voz que se usa si no se elige otra.
pub const VOZ_POR_DEFECTO: &str = "es-MX-DaliaNeural";

/// Las regiones de Azure son palabras en minúscula ("eastus", "brazilsouth").
pub fn region_valida(region: &str) -> bool {
    (2..=30).contains(&region.len()) && region.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Los nombres de voz son como "es-MX-DaliaNeural", o las HD (alta
/// definición) como "es-MX-Ximena:DragonHDLatestNeural".
pub fn voz_valida(voz: &str) -> bool {
    let (nombre, modelo) = voz.split_once(':').unwrap_or((voz, ""));
    let partes: Vec<&str> = nombre.split('-').collect();
    partes.len() >= 3
        && voz.len() <= 70
        && voz.ends_with("Neural")
        && partes[0].len() == 2
        && (modelo.is_empty() || modelo.starts_with("DragonHD"))
        && nombre.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && modelo.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Las voces HD entonan solas según lo que dicen. No aceptan `<prosody>`:
/// con ellas la velocidad no se puede cambiar.
pub fn es_hd(voz: &str) -> bool {
    voz.contains(":DragonHD")
}

/// El idioma de la voz ("es-MX-DaliaNeural" → "es-MX").
fn idioma(voz: &str) -> String {
    voz.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")
}

/// SSML (el formato que pide Azure): el texto escapado, con la voz y la
/// velocidad. 1.0 = normal, 1.2 = "+20%".
pub fn ssml(texto: &str, voz: &str, velocidad: f32) -> String {
    let escapado = texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    if es_hd(voz) {
        return format!("<speak version='1.0' xml:lang='{idioma}'><voice name='{voz}'>{escapado}</voice></speak>", idioma = idioma(voz));
    }
    let porcentaje = ((velocidad - 1.0) * 100.0).round() as i32;
    format!(
        "<speak version='1.0' xml:lang='{idioma}'><voice name='{voz}'><prosody rate='{porcentaje:+}%'>{escapado}</prosody></voice></speak>",
        idioma = idioma(voz),
    )
}

/// El error, en palabras para Ajustes.
fn explicar(estado: u16) -> String {
    match estado {
        400 => "Azure no entendió el pedido (¿la voz existe en esa región?).".into(),
        401 | 403 => "La key de Azure no es válida o es de otra región.".into(),
        429 => "Azure está limitando los pedidos (o se terminó el plan gratis del mes).".into(),
        _ => format!("Azure respondió con un error ({estado})."),
    }
}

#[derive(Clone)]
pub struct Azure {
    http: reqwest::Client,
    key: String,
    region: String,
    pub voz: String,
    velocidad: f32,
}

impl Azure {
    pub fn new(key: String, region: String, voz: String, velocidad: f32) -> Resultado<Self> {
        Ok(Azure { http: cliente_http()?, key, region, voz, velocidad })
    }

    /// Texto → (frecuencia, audio mono).
    pub async fn sintetizar(&self, texto: &str) -> Resultado<(u32, Vec<f32>)> {
        let respuesta = self
            .http
            .post(format!("https://{}.tts.speech.microsoft.com/cognitiveservices/v1", self.region))
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .header("Content-Type", "application/ssml+xml")
            // WAV a 24 kHz: no hace falta decodificar MP3.
            .header("X-Microsoft-OutputFormat", "riff-24khz-16bit-mono-pcm")
            .body(ssml(texto, &self.voz, self.velocidad))
            .send()
            .await
            .map_err(|e| error_de_red("Azure", &e))?;
        let estado = respuesta.status();
        if !estado.is_success() {
            return Err(Error::Sistema(explicar(estado.as_u16())));
        }
        let bytes = respuesta.bytes().await.map_err(|e| Error::Sistema(format!("Se cortó la voz de Azure: {e}")))?;
        leer_wav(&bytes).map_err(Error::Sistema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_el_ssml() {
        assert_eq!(
            ssml("Listo, te lo anoto.", "es-MX-DaliaNeural", 1.0),
            "<speak version='1.0' xml:lang='es-MX'><voice name='es-MX-DaliaNeural'><prosody rate='+0%'>Listo, te lo anoto.</prosody></voice></speak>"
        );
        assert!(ssml("hola", "es-MX-DaliaNeural", 1.2).contains("rate='+20%'"));
        assert!(ssml("hola", "es-MX-DaliaNeural", 0.8).contains("rate='-20%'"));
    }

    #[test]
    fn escapa_lo_que_romperia_el_xml() {
        let s = ssml("Boca 2 < River 3 & \"todo\" bien </voice>", "es-ES-ElviraNeural", 1.0);
        assert!(s.contains("Boca 2 &lt; River 3 &amp; &quot;todo&quot; bien &lt;/voice&gt;"));
        assert!(s.contains("xml:lang='es-ES'"));
    }

    #[test]
    fn valida_region_y_voz() {
        assert!(region_valida("eastus") && region_valida("brazilsouth"));
        assert!(!region_valida("") && !region_valida("East US") && !region_valida("x.evil.com/"));
        assert!(voz_valida("es-MX-DaliaNeural") && voz_valida("es-US-PalomaNeural"));
        assert!(!voz_valida("Dalia") && !voz_valida("es-MX-Dalia'/><x") && !voz_valida("es-MX-Dalia"));
        assert!(voz_valida("es-MX-Ximena:DragonHDLatestNeural") && voz_valida("es-AR-ElenaNeural"));
        assert!(!voz_valida("es-MX-Ximena:Otra'Neural") && !voz_valida("es-MX-Ximena:CualquieraNeural"));
    }

    #[test]
    fn las_hd_van_sin_prosody() {
        let s = ssml("Hola", "es-MX-Ximena:DragonHDLatestNeural", 1.2);
        assert_eq!(s, "<speak version='1.0' xml:lang='es-MX'><voice name='es-MX-Ximena:DragonHDLatestNeural'>Hola</voice></speak>");
    }

    /// Contra Azure de verdad, con la key del llavero y la región en
    /// `NOVA_AZURE_REGION`. `cargo test -- --ignored azure`.
    #[test]
    #[ignore]
    fn azure_de_verdad() {
        let key = crate::llavero::leer(CUENTA_LLAVERO).unwrap().expect("falta la key de Azure en el llavero");
        let region = std::env::var("NOVA_AZURE_REGION").expect("falta NOVA_AZURE_REGION");
        let azure = Azure::new(key, region, VOZ_POR_DEFECTO.into(), 1.0).unwrap();
        let (frecuencia, audio) = tauri::async_runtime::block_on(azure.sintetizar("Hola, soy Nova.")).unwrap();
        assert_eq!(frecuencia, 24_000);
        assert!(audio.len() > 12_000, "muy corto: {}", audio.len());
    }
}
