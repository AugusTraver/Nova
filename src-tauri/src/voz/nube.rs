//! Las voces por internet (Azure, ElevenLabs, OpenAI) detrás de una sola
//! forma de usarlas: texto → (frecuencia, audio mono). Lo que tienen en
//! común (el cliente HTTP, los errores en palabras, dónde va cada key) está
//! acá. Piper, la voz local, no pasa por acá: es el respaldo sin internet.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::AjustesVoz;
use super::azure::{self, Azure};
use super::elevenlabs::{self, ElevenLabs};
use super::openai::{self, OpenAi};
use crate::error::{Error, Resultado};

/// Más que esto y conviene hablar con Piper: la respuesta ya se hace esperar.
const ESPERA_MAXIMA: Duration = Duration::from_secs(6);

/// Con quién habla Nova.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Proveedor {
    /// Si falta la región o la key, habla con Piper (así andaba antes).
    #[default]
    Azure,
    ElevenLabs,
    OpenAi,
    /// Piper, sin internet.
    Local,
}

impl Proveedor {
    pub fn nombre(self) -> &'static str {
        match self {
            Proveedor::Azure => "Azure",
            Proveedor::ElevenLabs => "ElevenLabs",
            Proveedor::OpenAi => "OpenAI",
            Proveedor::Local => "la voz local",
        }
    }

    /// Dónde va su key en el llavero (la local no tiene).
    pub fn cuenta_llavero(self) -> Option<&'static str> {
        match self {
            Proveedor::Azure => Some(azure::CUENTA_LLAVERO),
            Proveedor::ElevenLabs => Some(elevenlabs::CUENTA_LLAVERO),
            Proveedor::OpenAi => Some(openai::CUENTA_LLAVERO),
            Proveedor::Local => None,
        }
    }

    /// Una revisión por arriba, para avisar si se pegó otra cosa. Azure son
    /// letras y números; ElevenLabs y OpenAI empiezan con `sk` y llevan
    /// guiones.
    pub fn key_valida(self, key: &str) -> bool {
        let caracteres = |extra: &[char]| key.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(&c));
        match self {
            Proveedor::Azure => (16..=200).contains(&key.len()) && caracteres(&[]),
            Proveedor::ElevenLabs => (20..=200).contains(&key.len()) && caracteres(&['_', '-']),
            Proveedor::OpenAi => key.starts_with("sk-") && (20..=300).contains(&key.len()) && caracteres(&['_', '-']),
            Proveedor::Local => false,
        }
    }

    pub const CON_KEY: [Proveedor; 3] = [Proveedor::Azure, Proveedor::ElevenLabs, Proveedor::OpenAi];
}

pub fn cliente_http() -> Resultado<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(ESPERA_MAXIMA)
        .user_agent("Nova")
        .build()
        .map_err(|e| Error::Sistema(e.to_string()))
}

pub fn error_de_red(quien: &str, e: &reqwest::Error) -> Error {
    Error::Sistema(if e.is_timeout() { format!("{quien} tardó demasiado.") } else { format!("No pude llegar a {quien}: {e}") })
}

/// El mensaje que viene en el cuerpo de un error, si se entiende: ElevenLabs
/// usa `{"detail":{"message":…}}` y OpenAI `{"error":{"message":…}}`.
pub fn mensaje_del_cuerpo(cuerpo: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(cuerpo).ok()?;
    let m = v["detail"]["message"].as_str().or(v["error"]["message"].as_str()).or(v["detail"].as_str())?;
    // Sin textos larguísimos en Ajustes.
    Some(m.chars().take(160).collect())
}

/// El error, en palabras para Ajustes.
pub fn explicar(quien: &str, estado: u16, cuerpo: &str) -> String {
    let base = match estado {
        401 | 403 => format!("La key de {quien} no es válida o no tiene permiso para esto."),
        402 => format!("A la cuenta de {quien} no le quedan créditos."),
        429 => format!("{quien} está limitando los pedidos (o se terminó lo del mes)."),
        _ => format!("{quien} respondió con un error ({estado})."),
    };
    match mensaje_del_cuerpo(cuerpo) {
        Some(m) => format!("{base} Dice: «{m}»"),
        None => base,
    }
}

