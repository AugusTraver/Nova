//! Cliente para APIs "compatibles con OpenAI" (`/chat/completions`). Lo
//! hablan Groq, Gemini y Ollama, y casi todos los demás proveedores.
//!
//! Armar el pedido y leer la respuesta son funciones puras (con tests); la
//! única parte que toca la red es `completar`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::cupo;
use super::mensajes::{Llamada, Mensaje, RespuestaModelo};
use super::proveedor::{ProveedorIa, TipoProveedor};
use crate::error::{Error, Resultado};

pub struct ClienteCompatible {
    http: reqwest::Client,
    tipo: TipoProveedor,
    base: String,
    modelo: String,
    api_key: Option<String>,
    /// Tokens gastados por este cliente (pedido + respuesta), para medir.
    /// `AtomicU64` porque `completar` recibe `&self`: se puede sumar sin `mut`.
    usados: AtomicU64,
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
            TipoProveedor::Groq | TipoProveedor::Gemini => Duration::from_secs(60),
        };
        let http = reqwest::Client::builder()
            .timeout(espera)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteCompatible { http, tipo, base, modelo, api_key, usados: AtomicU64::new(0) })
    }

    pub fn tipo(&self) -> TipoProveedor {
        self.tipo
    }

    /// Cuántos tokens gastó este cliente desde que se creó (para medir en
    /// los tests).
    #[cfg(test)]
    pub fn tokens_usados(&self) -> u64 {
        self.usados.load(Ordering::Relaxed)
    }

    /// Un pedido mínimo, sin herramientas, para "Probar conexión".
    pub async fn probar(&self) -> Resultado<()> {
        let mensajes = [Mensaje::Usuario("Respondé solo: ok".into())];
        self.pedir(armar_pedido(&self.modelo, &mensajes, None)).await.map(|_| ())
    }

    /// Un pedido sin herramientas que devuelve solo el texto (para tareas
    /// internas, como elegir noticias). Con gpt-oss pide razonar poco: es más
    /// rápido y gasta menos del límite de tokens.
    pub async fn texto(&self, mensajes: &[Mensaje]) -> Resultado<String> {
        let mut cuerpo = armar_pedido(&self.modelo, mensajes, None);
        if self.modelo.contains("gpt-oss") {
            cuerpo["reasoning_effort"] = json!("low");
        }
        self.pedir(cuerpo).await.map(|r| r.texto)
    }

    async fn pedir(&self, cuerpo: Value) -> Resultado<RespuestaModelo> {
        let mut pedido = self.http.post(format!("{}/chat/completions", self.base)).json(&cuerpo);
        if let Some(key) = &self.api_key {
            pedido = pedido.bearer_auth(key);
        }
        let respuesta = pedido.send().await.map_err(|e| self.error_de_red(&e))?;
        let estado = respuesta.status().as_u16();
        let encabezado = |nombre: &str| respuesta.headers().get(nombre).and_then(|v| v.to_str().ok()).map(String::from);
        let quedan = encabezado("x-ratelimit-remaining-tokens");
        let se_renueva = encabezado("x-ratelimit-reset-tokens");
        let reintentar = encabezado("retry-after");
        let texto = respuesta.text().await.map_err(|e| self.error_de_red(&e))?;
        if estado == 429 {
            eprintln!("IA: límite de {} ({}).", self.tipo.nombre(), texto.chars().take(300).collect::<String>());
            // Hasta que se libere, los pedidos van al respaldo (v0.7.3).
            let espera = segundos_de_espera(&texto, reintentar.as_deref()).unwrap_or(20);
            cupo::pausar(self.tipo, Duration::from_secs(espera));
            return Err(Error::Sistema(mensaje_de_limite(self.tipo, &texto, reintentar.as_deref())));
        }
        if estado >= 500 {
            cupo::pausar(self.tipo, Duration::from_secs(30));
        }
        if estado >= 400 {
            eprintln!("IA: error {estado} de {} ({}).", self.tipo.nombre(), texto.chars().take(400).collect::<String>());
            return Err(Error::Sistema(error_http(self.tipo, &self.modelo, estado, &texto)));
        }
        let json: Value = serde_json::from_str(&texto)
            .map_err(|_| Error::Sistema(format!("{} respondió algo que no entiendo.", self.tipo.nombre())))?;
        if let (Some(tokens), Some(en)) =
            (quedan.as_deref().and_then(|q| q.parse().ok()), se_renueva.as_deref().and_then(cupo::leer_duracion))
        {
            cupo::anotar_quedan(self.tipo, tokens, en);
        }
        if let Some(uso) = leer_uso(&json) {
            self.usados.fetch_add(uso.total(), Ordering::Relaxed);
            let quedan = quedan.map(|q| format!("; quedan {q} este minuto")).unwrap_or_default();
            eprintln!(
                "IA ({}): {} tokens ({} de pedido, {} de respuesta){quedan}.",
                self.tipo.nombre(),
                uso.total(),
                uso.pedido,
                uso.respuesta
            );
        }
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
                TipoProveedor::Groq | TipoProveedor::Gemini => {
                    format!("No me puedo conectar con {}. ¿Tenés internet?", self.tipo.nombre())
                }
            }
        };
        Error::Sistema(texto)
    }
}

