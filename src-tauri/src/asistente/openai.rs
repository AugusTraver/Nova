//! Cliente para APIs "compatibles con OpenAI" (`/chat/completions`). Lo
//! hablan Groq y Ollama, y casi todos los demás proveedores.
//!
//! Armar el pedido y leer la respuesta son funciones puras (con tests); la
//! única parte que toca la red es `completar`.

use std::time::Duration;

use serde_json::{Value, json};

use super::mensajes::{Llamada, Mensaje, RespuestaModelo};
use super::proveedor::{ProveedorIa, TipoProveedor};
use crate::error::{Error, Resultado};

pub struct ClienteCompatible {
    http: reqwest::Client,
    tipo: TipoProveedor,
    base: String,
    modelo: String,
    api_key: Option<String>,
}

impl ClienteCompatible {
    pub fn nuevo(tipo: TipoProveedor, base: String, modelo: String, api_key: Option<String>) -> Resultado<Self> {
        if tipo.cuenta_llavero().is_some() && api_key.is_none() {
            return Err(Error::Validacion(format!(
                "Falta la API key de {}. Cargala en Ajustes.",
                tipo.nombre()
            )));
        }
        // Un modelo local puede tardar bastante en la primera respuesta
        // (tiene que cargarse en memoria).
        let espera = match tipo {
            TipoProveedor::Ollama => Duration::from_secs(180),
            TipoProveedor::Groq => Duration::from_secs(60),
        };
        let http = reqwest::Client::builder()
            .timeout(espera)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteCompatible { http, tipo, base, modelo, api_key })
    }

    /// Un pedido mínimo, sin herramientas, para "Probar conexión".
    pub async fn probar(&self) -> Resultado<()> {
        let mensajes = [Mensaje::Usuario("Respondé solo: ok".into())];
        self.pedir(armar_pedido(&self.modelo, &mensajes, None)).await.map(|_| ())
    }

    async fn pedir(&self, cuerpo: Value) -> Resultado<RespuestaModelo> {
        let mut pedido = self.http.post(format!("{}/chat/completions", self.base)).json(&cuerpo);
        if let Some(key) = &self.api_key {
            pedido = pedido.bearer_auth(key);
        }
        let respuesta = pedido.send().await.map_err(|e| self.error_de_red(&e))?;
        let estado = respuesta.status().as_u16();
        let texto = respuesta.text().await.map_err(|e| self.error_de_red(&e))?;
        if estado >= 400 {
            return Err(Error::Sistema(error_http(self.tipo, &self.modelo, estado, &texto)));
        }
        let json: Value = serde_json::from_str(&texto)
            .map_err(|_| Error::Sistema(format!("{} respondió algo que no entiendo.", self.tipo.nombre())))?;
        leer_respuesta(&json)
            .ok_or_else(|| Error::Sistema(format!("{} respondió algo que no entiendo.", self.tipo.nombre())))
    }

    fn error_de_red(&self, e: &reqwest::Error) -> Error {
        let texto = if e.is_timeout() {
            "La IA tardó demasiado en responder. Probá de nuevo.".to_string()
        } else {
            match self.tipo {
                TipoProveedor::Ollama => {
                    format!("No encuentro Ollama en {}. ¿Está abierto?", self.base.trim_end_matches("/v1"))
                }
                TipoProveedor::Groq => "No me puedo conectar con Groq. ¿Tenés internet?".to_string(),
            }
        };
        Error::Sistema(texto)
    }
}

impl ProveedorIa for ClienteCompatible {
    async fn completar(&self, mensajes: &[Mensaje], herramientas: &Value) -> Resultado<RespuestaModelo> {
        self.pedir(armar_pedido(&self.modelo, mensajes, Some(herramientas))).await
    }
}

/// El cuerpo del pedido en el formato de OpenAI.
pub fn armar_pedido(modelo: &str, mensajes: &[Mensaje], herramientas: Option<&Value>) -> Value {
    let mensajes: Vec<Value> = mensajes
        .iter()
        .map(|m| match m {
            Mensaje::Sistema(texto) => json!({ "role": "system", "content": texto }),
            Mensaje::Usuario(texto) => json!({ "role": "user", "content": texto }),
            Mensaje::Asistente { texto, llamadas } if llamadas.is_empty() => {
                json!({ "role": "assistant", "content": texto })
            }
            Mensaje::Asistente { texto, llamadas } => json!({
                "role": "assistant",
                "content": if texto.is_empty() { Value::Null } else { Value::String(texto.clone()) },
                "tool_calls": llamadas.iter().map(|l| json!({
                    "id": l.id,
                    "type": "function",
                    "function": { "name": l.nombre, "arguments": l.argumentos }
                })).collect::<Vec<_>>()
            }),
            Mensaje::Herramienta { llamada_id, nombre, contenido } => json!({
                "role": "tool", "tool_call_id": llamada_id, "name": nombre, "content": contenido
            }),
        })
        .collect();

    let mut cuerpo = json!({ "model": modelo, "messages": mensajes, "temperature": 0.3 });
    if let Some(herramientas) = herramientas {
        cuerpo["tools"] = herramientas.clone();
        cuerpo["tool_choice"] = json!("auto");
    }
    cuerpo
}

