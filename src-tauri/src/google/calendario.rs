//! La API de Google Calendar (v3), sobre el calendario principal. Solo
//! habla con Google: qué pedir y qué hacer con la respuesta lo decide
//! `google::sincronizar`.
//!
//! `ApiCalendar` es un trait (una interfaz) para que los tests de la
//! sincronización usen un Google de mentira en memoria.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use super::oauth::{self, Cliente};
use crate::dominio::google::EventoGoogle;
use crate::error::Error;
use crate::llavero;

const BASE: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";

/// Lo que puede salir mal, separado según lo que hace la sincronización.
#[derive(Debug, Clone, PartialEq)]
pub enum Falla {
    /// Sin internet o Google no contesta: se reintenta más tarde.
    SinRed(String),
    /// El permiso ya no sirve (o no hay): hay que reconectar.
    Permiso(String),
    /// El `syncToken` venció (410): hay que traer todo de nuevo.
    SyncVencido,
    /// El evento cambió en Google desde la última vez (412, `If-Match`).
    Conflicto,
    /// Ese evento no existe en Google (404 o 410).
    NoExiste,
    /// Cualquier otra respuesta de Google.
    Otra(String),
}

impl Falla {
    pub fn mensaje(&self) -> String {
        match self {
            Falla::SinRed(m) | Falla::Permiso(m) | Falla::Otra(m) => m.clone(),
            Falla::SyncVencido => "Google pidió volver a traer todo.".into(),
            Falla::Conflicto => "El evento cambió en Google mientras tanto.".into(),
            Falla::NoExiste => "Ese evento ya no está en Google.".into(),
        }
    }
}

impl From<Falla> for Error {
    fn from(f: Falla) -> Self {
        match f {
            Falla::Permiso(m) => Error::Validacion(m),
            otra => Error::Sistema(otra.mensaje()),
        }
    }
}

/// Una lista completa (todas las páginas juntas).
#[derive(Debug, Clone, Default)]
pub struct Lista {
    pub eventos: Vec<EventoGoogle>,
    /// Para pedir la próxima vez solo lo que cambió.
    pub sync_token: Option<String>,
    /// La zona del calendario ("America/Argentina/Buenos_Aires").
    pub zona: Option<String>,
}

pub trait ApiCalendar {
    /// Todo desde `desde` (primera vez) o lo que cambió desde `sync_token`.
    async fn listar(&self, sync_token: Option<&str>, desde: Option<&str>) -> Result<Lista, Falla>;
    /// Las repeticiones de una serie entre dos momentos (RFC 3339).
    async fn repeticiones(&self, serie: &str, desde: &str, hasta: &str) -> Result<Vec<EventoGoogle>, Falla>;
    async fn obtener(&self, id: &str) -> Result<EventoGoogle, Falla>;
    async fn crear(&self, cuerpo: &Value) -> Result<EventoGoogle, Falla>;
    /// `etag`: si viene, Google rechaza el cambio (412) si el evento cambió.
    async fn modificar(&self, id: &str, cuerpo: &Value, etag: Option<&str>) -> Result<EventoGoogle, Falla>;
    async fn borrar(&self, id: &str) -> Result<(), Falla>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagina {
    #[serde(default)]
    items: Vec<EventoGoogle>,
    next_page_token: Option<String>,
    next_sync_token: Option<String>,
    time_zone: Option<String>,
}

/// El cliente de verdad. Guarda el permiso corto en memoria y lo renueva
/// cuando vence.
pub struct ClienteCalendar {
    http: reqwest::Client,
    acceso: Mutex<Option<(String, Instant)>>,
}

impl ClienteCalendar {
    pub fn nuevo() -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("Nova/0.7 (app de escritorio personal)")
            .build()
            .map_err(|e| Error::Sistema(format!("No pude preparar la conexión: {e}")))?;
        Ok(ClienteCalendar { http, acceso: Mutex::new(None) })
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Guarda un permiso corto recién conseguido (al conectar).
    pub fn recordar_acceso(&self, token: String, dura_segundos: u64) {
        let vence = Instant::now() + Duration::from_secs(dura_segundos.saturating_sub(60));
        *self.acceso.lock().unwrap_or_else(|e| e.into_inner()) = Some((token, vence));
    }

