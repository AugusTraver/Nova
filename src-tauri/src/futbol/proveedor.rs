//! La interfaz con la API de fútbol. Igual que `ProveedorIa`: la tarea de
//! fondo y los comandos solo conocen este trait.

use std::future::Future;

use super::{EstadoJugador, JugadorEncontrado, Partido, Tabla};
use crate::error::Resultado;

pub trait ProveedorFutbol {
    /// Partidos de un equipo, jugados y por jugar, de todas las competencias
    /// (ordenados por fecha). Cuesta `PEDIDOS_CALENDARIO` pedidos.
    fn calendario(&self, equipo_id: &str) -> impl Future<Output = Resultado<Vec<Partido>>> + Send;

    /// Un partido con marcador, minuto y eventos (para el modo en vivo).
    fn partido(&self, partido_id: &str) -> impl Future<Output = Resultado<Partido>> + Send;

    /// La tabla de posiciones de una liga.
    fn tabla(&self, liga: &str) -> impl Future<Output = Resultado<Tabla>> + Send;

    /// Club, último partido y totales de un jugador.
    fn jugador(&self, jugador_id: &str, nombre: &str) -> impl Future<Output = Resultado<EstadoJugador>> + Send;

    fn buscar_jugadores(&self, texto: &str) -> impl Future<Output = Resultado<Vec<JugadorEncontrado>>> + Send;
}

/// `calendario` hace dos pedidos: los jugados y los que vienen.
pub const PEDIDOS_CALENDARIO: u32 = 2;