/// Deja pasar la respuesta si salió bien; si no, la convierte en un error claro.
pub async fn explicar_respuesta(quien: &str, respuesta: reqwest::Response) -> Resultado<reqwest::Response> {
    let estado = respuesta.status();
    if estado.is_success() {
        return Ok(respuesta);
    }
    let cuerpo = respuesta.text().await.unwrap_or_default();
    Err(Error::Sistema(explicar(quien, estado.as_u16(), &cuerpo)))
}

/// Un cliente listo para hablar con una de las voces de internet.
#[derive(Clone)]
pub enum VozNube {
    Azure(Azure),
    ElevenLabs(ElevenLabs),
    OpenAi(OpenAi),
}

impl VozNube {
    pub async fn sintetizar(&self, texto: &str) -> Resultado<(u32, Vec<f32>)> {
        match self {
            VozNube::Azure(c) => c.sintetizar(texto).await,
            VozNube::ElevenLabs(c) => c.sintetizar(texto).await,
            VozNube::OpenAi(c) => c.sintetizar(texto).await,
        }
    }

    /// Arma el cliente de `proveedor` con los ajustes y su key del llavero.
    /// `Ok(None)` es la voz local. Si falta algo, el error dice qué.
    pub fn armar(proveedor: Proveedor, ajustes: &AjustesVoz) -> Resultado<Option<VozNube>> {
        let Some(cuenta) = proveedor.cuenta_llavero() else { return Ok(None) };
        let falta = |que: &str| Error::Validacion(format!("Falta {que} de {}.", proveedor.nombre()));
        let key = crate::llavero::leer(cuenta)?.ok_or_else(|| falta("la key"))?;
        let v = ajustes.velocidad;
        let cliente = match proveedor {
            Proveedor::Azure => {
                if ajustes.region_azure.is_empty() {
                    return Err(falta("la región"));
                }
                VozNube::Azure(Azure::new(key, ajustes.region_azure.clone(), ajustes.voz_azure.clone(), v)?)
            }
            Proveedor::ElevenLabs => {
                if ajustes.voz_eleven.is_empty() {
                    return Err(Error::Validacion("Elegí una voz de ElevenLabs.".into()));
                }
                VozNube::ElevenLabs(ElevenLabs::new(key, ajustes.voz_eleven.clone(), ajustes.modelo_eleven.clone(), v)?)
            }
            Proveedor::OpenAi => VozNube::OpenAi(OpenAi::new(key, ajustes.voz_openai.clone(), v)?),
            Proveedor::Local => return Ok(None),
        };
        Ok(Some(cliente))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_guarda_en_minusculas() {
        assert_eq!(serde_json::to_string(&Proveedor::ElevenLabs).unwrap(), "\"elevenlabs\"");
        assert_eq!(serde_json::from_str::<Proveedor>("\"openai\"").unwrap(), Proveedor::OpenAi);
    }

    #[test]
    fn revisa_las_keys_por_arriba() {
        assert!(Proveedor::Azure.key_valida("0123456789abcdef0123456789abcdef"));
        assert!(!Proveedor::Azure.key_valida("sk-algo-con-guiones-0000"));
        assert!(Proveedor::ElevenLabs.key_valida("sk_0123456789abcdef0123456789abcdef"));
        assert!(Proveedor::OpenAi.key_valida("sk-proj-AbC_123-xyz0123456789"));
        assert!(!Proveedor::OpenAi.key_valida("sk_0123456789abcdef0123456789abcdef"));
        assert!(!Proveedor::OpenAi.key_valida("sk-proj con espacios 0123456789"));
        assert!(!Proveedor::Local.key_valida("cualquiera"));
    }

    #[test]
    fn explica_los_errores() {
        assert_eq!(explicar("OpenAI", 401, ""), "La key de OpenAI no es válida o no tiene permiso para esto.");
        let eleven = r#"{"detail":{"status":"quota_exceeded","message":"This request exceeds your quota."}}"#;
        assert!(explicar("ElevenLabs", 401, eleven).ends_with("«This request exceeds your quota.»"));
        let openai = r#"{"error":{"message":"Invalid voice"}}"#;
        assert!(explicar("OpenAI", 400, openai).contains("(400)") && explicar("OpenAI", 400, openai).contains("Invalid voice"));
        assert_eq!(mensaje_del_cuerpo("<html>"), None);
    }
}
