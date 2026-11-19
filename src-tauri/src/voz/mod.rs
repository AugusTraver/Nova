//! Voz (v0.5, SPEC 5.1): Nova escucha sola al abrir el panel y contesta
//! hablando. El micrófono y la transcripción son locales: tu audio nunca
//! sale de la compu. Para hablar usa la voz de internet que elijas (Azure,
//! ElevenLabs u OpenAI) y, sin eso o sin internet, Piper, que es local.
//!
//! - `audio`, `vad`, `texto`: cuentas puras con tests (remuestrear, detectar
//!   cuándo hablás, limpiar lo que entiende Whisper, partir en oraciones).
//! - `dispositivos`: micrófono y parlantes (cpal).
//! - `whisper`, `piper`: los programas de voz → texto y texto → voz.
//! - `azure`, `elevenlabs`, `openai`: las voces por internet; `nube`, lo
//!   que tienen en común.
//! - `sintetizador`: el hilo que convierte las frases en audio por adelantado.
//! - `modelos`: dónde están y cómo se bajan.
//! - `palabra`: "Oye Nova" con el panel cerrado (v0.6).
//! - `motor`: el hilo que junta todo.

pub mod audio;
pub mod azure;
pub mod dispositivos;
pub mod elevenlabs;
pub mod modelos;
pub mod motor;
pub mod nube;
pub mod openai;
pub mod palabra;
pub mod piper;
pub mod sintetizador;
pub mod sistema;
pub mod texto;
pub mod vad;
pub mod whisper;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, channel};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use modelos::Rutas;
use motor::{Motor, Orden};

/// Lo que se guarda en preferencias (clave `voz`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesVoz {
    /// Escuchar al abrir el panel y contestar hablando.
    pub activada: bool,
    /// 1 = normal; de 0.7 a 1.4.
    pub velocidad: f32,
    /// Leer el resumen del día la primera vez que se abre el panel en el día.
    pub leer_resumen: bool,
    /// Usar la placa de video para Whisper (si hay una NVIDIA).
    pub usar_gpu: bool,
    /// Con el panel cerrado, "Oye Nova" lo abre y empieza a escuchar.
    pub palabra_clave: bool,
    /// Desenchufada, "Oye Nova" descansa (cuida la batería).
    pub palabra_pausa_bateria: bool,
    /// El nombre del micrófono elegido ("" = el que tenga Windows por defecto).
    pub microfono: String,
    /// Región del recurso de Azure ("eastus"). Vacía = no usar Azure. La
    /// key no va acá: va al llavero.
    pub region_azure: String,
    /// La voz de Azure ("es-MX-DaliaNeural").
    pub voz_azure: String,
    /// Con quién habla. Lo guardado antes de esto no lo tiene: queda Azure,
    /// que sin región ni key habla con Piper, como antes.
    pub proveedor: nube::Proveedor,
    /// El ID de una voz de la cuenta de ElevenLabs (vacío = sin elegir).
    pub voz_eleven: String,
    pub modelo_eleven: String,
    /// La voz de OpenAI ("coral").
    pub voz_openai: String,
}

impl Default for AjustesVoz {
    fn default() -> Self {
        AjustesVoz {
            activada: true,
            velocidad: 1.0,
            leer_resumen: true,
            usar_gpu: true,
            palabra_clave: true,
            palabra_pausa_bateria: true,
            microfono: String::new(),
            region_azure: String::new(),
            voz_azure: azure::VOZ_POR_DEFECTO.into(),
            proveedor: nube::Proveedor::default(),
            voz_eleven: String::new(),
            modelo_eleven: elevenlabs::MODELO_POR_DEFECTO.into(),
            voz_openai: openai::VOZ_POR_DEFECTO.into(),
        }
    }
}

impl AjustesVoz {
    pub fn validar(mut self) -> Result<Self, String> {
        if !(0.7..=1.4).contains(&self.velocidad) {
            return Err("La velocidad tiene que estar entre 0,7 y 1,4.".into());
        }
        // "East US" o " eastus " también valen: Azure la escribe de las dos formas.
        self.region_azure = self.region_azure.trim().to_lowercase().replace(' ', "");
        if !self.region_azure.is_empty() && !azure::region_valida(&self.region_azure) {
            return Err("La región de Azure es una palabra como \"eastus\" o \"brazilsouth\".".into());
        }
        if !azure::voz_valida(&self.voz_azure) {
            return Err("Esa voz de Azure no existe.".into());
        }
        if !self.voz_eleven.is_empty() && !elevenlabs::voz_valida(&self.voz_eleven) {
            return Err("Esa voz de ElevenLabs no existe.".into());
        }
        if !elevenlabs::modelo_valido(&self.modelo_eleven) {
            return Err("Ese modelo de ElevenLabs no existe.".into());
        }
        if !openai::voz_valida(&self.voz_openai) {
            return Err("Esa voz de OpenAI no existe.".into());
        }
        Ok(self)
    }
}