/// Lee `choices[0].message`. `None` si no tiene la forma esperada.
pub fn leer_respuesta(json: &Value) -> Option<RespuestaModelo> {
    let mensaje = json.get("choices")?.get(0)?.get("message")?;
    let texto = mensaje.get("content").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let llamadas = mensaje
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|lista| {
            lista
                .iter()
                .enumerate()
                .filter_map(|(i, l)| {
                    let funcion = l.get("function")?;
                    let nombre = funcion.get("name")?.as_str()?.to_string();
                    // Los argumentos suelen venir como texto JSON, pero algunos
                    // servidores mandan el objeto directo.
                    let argumentos = match funcion.get("arguments") {
                        Some(Value::String(texto)) => texto.clone(),
                        Some(Value::Null) | None => "{}".to_string(),
                        Some(otro) => otro.to_string(),
                    };
                    let id = l.get("id").and_then(Value::as_str).map(String::from).unwrap_or_else(|| format!("llamada_{i}"));
                    Some(Llamada { id, nombre, argumentos })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(RespuestaModelo { texto, llamadas })
}

/// Un error HTTP explicado para el usuario.
pub fn error_http(tipo: TipoProveedor, modelo: &str, estado: u16, cuerpo: &str) -> String {
    let proveedor = tipo.nombre();
    match estado {
        401 | 403 => format!("La API key de {proveedor} no es válida. Revisala en Ajustes."),
        404 => match tipo {
            TipoProveedor::Ollama => format!("Ollama no tiene el modelo «{modelo}». Bajalo con: ollama pull {modelo}"),
            TipoProveedor::Groq => format!("{proveedor} no tiene el modelo «{modelo}». Elegí otro en Ajustes."),
        },
        413 => "La conversación quedó muy larga. Borrá el historial en Ajustes.".to_string(),
        429 => format!("Llegué al límite del plan gratis de {proveedor}. Probá en un ratito."),
        400 if cuerpo.contains("tool_use_failed") || cuerpo.contains("tool call") => {
            "El modelo armó mal un pedido. Probá decírmelo de otra forma.".to_string()
        }
        400 if cuerpo.contains("does not support tools") => {
            format!("El modelo «{modelo}» no sabe usar herramientas. Elegí otro en Ajustes.")
        }
        500.. => format!("{proveedor} está con problemas. Probá en un rato."),
        _ => format!("{proveedor} respondió con un error ({estado})."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_el_pedido_con_herramientas_y_resultados() {
        let mensajes = vec![
            Mensaje::Sistema("reglas".into()),
            Mensaje::Usuario("hola".into()),
            Mensaje::Asistente {
                texto: String::new(),
                llamadas: vec![Llamada { id: "c1".into(), nombre: "listar_materias".into(), argumentos: "{}".into() }],
            },
            Mensaje::Herramienta { llamada_id: "c1".into(), nombre: "listar_materias".into(), contenido: "[]".into() },
            Mensaje::Asistente { texto: "No tenés.".into(), llamadas: vec![] },
        ];
        let herramientas = json!([{ "type": "function" }]);
        let cuerpo = armar_pedido("m", &mensajes, Some(&herramientas));
        assert_eq!(cuerpo["model"], "m");
        assert_eq!(cuerpo["tool_choice"], "auto");
        let m = cuerpo["messages"].as_array().unwrap();
        assert_eq!(m[0], json!({ "role": "system", "content": "reglas" }));
        assert_eq!(m[2]["content"], Value::Null);
        assert_eq!(m[2]["tool_calls"][0]["function"]["name"], "listar_materias");
        assert_eq!(m[3]["role"], "tool");
        assert_eq!(m[3]["tool_call_id"], "c1");
        assert_eq!(m[4], json!({ "role": "assistant", "content": "No tenés." }));

        let sin = armar_pedido("m", &mensajes[..2], None);
        assert!(sin.get("tools").is_none());
    }

    #[test]
    fn lee_texto_y_llamadas() {
        let json = json!({ "choices": [{ "message": {
            "role": "assistant", "content": null,
            "tool_calls": [
                { "id": "x1", "type": "function", "function": { "name": "crear_evento", "arguments": "{\"titulo\":\"A\"}" } },
                { "type": "function", "function": { "name": "listar_materias", "arguments": {} } }
            ]
        }}]});
        let r = leer_respuesta(&json).unwrap();
        assert_eq!(r.texto, "");
        assert_eq!(r.llamadas[0], Llamada { id: "x1".into(), nombre: "crear_evento".into(), argumentos: "{\"titulo\":\"A\"}".into() });
        assert_eq!(r.llamadas[1].id, "llamada_1");
        assert_eq!(r.llamadas[1].argumentos, "{}");

        let texto = json!({ "choices": [{ "message": { "content": "  Listo.  " } }] });
        assert_eq!(leer_respuesta(&texto).unwrap(), RespuestaModelo { texto: "Listo.".into(), llamadas: vec![] });
        assert!(leer_respuesta(&json!({ "error": "x" })).is_none());
    }

    #[test]
    fn explica_los_errores() {
        assert!(error_http(TipoProveedor::Groq, "m", 401, "").contains("API key de Groq no es válida"));
        assert!(error_http(TipoProveedor::Groq, "m", 429, "").contains("límite del plan gratis"));
        assert!(error_http(TipoProveedor::Ollama, "qwen3:8b", 404, "").contains("ollama pull qwen3:8b"));
        assert!(error_http(TipoProveedor::Groq, "m", 400, "{\"code\":\"tool_use_failed\"}").contains("armó mal"));
        assert!(error_http(TipoProveedor::Groq, "m", 503, "").contains("con problemas"));
    }

    #[test]
    fn groq_sin_key_no_arranca() {
        let Err(e) = ClienteCompatible::nuevo(TipoProveedor::Groq, "u".into(), "m".into(), None) else { panic!() };
        assert!(e.to_string().contains("Falta la API key de Groq"));
        assert!(ClienteCompatible::nuevo(TipoProveedor::Ollama, "u".into(), "m".into(), None).is_ok());
    }
}
