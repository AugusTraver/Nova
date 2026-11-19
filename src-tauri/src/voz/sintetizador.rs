//! Convierte las frases en audio en un hilo aparte, por adelantado.
//!
//! Antes el motor sintetizaba la frase siguiente recién cuando a la actual
//! le quedaba menos de un segundo, y mientras esperaba a internet no podía
//! atender nada (ni un `Esc`). Ahora el motor le pasa todas las frases de
//! una respuesta de una vez, sigue atendiendo, y el audio le vuelve por su
//! canal de órdenes a medida que está, en orden.
//!
//! Cada respuesta lleva un número (`habla`). Callarse sube el número
//! vigente: lo que estaba en la fila con el número viejo se saltea sin
//! gastar un pedido a internet.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use super::AjustesVoz;
use super::modelos::Rutas;
use super::nube::VozNube;
use super::piper::Piper;
use crate::error::{Error, Resultado};

/// Si la voz de internet falla (sin internet, key mala), se habla con Piper
/// este rato antes de volver a probar: así no se espera el error en cada frase.
const PAUSA_DE_LA_NUBE: Duration = Duration::from_secs(120);

pub type Audio = (u32, Vec<f32>);
/// Cómo devuelve cada frase: el número de la respuesta y el audio (o el error).
pub type AlSintetizar = Box<dyn Fn(u64, Resultado<Audio>) + Send>;

enum Pedido {
    Frase { habla: u64, texto: String },
    Configurar(AjustesVoz),
}

pub struct Sintetizador {
    pedidos: Sender<Pedido>,
    vigente: Arc<AtomicU64>,
}

impl Sintetizador {
    /// Arranca el hilo. Se termina solo cuando se suelta el `Sintetizador`
    /// (se cierra el canal) y al terminar cierra Piper.
    pub fn arrancar(rutas: Rutas, ajustes: AjustesVoz, al_sintetizar: AlSintetizar) -> Resultado<Self> {
        let (pedidos, rx) = channel();
        let vigente = Arc::new(AtomicU64::new(0));
        let trabajo = Trabajo {
            rutas,
            ajustes,
            piper: None,
            nube: None,
            nube_cargada: false,
            nube_en_pausa_hasta: Instant::now(),
            vigente: vigente.clone(),
            al_sintetizar,
        };
        std::thread::Builder::new()
            .name("nova-sintesis".into())
            .spawn(move || trabajo.correr(rx))
            .map_err(|e| Error::Sistema(format!("No pude arrancar el hilo de la voz: {e}")))?;
        Ok(Sintetizador { pedidos, vigente })
    }

    /// Manda las frases de una respuesta nueva. Devuelve su número.
    pub fn decir(&self, frases: impl IntoIterator<Item = String>) -> u64 {
        let habla = self.vigente.fetch_add(1, Ordering::SeqCst) + 1;
        for texto in frases {
            let _ = self.pedidos.send(Pedido::Frase { habla, texto });
        }
        habla
    }

    /// Lo que falte de la respuesta actual ya no se sintetiza.
    pub fn cancelar(&self) {
        self.vigente.fetch_add(1, Ordering::SeqCst);
    }

    pub fn configurar(&self, ajustes: AjustesVoz) {
        let _ = self.pedidos.send(Pedido::Configurar(ajustes));
    }
}

struct Trabajo {
    rutas: Rutas,
    ajustes: AjustesVoz,
    piper: Option<Piper>,
    /// La voz de internet elegida, si está todo cargado. Se arma la primera
    /// vez que hace falta y se rearma cuando cambian los ajustes.
    nube: Option<VozNube>,
    nube_cargada: bool,
    nube_en_pausa_hasta: Instant,
    vigente: Arc<AtomicU64>,
    al_sintetizar: AlSintetizar,
}

impl Trabajo {
    fn correr(mut self, pedidos: Receiver<Pedido>) {
        // `recv` falla cuando se soltó el `Sintetizador`: ahí termina el hilo
        // y, al soltarse `self`, se cierra Piper.
        while let Ok(pedido) = pedidos.recv() {
            match pedido {
                Pedido::Configurar(ajustes) => {
                    if (ajustes.velocidad - self.ajustes.velocidad).abs() > f32::EPSILON {
                        self.piper = None; // se reabre con la velocidad nueva
                    }
                    self.ajustes = ajustes;
                    // Proveedor, voz o key nuevos: se rearma y se vuelve a probar ya.
                    self.nube_cargada = false;
                    self.nube_en_pausa_hasta = Instant::now();
                }
                Pedido::Frase { habla, texto } => {
                    if habla != self.vigente.load(Ordering::SeqCst) {
                        continue; // se calló: ni la sintetiza
                    }
                    let resultado = self.sintetizar(&texto);
                    (self.al_sintetizar)(habla, resultado);
                }
            }
        }
    }

    fn sintetizar(&mut self, frase: &str) -> Resultado<Audio> {
        if !self.nube_cargada {
            self.nube_cargada = true;
            // Si falta algo (key, región, voz), se habla con Piper sin avisar:
            // Ajustes ya lo muestra al probar.
            self.nube = VozNube::armar(self.ajustes.proveedor, &self.ajustes).ok().flatten();
        }
        if let Some(nube) = self.nube.as_ref().filter(|_| Instant::now() >= self.nube_en_pausa_hasta) {
            match tauri::async_runtime::block_on(nube.sintetizar(frase)) {
                Ok(audio) => return Ok(audio),
                Err(e) => {
                    eprintln!("Voz: {e} Sigo con la voz local.");
                    self.nube_en_pausa_hasta = Instant::now() + PAUSA_DE_LA_NUBE;
                }
            }
        }
        if self.piper.as_mut().is_none_or(|p| !p.vivo()) {
            self.piper = Some(Piper::abrir(&self.rutas, self.ajustes.velocidad)?);
        }
        let resultado = match self.piper.as_mut() {
            Some(piper) => piper.sintetizar(frase),
            None => Err(Error::Sistema("No pude abrir la voz local.".into())),
        };
        if resultado.is_err() {
            self.piper = None; // la próxima se reabre
        }
        resultado
    }
}