/// El estado que guarda Tauri (`app.manage`): el canal para mandarle
/// órdenes al motor. El motor arranca la primera vez que se lo necesita.
pub struct Voz {
    canal: Mutex<Option<Sender<Orden>>>,
    pub rutas: Rutas,
    /// El panel se abrió por "Oye Nova" (no por el atajo): ahí no se lee el
    /// resumen del día, que taparía lo que estás pidiendo.
    despertado: Arc<AtomicBool>,
}

impl Voz {
    pub fn new(rutas: Rutas) -> Self {
        // Si Nova se cerró de golpe la vez anterior, pueden haber quedado
        // Whisper o Piper abiertos: se cierran.
        modelos::cerrar_huerfano(&rutas.archivo_pid("whisper"), "whisper-server.exe");
        modelos::cerrar_huerfano(&rutas.archivo_pid("piper"), "piper.exe");
        Voz { canal: Mutex::new(None), rutas, despertado: Arc::default() }
    }

    /// Al cerrar Nova: el motor cierra Whisper y Piper (espera hasta 2 s).
    pub fn apagar(&self) {
        let Ok(mut canal) = self.canal.lock() else { return };
        if let Some(tx) = canal.take() {
            let (listo, esperar) = channel();
            if tx.send(Orden::Salir(listo)).is_ok() {
                let _ = esperar.recv_timeout(std::time::Duration::from_secs(2));
            }
        }
    }

    /// Manda una orden; si el motor no existe, lo crea en un hilo nuevo.
    pub fn mandar(&self, app: &AppHandle, ajustes: &AjustesVoz, orden: Orden) {
        let Ok(mut canal) = self.canal.lock() else { return };
        if canal.is_none() {
            let (tx, rx) = channel();
            let (abrir, despertado, bandeja) = (app.clone(), self.despertado.clone(), app.clone());
            let motor = Motor::new(motor::avisador_de_tauri(app.clone()), self.rutas.clone(), ajustes.clone(), tx.clone())
                .al_despertar(Box::new(move || {
                    despertado.store(true, Ordering::SeqCst);
                    crate::ventana::mostrar(&abrir);
                }))
                .al_cambiar_espera(Box::new(move |escuchando| crate::bandeja::marcar_escucha(&bandeja, escuchando)));
            // Un hilo común (no async): el motor bloquea esperando audio y
            // hablando con los programas de voz.
            let creado = std::thread::Builder::new().name("nova-voz".into()).spawn(move || motor.correr(rx));
            if creado.is_err() {
                eprintln!("No pude arrancar el hilo de voz.");
                return;
            }
            *canal = Some(tx);
        }
        if let Some(tx) = canal.as_ref()
            && tx.send(orden).is_err()
        {
            // El motor se cayó: la próxima orden lo vuelve a crear.
            *canal = None;
        }
    }

    /// ¿El panel se acaba de abrir por "Oye Nova"? (y lo olvida).
    pub fn tomar_despertado(&self) -> bool {
        self.despertado.swap(false, Ordering::SeqCst)
    }