    pub fn olvidar_acceso(&self) {
        *self.acceso.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    async fn acceso(&self) -> Result<String, Falla> {
        if let Some((token, vence)) = self.acceso.lock().unwrap_or_else(|e| e.into_inner()).clone()
            && Instant::now() < vence
        {
            return Ok(token);
        }
        let sin_conectar = || Falla::Permiso("Google Calendar no está conectado.".into());
        let cliente = Cliente::del_llavero().map_err(|e| Falla::Otra(e.to_string()))?.ok_or_else(sin_conectar)?;
        let permiso = llavero::leer(oauth::CUENTA_PERMISO).map_err(|e| Falla::Otra(e.to_string()))?.ok_or_else(sin_conectar)?;
        let tokens = oauth::renovar(&self.http, &cliente, &permiso).await.map_err(|e| match e {
            Error::Validacion(m) => Falla::Permiso(m),
            // Sin internet no se puede renovar: se reintenta más tarde.
            otro => Falla::SinRed(otro.to_string()),
        })?;
        self.recordar_acceso(tokens.access_token.clone(), tokens.expires_in.max(120));
        Ok(tokens.access_token)
    }

    fn direccion(&self, id: Option<&str>, extra: &[&str]) -> Result<url::Url, Falla> {
        let mut url = url::Url::parse(BASE).map_err(|e| Falla::Otra(e.to_string()))?;
        if let Ok(mut segmentos) = url.path_segments_mut() {
            if let Some(id) = id {
                segmentos.push(id);
            }
            segmentos.extend(extra);
        }
        Ok(url)
    }

    /// Un pedido con el permiso. Si Google dice 401 (venció antes de lo
    /// esperado), se renueva una vez y se reintenta.
    async fn pedir(&self, metodo: Method, url: url::Url, cuerpo: Option<&Value>, etag: Option<&str>) -> Result<Option<Value>, Falla> {
        for intento in 0..2 {
            let token = self.acceso().await?;
            let mut pedido = self.http.request(metodo.clone(), url.clone()).bearer_auth(&token);
            if let Some(c) = cuerpo {
                pedido = pedido.json(c);
            }
            if let Some(e) = etag {
                pedido = pedido.header("If-Match", e);
            }
            let respuesta = pedido.send().await.map_err(|e| Falla::SinRed(format!("No pude hablar con Google Calendar: {e}")))?;
            let estado = respuesta.status();
            if estado == StatusCode::UNAUTHORIZED && intento == 0 {
                self.olvidar_acceso();
                continue;
            }
            let texto = respuesta.text().await.map_err(|e| Falla::SinRed(e.to_string()))?;
            return match estado {
                s if s.is_success() => {
                    if texto.trim().is_empty() {
                        Ok(None)
                    } else {
                        serde_json::from_str(&texto).map(Some).map_err(|e| Falla::Otra(format!("Google contestó algo raro: {e}")))
                    }
                }
                StatusCode::NOT_FOUND => Err(Falla::NoExiste),
                StatusCode::GONE => Err(Falla::SyncVencido),
                StatusCode::PRECONDITION_FAILED => Err(Falla::Conflicto),
                StatusCode::UNAUTHORIZED => Err(Falla::Permiso(oauth::PERMISO_VENCIDO.into())),
                StatusCode::FORBIDDEN if texto.contains("insufficientPermissions") || texto.contains("ACCESS_TOKEN_SCOPE_INSUFFICIENT") => {
                    Err(Falla::Permiso("Nova no tiene permiso para tu calendario: volvé a conectar y aceptá el permiso.".into()))
                }
                // Límite de pedidos o Google con problemas: como sin red, se reintenta.
                StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => Err(Falla::SinRed("Google pidió esperar un rato.".into())),
                s if s.is_server_error() => Err(Falla::SinRed(format!("Google Calendar no anda ({s}).")) ),
                s => Err(Falla::Otra(format!("Google Calendar contestó {s}: {}", texto.chars().take(200).collect::<String>()))),
            };
        }
        Err(Falla::Permiso(oauth::PERMISO_VENCIDO.into()))
    }

    fn evento(valor: Option<Value>) -> Result<EventoGoogle, Falla> {
        serde_json::from_value(valor.unwrap_or_default()).map_err(|e| Falla::Otra(format!("Google contestó algo raro: {e}")))
    }

    async fn todas_las_paginas(&self, base: url::Url) -> Result<Lista, Falla> {
        let mut lista = Lista::default();
        let mut pagina_siguiente: Option<String> = None;
        // Tope por las dudas: 40 páginas de 250 son 10.000 eventos.
        for _ in 0..40 {
            let mut url = base.clone();
            if let Some(p) = &pagina_siguiente {
                url.query_pairs_mut().append_pair("pageToken", p);
            }
            let valor = self.pedir(Method::GET, url, None, None).await?.unwrap_or_default();
            let pagina: Pagina = serde_json::from_value(valor).map_err(|e| Falla::Otra(format!("Google contestó algo raro: {e}")))?;
            lista.eventos.extend(pagina.items);
            lista.zona = pagina.time_zone.or(lista.zona);
            if let Some(token) = pagina.next_sync_token {
                lista.sync_token = Some(token);
            }
            match pagina.next_page_token {
                Some(p) => pagina_siguiente = Some(p),
                None => return Ok(lista),
            }
        }
        Err(Falla::Otra("Google mandó demasiadas páginas.".into()))
    }
}

impl ApiCalendar for ClienteCalendar {
    async fn listar(&self, sync_token: Option<&str>, desde: Option<&str>) -> Result<Lista, Falla> {
        let mut url = self.direccion(None, &[])?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("maxResults", "250");
            // Las series como series (con su RRULE), no repetición por repetición.
            q.append_pair("singleEvents", "false");
            q.append_pair("showDeleted", "true");
            match (sync_token, desde) {
                // Con syncToken Google no acepta timeMin.
                (Some(token), _) => {
                    q.append_pair("syncToken", token);
                }
                (None, Some(desde)) => {
                    q.append_pair("timeMin", desde);
                }
                (None, None) => {}
            }
        }
        self.todas_las_paginas(url).await
    }

