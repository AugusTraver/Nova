//! El permiso de Google (OAuth 2.0 para apps de escritorio), como lo
//! recomienda Google: se abre el navegador, vos aceptás, y Google vuelve a
//! un puerto de esta compu (`http://127.0.0.1:<puerto>`) con un código que
//! Nova cambia por el permiso.
//!
//! PKCE: Nova inventa un secreto al azar (`verificador`), le manda a Google
//! solo su hash (`desafio`) y después, al cambiar el código, el secreto. Así,
//! si otro programa se quedara con el código, no le serviría.
//!
//! Lo que se guarda en el llavero: el ID y el secreto del cliente (los
//! cargás en Ajustes) y el permiso de largo plazo (`refresh_token`). El
//! permiso corto (`access_token`, dura una hora) vive solo en memoria.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, Resultado};
use crate::llavero;

pub const CUENTA_CLIENTE_ID: &str = "google_cliente_id";
pub const CUENTA_CLIENTE_SECRETO: &str = "google_cliente_secreto";
pub const CUENTA_PERMISO: &str = "google_permiso";

/// Solo los eventos de los calendarios que son tuyos (ni compartidos ni la
/// configuración de la cuenta).
pub const ALCANCE: &str = "https://www.googleapis.com/auth/calendar.events.owned";

const URL_AUTORIZAR: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const URL_TOKEN: &str = "https://oauth2.googleapis.com/token";
const URL_REVOCAR: &str = "https://oauth2.googleapis.com/revoke";

/// Cuánto se espera a que aceptes en el navegador.
pub const ESPERA_MAXIMA: Duration = Duration::from_secs(5 * 60);

/// El mensaje cuando Google ya no acepta el permiso guardado.
pub const PERMISO_VENCIDO: &str = "Google ya no acepta el permiso de Nova: volvé a conectar Google Calendar en Ajustes.";

#[derive(Clone)]
pub struct Cliente {
    pub id: String,
    pub secreto: String,
}

impl Cliente {
    pub fn del_llavero() -> Resultado<Option<Cliente>> {
        let id = llavero::leer(CUENTA_CLIENTE_ID)?;
        let secreto = llavero::leer(CUENTA_CLIENTE_SECRETO)?;
        Ok(id.zip(secreto).map(|(id, secreto)| Cliente { id, secreto }))
    }
}

// ---------- PKCE y la dirección (puro) ----------