impl ProveedorIa for ClienteCompatible {
    async fn completar(&self, mensajes: &[Mensaje], herramientas: &Value) -> Resultado<RespuestaModelo> {
        // Sin herramientas (un saludo) no se manda la lista: vacía, algunos
        // proveedores la rechazan.
        let herramientas = herramientas.as_array().is_some_and(|h| !h.is_empty()).then_some(herramientas);
        let mut cuerpo = armar_pedido(&self.modelo, mensajes, herramientas);
        if self.tipo == TipoProveedor::Gemini {
            firmar_llamadas_ajenas(&mut cuerpo);
        } else {
            quitar_firmas(&mut cuerpo);
        }
        match self.pedir(cuerpo.clone()).await {
            // gpt-oss a veces arma mal una llamada (Groq lo rechaza con
            // "tool_use_failed"). Casi siempre sale bien al segundo intento.
            Err(Error::Sistema(e)) if e.contains("armó mal") => {
                eprintln!("IA: el modelo armó mal una llamada; reintento.");
                self.pedir(cuerpo).await
            }
            otro => otro,
        }
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
                "tool_calls": llamadas.iter().map(|l| {
                    let mut llamada = json!({
                        "id": l.id,
                        "type": "function",
                        "function": { "name": l.nombre, "arguments": l.argumentos }
                    });
                    if let Some(extra) = &l.extra {
                        llamada["extra_content"] = extra.clone();
                    }
                    llamada
                }).collect::<Vec<_>>()
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

/// Gemini 3 "firma" cada llamada a una herramienta y, en la vuelta siguiente,
/// rechaza las que no traen la firma. Las que armó otro proveedor (Groq, antes
/// de pasarse al respaldo) no la tienen: Google acepta este valor para esos
/// casos.
pub fn firmar_llamadas_ajenas(cuerpo: &mut Value) {
    let Some(mensajes) = cuerpo.get_mut("messages").and_then(Value::as_array_mut) else { return };
    for llamada in mensajes.iter_mut().filter_map(|m| m.get_mut("tool_calls")).filter_map(Value::as_array_mut).flatten() {
        if llamada.get("extra_content").is_none() {
            llamada["extra_content"] = json!({ "google": { "thought_signature": "skip_thought_signature_validator" } });
        }
    }
}

/// Los demás no conocen `extra_content` y podrían rechazar el pedido.
pub fn quitar_firmas(cuerpo: &mut Value) {
    let Some(mensajes) = cuerpo.get_mut("messages").and_then(Value::as_array_mut) else { return };
    for llamada in mensajes.iter_mut().filter_map(|m| m.get_mut("tool_calls")).filter_map(Value::as_array_mut).flatten() {
        if let Some(objeto) = llamada.as_object_mut() {
            objeto.remove("extra_content");
        }
    }
}

/// Cuántos tokens va a gastar un pedido, más o menos: unos 4 caracteres por
/// token, más lo que suele ocupar la respuesta (gpt-oss razona antes de
/// contestar).
pub fn estimar_tokens(cuerpo: &Value) -> u64 {
    const RESPUESTA: u64 = 900;
    cuerpo.to_string().chars().count() as u64 / 4 + RESPUESTA
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
                    // Gemini firma sus llamadas y pide la firma de vuelta.
                    let extra = l.get("extra_content").filter(|e| !e.is_null()).cloned();
                    Some(Llamada { id, nombre, argumentos, extra })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(RespuestaModelo { texto, llamadas })
}

/// Los tokens de un pedido, según `usage` de la respuesta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uso {
    pub pedido: u64,
    pub respuesta: u64,
}

impl Uso {
    pub fn total(&self) -> u64 {
        self.pedido + self.respuesta
    }
}

pub fn leer_uso(json: &Value) -> Option<Uso> {
    let uso = json.get("usage")?;
    Some(Uso {
        pedido: uso.get("prompt_tokens")?.as_u64()?,
        respuesta: uso.get("completion_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
}

/// Cuántos segundos hay que esperar, según el encabezado `retry-after` o el
/// texto de Groq ("Please try again in 7.66s" o "in 1m2.5s").
pub fn segundos_de_espera(cuerpo: &str, retry_after: Option<&str>) -> Option<u64> {
    if let Some(s) = retry_after.and_then(|r| r.trim().parse::<f64>().ok()) {
        return Some(s.ceil() as u64);
    }
    // Groq dice "try again in 7.66s"; Gemini, "Please retry in 23.4s".
    let (inicio, frase) = ["try again in ", "retry in "].iter().find_map(|f| cuerpo.find(f).map(|i| (i, *f)))?;
    let resto = &cuerpo[inicio + frase.len()..];
    let fin = resto.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | 'h' | 'm' | 's'))).unwrap_or(resto.len());
    let mut total = 0.0;
    let mut numero = String::new();
    for c in resto[..fin].chars() {
        match c {
            'h' | 'm' | 's' => {
                let n: f64 = numero.parse().ok()?;
                total += n * match c {
                    'h' => 3600.0,
                    'm' => 60.0,
                    _ => 1.0,
                };
                numero.clear();
            }
            _ => numero.push(c),
        }
    }
    (total > 0.0).then(|| total.ceil() as u64)
}

/// El aviso cuando el proveedor dice que se pasó del límite (error 429).
/// Groq tiene dos: por minuto y por día. El del día cuenta las últimas 24
/// horas, así que se va liberando de a poco: también trae cuánto esperar.
pub fn mensaje_de_limite(tipo: TipoProveedor, cuerpo: &str, retry_after: Option<&str>) -> String {
    let proveedor = tipo.nombre();
    // Gemini lo dice en el nombre de la cuota ("…PerDayPerProjectPerModel…").
    let por_dia =
        cuerpo.contains("per day") || cuerpo.contains("(TPD)") || cuerpo.contains("(RPD)") || cuerpo.contains("PerDay");
    let cual = if por_dia { "del día" } else { "por minuto" };
    match segundos_de_espera(cuerpo, retry_after) {
        Some(s) if s <= 90 => format!("Llegué al límite {cual} de {proveedor}. Probá en {s} segundos."),
        Some(s) if s < 2 * 3600 => format!("Llegué al límite {cual} de {proveedor}. Probá en unos {} minutos.", s.div_ceil(60)),
        Some(s) => format!("Llegué al límite {cual} de {proveedor}. Probá en unas {} horas.", s.div_ceil(3600)),
        None => format!("Llegué al límite del plan gratis de {proveedor}. Probá en un ratito."),
    }
}

/// Un error HTTP explicado para el usuario.
pub fn error_http(tipo: TipoProveedor, modelo: &str, estado: u16, cuerpo: &str) -> String {
    let proveedor = tipo.nombre();
    match estado {
        401 | 403 => format!("La API key de {proveedor} no es válida. Revisala en Ajustes."),
        // Gemini contesta 400 cuando la key está mal.
        400 if cuerpo.contains("API key not valid") || cuerpo.contains("API_KEY_INVALID") => {
            format!("La API key de {proveedor} no es válida. Revisala en Ajustes.")
        }
        404 => match tipo {
            TipoProveedor::Ollama => format!("Ollama no tiene el modelo «{modelo}». Bajalo con: ollama pull {modelo}"),
            TipoProveedor::Groq | TipoProveedor::Gemini => {
                format!("{proveedor} no tiene el modelo «{modelo}». Elegí otro en Ajustes.")
            }
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
                llamadas: vec![Llamada { id: "c1".into(), nombre: "listar_materias".into(), argumentos: "{}".into(), extra: None }],
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
        assert_eq!(
            r.llamadas[0],
            Llamada { id: "x1".into(), nombre: "crear_evento".into(), argumentos: "{\"titulo\":\"A\"}".into(), extra: None }
        );
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
    fn lee_el_uso_de_tokens() {
        let json = json!({ "usage": { "prompt_tokens": 1200, "completion_tokens": 80, "total_tokens": 1280 } });
        assert_eq!(leer_uso(&json), Some(Uso { pedido: 1200, respuesta: 80 }));
        assert_eq!(leer_uso(&json).unwrap().total(), 1280);
        assert_eq!(leer_uso(&json!({})), None);
    }

    #[test]
    fn explica_el_limite_con_la_espera() {
        let minuto = r#"{"error":{"message":"Rate limit reached for model `openai/gpt-oss-120b` on tokens per minute (TPM): Limit 8000, Used 7000, Requested 2100. Please try again in 8.25s. Need more tokens?"}}"#;
        assert_eq!(segundos_de_espera(minuto, None), Some(9));
        assert_eq!(mensaje_de_limite(TipoProveedor::Groq, minuto, None), "Llegué al límite por minuto de Groq. Probá en 9 segundos.");
        assert_eq!(segundos_de_espera("Please try again in 1m2.5s.", None), Some(63));
        assert_eq!(segundos_de_espera("", Some("12")), Some(12));
        assert_eq!(segundos_de_espera("otra cosa", None), None);

        let dia = "Rate limit reached on tokens per day (TPD): Limit 200000. Please try again in 5m19.248s.";
        assert_eq!(mensaje_de_limite(TipoProveedor::Groq, dia, None), "Llegué al límite del día de Groq. Probá en unos 6 minutos.");
        assert!(mensaje_de_limite(TipoProveedor::Groq, "per day. try again in 3h2m", None).contains("unas 4 horas"));
        assert!(mensaje_de_limite(TipoProveedor::Groq, "", None).contains("en un ratito"));
    }

    #[test]
    fn entiende_los_limites_y_errores_de_gemini() {
        let minuto = r#"[{"error":{"code":429,"message":"You exceeded your current quota. Please retry in 23.4s.","status":"RESOURCE_EXHAUSTED","details":[{"quotaId":"GenerateRequestsPerMinutePerProjectPerModel-FreeTier"}]}}]"#;
        assert_eq!(segundos_de_espera(minuto, None), Some(24));
        assert_eq!(mensaje_de_limite(TipoProveedor::Gemini, minuto, None), "Llegué al límite por minuto de Gemini. Probá en 24 segundos.");
        let dia = r#"{"quotaId":"GenerateRequestsPerDayPerProjectPerModel-FreeTier","message":"Please retry in 5h1m"}"#;
        assert!(mensaje_de_limite(TipoProveedor::Gemini, dia, None).starts_with("Llegué al límite del día de Gemini"));
        let key = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#;
        assert!(error_http(TipoProveedor::Gemini, "m", 400, key).contains("API key de Gemini no es válida"));
        assert!(error_http(TipoProveedor::Gemini, "gemini-x", 404, "").contains("no tiene el modelo «gemini-x»"));
    }

    #[test]
    fn devuelve_la_firma_de_gemini_y_firma_las_ajenas() {
        let firma = json!({ "google": { "thought_signature": "abc" } });
        let json = json!({ "choices": [{ "message": { "tool_calls": [
            { "id": "g1", "function": { "name": "listar_materias", "arguments": "{}" }, "extra_content": firma }
        ]}}]});
        let llamada = leer_respuesta(&json).unwrap().llamadas.remove(0);
        assert_eq!(llamada.extra, Some(firma.clone()));

        let ajena = Llamada { id: "q1".into(), nombre: "listar_materias".into(), argumentos: "{}".into(), extra: None };
        let mensajes = vec![Mensaje::Asistente { texto: String::new(), llamadas: vec![llamada, ajena] }];
        let mut cuerpo = armar_pedido("m", &mensajes, None);
        assert_eq!(cuerpo["messages"][0]["tool_calls"][0]["extra_content"], firma);
        assert!(cuerpo["messages"][0]["tool_calls"][1].get("extra_content").is_none());
        firmar_llamadas_ajenas(&mut cuerpo);
        assert_eq!(cuerpo["messages"][0]["tool_calls"][0]["extra_content"], firma, "la propia no se toca");
        assert_eq!(
            cuerpo["messages"][0]["tool_calls"][1]["extra_content"]["google"]["thought_signature"],
            "skip_thought_signature_validator"
        );
        quitar_firmas(&mut cuerpo);
        assert!(cuerpo["messages"][0]["tool_calls"].as_array().unwrap().iter().all(|l| l.get("extra_content").is_none()));
    }

    #[test]
    fn estima_los_tokens_de_un_pedido() {
        let cuerpo = armar_pedido("m", &[Mensaje::Usuario("a".repeat(4000))], None);
        let estimado = estimar_tokens(&cuerpo);
        assert!((1900..2100).contains(&estimado), "{estimado}");
    }

    #[test]
    fn groq_sin_key_no_arranca() {
        let Err(e) = ClienteCompatible::nuevo(TipoProveedor::Groq, "u".into(), "m".into(), None) else { panic!() };
        assert!(e.to_string().contains("Falta la API key de Groq"));
        assert!(ClienteCompatible::nuevo(TipoProveedor::Ollama, "u".into(), "m".into(), None).is_ok());
    }
}