    async fn repeticiones(&self, serie: &str, desde: &str, hasta: &str) -> Result<Vec<EventoGoogle>, Falla> {
        let mut url = self.direccion(Some(serie), &["instances"])?;
        url.query_pairs_mut()
            .append_pair("maxResults", "250")
            .append_pair("timeMin", desde)
            .append_pair("timeMax", hasta);
        Ok(self.todas_las_paginas(url).await?.eventos)
    }

    async fn obtener(&self, id: &str) -> Result<EventoGoogle, Falla> {
        let url = self.direccion(Some(id), &[])?;
        Self::evento(self.pedir(Method::GET, url, None, None).await?)
    }

    async fn crear(&self, cuerpo: &Value) -> Result<EventoGoogle, Falla> {
        let url = self.direccion(None, &[])?;
        Self::evento(self.pedir(Method::POST, url, Some(cuerpo), None).await?)
    }

    async fn modificar(&self, id: &str, cuerpo: &Value, etag: Option<&str>) -> Result<EventoGoogle, Falla> {
        let url = self.direccion(Some(id), &[])?;
        Self::evento(self.pedir(Method::PATCH, url, Some(cuerpo), etag).await?)
    }

    async fn borrar(&self, id: &str) -> Result<(), Falla> {
        let url = self.direccion(Some(id), &[])?;
        match self.pedir(Method::DELETE, url, None, None).await {
            // Ya no estaba: es lo que queríamos.
            Ok(_) | Err(Falla::NoExiste | Falla::SyncVencido) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Con tu cuenta de verdad (necesita Google conectado desde Ajustes):
    /// `cargo test lista_mi_calendario -- --ignored --nocapture`. Solo lee.
    #[tokio::test]
    #[ignore]
    async fn lista_mi_calendario() {
        let api = ClienteCalendar::nuevo().unwrap();
        let hoy = chrono::Local::now().fixed_offset().to_rfc3339();
        let lista = api.listar(None, Some(&hoy)).await.unwrap();
        println!("Zona: {:?}. {} eventos desde hoy.", lista.zona, lista.eventos.len());
        for ev in lista.eventos.iter().take(10) {
            println!("- {:?} {:?}", ev.summary, ev.start);
        }
        assert!(lista.sync_token.is_some());
    }
}