/// Base64 "para URLs" sin relleno (`-` y `_` en vez de `+` y `/`), que es lo
/// que pide PKCE. Son 15 líneas: no vale la pena una librería.
pub fn base64_url(bytes: &[u8]) -> String {
    const LETRAS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut salida = String::with_capacity(bytes.len() * 4 / 3 + 3);
    for bloque in bytes.chunks(3) {
        let n = (u32::from(bloque[0]) << 16)
            | (u32::from(*bloque.get(1).unwrap_or(&0)) << 8)
            | u32::from(*bloque.get(2).unwrap_or(&0));
        let cuantos = bloque.len() + 1; // 1 byte → 2 letras, 2 → 3, 3 → 4
        for i in 0..cuantos {
            salida.push(LETRAS[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    salida
}

pub fn desafio(verificador: &str) -> String {
    base64_url(&Sha256::digest(verificador.as_bytes()))
}

fn al_azar(bytes: usize) -> Resultado<String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|e| Error::Sistema(format!("No pude generar números al azar: {e}")))?;
    Ok(base64_url(&buffer))
}

/// La dirección de Google donde aceptás. `access_type=offline` y
/// `prompt=consent` hacen que Google mande el permiso de largo plazo aunque
/// ya hayas aceptado antes.
pub fn direccion_autorizar(cliente_id: &str, redireccion: &str, desafio: &str, estado: &str) -> String {
    let consulta = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", cliente_id)
        .append_pair("redirect_uri", redireccion)
        .append_pair("response_type", "code")
        .append_pair("scope", ALCANCE)
        .append_pair("code_challenge", desafio)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", estado)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .finish();
    format!("{URL_AUTORIZAR}?{consulta}")
}

/// Lo que trae la vuelta de Google: `GET /?state=...&code=... HTTP/1.1`.
#[derive(Debug, PartialEq)]
pub enum Vuelta {
    Codigo(String),
    /// Tocaste "Cancelar" o Google devolvió un error.
    Rechazo(String),
    /// Otra cosa (el ícono de la pestaña, un pedido con otro `state`).
    Otra,
}

pub fn leer_vuelta(primera_linea: &str, estado: &str) -> Vuelta {
    let Some(ruta) = primera_linea.strip_prefix("GET ").and_then(|r| r.split_whitespace().next()) else {
        return Vuelta::Otra;
    };
    let Some((camino, consulta)) = ruta.split_once('?') else {
        return Vuelta::Otra;
    };
    if camino != "/" {
        return Vuelta::Otra;
    }
    let pares: Vec<(String, String)> = url::form_urlencoded::parse(consulta.as_bytes()).into_owned().collect();
    let valor = |clave: &str| pares.iter().find(|(k, _)| k == clave).map(|(_, v)| v.clone());
    if valor("state").as_deref() != Some(estado) {
        return Vuelta::Otra;
    }
    if let Some(error) = valor("error") {
        return Vuelta::Rechazo(error);
    }
    valor("code").filter(|c| !c.is_empty()).map_or(Vuelta::Otra, Vuelta::Codigo)
}

// ---------- El ida y vuelta con el navegador ----------

/// Un intento de conectar: el puerto ya abierto y los secretos de PKCE.
pub struct Intento {
    escucha: TcpListener,
    pub redireccion: String,
    pub direccion: String,
    verificador: String,
    estado: String,
}

impl Intento {
    /// Abre un puerto libre de esta compu (el sistema elige cuál: puerto 0)
    /// solo en 127.0.0.1, así nadie de la red lo puede usar.
    pub fn nuevo(cliente_id: &str) -> Resultado<Intento> {
        let escucha = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| Error::Sistema(format!("No pude abrir un puerto para la vuelta de Google: {e}")))?;
        let puerto = escucha.local_addr().map_err(|e| Error::Sistema(e.to_string()))?.port();
        let redireccion = format!("http://127.0.0.1:{puerto}/");
        let verificador = al_azar(64)?;
        let estado = al_azar(24)?;
        let direccion = direccion_autorizar(cliente_id, &redireccion, &desafio(&verificador), &estado);
        Ok(Intento { escucha, redireccion, direccion, verificador, estado })
    }

    /// Espera a que Google vuelva con el código. Bloquea el hilo: se llama
    /// con `spawn_blocking` (un hilo aparte para cosas que esperan).
    pub fn esperar_codigo(&self, cancelado: &dyn Fn() -> bool) -> Resultado<String> {
        self.escucha.set_nonblocking(true).map_err(|e| Error::Sistema(e.to_string()))?;
        let limite = Instant::now() + ESPERA_MAXIMA;
        while Instant::now() < limite {
            if cancelado() {
                return Err(Error::Validacion("Se canceló la conexión con Google.".into()));
            }
            match self.escucha.accept() {
                Ok((conexion, origen)) if origen.ip().is_loopback() => {
                    if let Some(resultado) = self.atender(conexion) {
                        return resultado;
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(150)),
                Err(e) => return Err(Error::Sistema(format!("Falló la vuelta de Google: {e}"))),
            }
        }
        Err(Error::Validacion("Pasaron 5 minutos sin respuesta de Google. Probá conectar de nuevo.".into()))
    }

    /// Lee un pedido del navegador y le contesta con una página corta.
    fn atender(&self, mut conexion: TcpStream) -> Option<Resultado<String>> {
        let _ = conexion.set_nonblocking(false);
        let _ = conexion.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buffer = [0u8; 4096];
        let leidos = conexion.read(&mut buffer).ok()?;
        let pedido = String::from_utf8_lossy(&buffer[..leidos]);
        let vuelta = leer_vuelta(pedido.lines().next().unwrap_or_default(), &self.estado);
        let (titulo, texto, resultado) = match vuelta {
            Vuelta::Codigo(codigo) => ("Listo", "Nova ya puede usar tu Google Calendar. Podés cerrar esta pestaña y volver.", Some(Ok(codigo))),
            Vuelta::Rechazo(_) => (
                "No se conectó",
                "No le diste permiso a Nova. Si fue sin querer, probá de nuevo desde Ajustes.",
                Some(Err(Error::Validacion("No le diste permiso a Nova en Google.".into()))),
            ),
            Vuelta::Otra => ("Nova", "", None),
        };
        let html = format!(
            "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><title>{titulo}</title>\
             <body style=\"font:18px/1.5 Georgia,serif;max-width:32em;margin:15vh auto;padding:0 16px;color:#23201c;background:#f6f3ee\">\
             <h1 style=\"font-weight:400\">{titulo}</h1><p>{texto}</p></body></html>"
        );
        let estado_http = if resultado.is_some() { "200 OK" } else { "404 Not Found" };
        let respuesta = format!(
            "HTTP/1.1 {estado_http}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        let _ = conexion.write_all(respuesta.as_bytes());
        resultado
    }

    pub fn verificador(&self) -> &str {
        &self.verificador
    }
}

// ---------- Hablar con Google ----------

#[derive(Debug, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct ErrorToken {
    error: String,
}

/// Arma un cuerpo `application/x-www-form-urlencoded` a mano (así no hace
/// falta la función de formularios de reqwest).
fn formulario(pares: &[(&str, &str)]) -> String {
    let mut s = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pares {
        s.append_pair(k, v);
    }
    s.finish()
}

async fn pedir_token(http: &reqwest::Client, pares: &[(&str, &str)]) -> Resultado<Tokens> {
    let respuesta = http
        .post(URL_TOKEN)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(formulario(pares))
        .send()
        .await
        .map_err(|e| Error::Sistema(format!("No pude hablar con Google: {e}")))?;
    let estado = respuesta.status();
    let texto = respuesta.text().await.map_err(|e| Error::Sistema(e.to_string()))?;
    if estado.is_success() {
        return serde_json::from_str(&texto).map_err(|e| Error::Sistema(format!("Google contestó algo raro: {e}")));
    }
    let error = serde_json::from_str::<ErrorToken>(&texto).map(|e| e.error).unwrap_or_default();
    Err(match error.as_str() {
        "invalid_grant" => Error::Validacion(PERMISO_VENCIDO.into()),
        "invalid_client" | "unauthorized_client" => {
            Error::Validacion("Google no reconoce el ID o el secreto del cliente. Revisalos en Ajustes.".into())
        }
        _ => Error::Sistema(format!("Google no dio el permiso ({estado}): {error}")),
    })
}

/// Cambia el código de la vuelta por los permisos.
pub async fn cambiar_codigo(http: &reqwest::Client, cliente: &Cliente, codigo: &str, intento: &Intento) -> Resultado<Tokens> {
    pedir_token(
        http,
        &[
            ("client_id", &cliente.id),
            ("client_secret", &cliente.secreto),
            ("code", codigo),
            ("code_verifier", intento.verificador()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &intento.redireccion),
        ],
    )
    .await
}

/// Un permiso corto nuevo a partir del de largo plazo.
pub async fn renovar(http: &reqwest::Client, cliente: &Cliente, permiso: &str) -> Resultado<Tokens> {
    pedir_token(
        http,
        &[
            ("client_id", &cliente.id),
            ("client_secret", &cliente.secreto),
            ("refresh_token", permiso),
            ("grant_type", "refresh_token"),
        ],
    )
    .await
}

/// Le avisa a Google que Nova ya no tiene permiso. Si falla (sin internet),
/// el permiso igual se borra del llavero; se puede quitar a mano en
/// myaccount.google.com/permissions.
pub async fn revocar(http: &reqwest::Client, permiso: &str) -> Resultado<()> {
    http.post(URL_REVOCAR)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(formulario(&[("token", permiso)]))
        .send()
        .await
        .map_err(|e| Error::Sistema(format!("No pude avisarle a Google: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_para_urls() {
        assert_eq!(base64_url(b""), "");
        assert_eq!(base64_url(b"f"), "Zg");
        assert_eq!(base64_url(b"fo"), "Zm8");
        assert_eq!(base64_url(b"foo"), "Zm9v");
        assert_eq!(base64_url(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn desafio_del_ejemplo_del_rfc() {
        // RFC 7636, apéndice B.
        assert_eq!(
            desafio("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn arma_la_direccion() {
        let d = direccion_autorizar("123.apps.googleusercontent.com", "http://127.0.0.1:5555/", "abc", "xyz");
        assert!(d.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=123.apps.googleusercontent.com&"));
        assert!(d.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5555%2F"));
        assert!(d.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.events.owned"));
        assert!(d.contains("code_challenge_method=S256"));
        assert!(d.contains("access_type=offline"));
    }

    #[test]
    fn lee_la_vuelta_de_google() {
        assert_eq!(leer_vuelta("GET /?state=xyz&code=4%2F0Ab HTTP/1.1", "xyz"), Vuelta::Codigo("4/0Ab".into()));
        assert_eq!(leer_vuelta("GET /?error=access_denied&state=xyz HTTP/1.1", "xyz"), Vuelta::Rechazo("access_denied".into()));
        // Otro `state`: puede ser un pedido viejo o de otro lado.
        assert_eq!(leer_vuelta("GET /?state=otro&code=abc HTTP/1.1", "xyz"), Vuelta::Otra);
        assert_eq!(leer_vuelta("GET /favicon.ico HTTP/1.1", "xyz"), Vuelta::Otra);
        assert_eq!(leer_vuelta("POST /?state=xyz&code=abc HTTP/1.1", "xyz"), Vuelta::Otra);
    }

    #[test]
    fn la_vuelta_llega_por_el_puerto() {
        let intento = Intento::nuevo("cliente").unwrap();
        assert!(intento.direccion.contains("code_challenge="));
        let estado = intento.estado.clone();
        let puerto = intento.escucha.local_addr().unwrap().port();
        let navegador = std::thread::spawn(move || {
            let mut c = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
            write!(c, "GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
            let mut c = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
            write!(c, "GET /?code=el-codigo&state={estado} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
            let mut respuesta = String::new();
            c.read_to_string(&mut respuesta).unwrap();
            respuesta
        });
        assert_eq!(intento.esperar_codigo(&|| false).unwrap(), "el-codigo");
        assert!(navegador.join().unwrap().contains("Podés cerrar esta pestaña"));
    }
}