    pub fn modelos_listos(&self) -> bool {
        modelos::faltantes(&self.rutas, false).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajustes_por_defecto_y_validacion() {
        let a = AjustesVoz::default();
        assert!(a.activada && a.leer_resumen && a.usar_gpu);
        assert!(AjustesVoz { velocidad: 2.0, ..a.clone() }.validar().is_err());
        let leidos: AjustesVoz = serde_json::from_str(r#"{"velocidad":1.2}"#).unwrap();
        assert_eq!(leidos.velocidad, 1.2);
        assert!(leidos.activada);
        // Lo guardado antes de Azure sigue andando: sin región, voz por defecto.
        assert_eq!(leidos.region_azure, "");
        assert_eq!(leidos.voz_azure, "es-MX-DaliaNeural");
        // Y lo de antes de los otros proveedores, también.
        assert_eq!(leidos.proveedor, nube::Proveedor::Azure);
        assert_eq!(leidos.voz_openai, "coral");
        assert_eq!(leidos.modelo_eleven, "eleven_flash_v2_5");
    }

    #[test]
    fn valida_las_voces_de_los_otros() {
        let a = AjustesVoz::default();
        assert!(AjustesVoz { voz_openai: "robot".into(), ..a.clone() }.validar().is_err());
        assert!(AjustesVoz { voz_eleven: "../x".into(), ..a.clone() }.validar().is_err());
        assert!(AjustesVoz { modelo_eleven: "otro".into(), ..a.clone() }.validar().is_err());
        assert!(AjustesVoz { proveedor: nube::Proveedor::ElevenLabs, voz_eleven: "21m00Tcm4TlvDq8ikWAM".into(), ..a }.validar().is_ok());
    }

    #[test]
    fn region_de_azure() {
        let a = AjustesVoz::default();
        let limpia = AjustesVoz { region_azure: " East US ".into(), ..a.clone() }.validar().unwrap();
        assert_eq!(limpia.region_azure, "eastus");
        assert!(AjustesVoz { region_azure: "eastus.evil.com".into(), ..a.clone() }.validar().is_err());
        assert!(AjustesVoz { voz_azure: "Dalia".into(), ..a }.validar().is_err());
    }
}

/// Pruebas con los programas de verdad. Necesitan los modelos bajados, así
/// que no corren con `cargo test`: `cargo test voz::real -- --ignored`.
#[cfg(test)]
mod real {
    use super::audio::{FRECUENCIA_WHISPER, remuestrear, wav};
    use super::modelos::Rutas;
    use super::piper::Piper;
    use super::whisper::ServidorWhisper;

    fn rutas() -> Rutas {
        let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA");
        Rutas::new(std::path::PathBuf::from(local).join("com.augustraver.nova").join("voz"))
    }

    /// El motor entero: le llega "por el micrófono" (a 48 kHz, con ruido de
    /// habitación) un pedido hablado, y tiene que avisar el texto final.
    #[test]
    #[ignore]
    fn el_motor_escucha_un_pedido() {
        use super::motor::{EVENTO_ESTADO, EVENTO_FINAL, Motor, Orden};
        use super::dispositivos::DelMicrofono;
        use std::sync::{Arc, Mutex};

        let r = rutas();
        let (frecuencia, voz) = {
            let mut piper = Piper::abrir(&r, 1.0).unwrap();
            piper.sintetizar("Recordame comprar el regalo de Juan para el viernes.").unwrap()
        };
        let voz = remuestrear(&voz, frecuencia, 48_000);
        let ruido = |segundos: f32| -> Vec<f32> {
            (0..(48_000.0 * segundos) as usize).map(|i| ((i * 7919 % 1000) as f32 / 1000.0 - 0.5) * 0.004).collect()
        };
        let audio: Vec<f32> = [ruido(2.0), voz, ruido(2.0)].concat();

        let eventos: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
        let anotar = eventos.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let motor = Motor::new(
            Box::new(move |e, v| anotar.lock().unwrap().push((e.to_string(), v))),
            r,
            super::AjustesVoz::default(),
            tx.clone(),
        )
        .sin_microfono(48_000);
        let hilo = std::thread::spawn(move || motor.correr(rx));
        tx.send(Orden::Activar).unwrap();
        // De a 10 ms, como un micrófono.
        for tanda in audio.chunks(480) {
            tx.send(Orden::Microfono(DelMicrofono::Audio(tanda.to_vec()))).unwrap();
        }
        // Espera hasta que haya `cuantos` eventos que cumplan `cumple`.
        let esperar_eventos = |cuantos: usize, que: &str, cumple: &dyn Fn(&str, &serde_json::Value) -> bool| {
            let limite = std::time::Instant::now() + std::time::Duration::from_secs(60);
            loop {
                let hay: Vec<serde_json::Value> =
                    eventos.lock().unwrap().iter().filter(|(e, v)| cumple(e, v)).map(|(_, v)| v.clone()).collect();
                if hay.len() >= cuantos {
                    return hay;
                }
                assert!(std::time::Instant::now() < limite, "no llegó: {que}");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        };
        let es_final = |e: &str, _: &serde_json::Value| e == EVENTO_FINAL;
        let final_ = esperar_eventos(1, "el primer final", &es_final).remove(0);

        // Nova contesta hablando y después tiene que volver a escuchar: un
        // segundo pedido también se entiende (antes quedaba sorda).
        tx.send(Orden::Hablar("Listo.".into())).unwrap();
        let estado_es = |cual: &'static str| move |e: &str, v: &serde_json::Value| e == EVENTO_ESTADO && v["estado"] == cual;
        esperar_eventos(1, "hablando", &estado_es("hablando"));
        // "escuchando" ya apareció una vez al principio: se espera la segunda.
        esperar_eventos(2, "escuchando de nuevo", &estado_es("escuchando"));
        std::thread::sleep(std::time::Duration::from_millis(500));
        for tanda in audio.chunks(480) {
            tx.send(Orden::Microfono(DelMicrofono::Audio(tanda.to_vec()))).unwrap();
        }
        let segundo = esperar_eventos(2, "el segundo final", &es_final).remove(1);
        let segundo = crate::dominio::lenguaje::normalizar(segundo.as_str().unwrap());
        assert!(segundo.contains("regalo de juan"), "{segundo}");
        let estados: Vec<String> = eventos
            .lock()
            .unwrap()
            .iter()
            .filter(|(e, _)| e == EVENTO_ESTADO)
            .map(|(_, v)| v["estado"].as_str().unwrap_or("").to_string())
            .collect();
        let (listo, esperar) = std::sync::mpsc::channel();
        tx.send(Orden::Salir(listo)).unwrap();
        esperar.recv().unwrap();
        hilo.join().unwrap();

        let texto = crate::dominio::lenguaje::normalizar(final_.as_str().unwrap());
        assert!(texto.contains("regalo de juan") && texto.contains("viernes"), "{texto}");
        // Pasó por todos los estados, en orden.
        let orden: Vec<&str> = ["preparando", "escuchando", "oyendo", "transcribiendo", "pensando"].to_vec();
        let mut i = 0;
        for e in &estados {
            if i < orden.len() && e == orden[i] {
                i += 1;
            }
        }
        assert_eq!(i, orden.len(), "{estados:?}");
    }

    #[test]
    #[ignore]
    fn piper_habla_y_whisper_entiende() {
        let r = rutas();
        let mut piper = Piper::abrir(&r, 1.0).unwrap();
        let (frecuencia, audio) = piper.sintetizar("Tengo parcial de Análisis el jueves a las nueve.").unwrap();
        assert!(audio.len() > frecuencia as usize);
        // Una segunda frase con el mismo proceso (sin volver a cargar la voz).
        let inicio = std::time::Instant::now();
        piper.sintetizar("Dale.").unwrap();
        assert!(inicio.elapsed().as_millis() < 3000, "{:?}", inicio.elapsed());

        let a_16k = remuestrear(&audio, frecuencia, FRECUENCIA_WHISPER);
        if let Ok(destino) = std::env::var("NOVA_GUARDAR_WAV") {
            std::fs::write(destino, wav(&a_16k, FRECUENCIA_WHISPER)).unwrap();
        }
        let texto = tauri::async_runtime::block_on(async {
            let servidor = ServidorWhisper::arrancar(&r, true).await.unwrap();
            assert!(servidor.gpu);
            servidor.transcribir(wav(&a_16k, FRECUENCIA_WHISPER)).await.unwrap()
        });
        let normal = crate::dominio::lenguaje::normalizar(&texto);
        // Piper cambia un poco cada vez que habla y a veces Whisper le
        // erra a una palabra: se piden las importantes.
        assert!(normal.contains("analisis") && normal.contains("jueves"), "{texto}");
    }
}

/// El micrófono y los parlantes de verdad (`cargo test voz::dispositivos_reales -- --ignored`).
/// El parlante suena en silencio: solo se comprueba que avance.
#[cfg(test)]
mod dispositivos_reales {
    use super::dispositivos::{DelMicrofono, Microfono, Parlante};
    use std::time::Duration;

    #[test]
    #[ignore]
    fn el_microfono_entrega_audio() {
        let (tx, rx) = std::sync::mpsc::channel::<DelMicrofono>();
        let nombre = std::env::var("NOVA_MICROFONO").unwrap_or_default();
        let mic = Microfono::abrir(&nombre, tx, |m| m).unwrap();
        assert!(mic.frecuencia >= 16_000);
        let mut muestras = 0;
        let limite = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < limite {
            if let Ok(DelMicrofono::Audio(a)) = rx.recv_timeout(Duration::from_millis(100)) {
                muestras += a.len();
            }
        }
        // En 2 s tienen que haber llegado al menos 1,5 s de audio.
        assert!(muestras as f32 > mic.frecuencia as f32 * 1.5, "{muestras}");
    }

    #[test]
    #[ignore]
    fn el_parlante_avanza() {
        let p = Parlante::abrir().unwrap();
        p.encolar(&vec![0.0; p.frecuencia as usize / 2]);
        std::thread::sleep(Duration::from_millis(900));
        assert!(p.sonadas() >= u64::from(p.frecuencia) / 2 - 2000, "{}", p.sonadas());
        assert_eq!(p.pendientes(), 0);
    }
}
