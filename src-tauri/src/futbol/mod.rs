//! Fútbol (SPEC 4.4): Boca, la Selección y los jugadores seguidos.
//!
//! - `proveedor`: el trait `ProveedorFutbol` (como `ProveedorIa`): si un día
//!   cambia la API, se escribe otra implementación y el resto no se toca.
//! - `espn`: la implementación con la API pública de ESPN (sin key).
//!
//! La lógica (próximo partido, en vivo, goles nuevos, cuándo refrescar) está
//! en `dominio::futbol`, sin red, con tests.

pub mod espn;
pub mod proveedor;

use serde::{Deserialize, Serialize};

/// Un equipo dentro de un partido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipoBreve {
    pub id: String,
    pub nombre: String,
    pub abreviatura: String,
    pub escudo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoPartido {
    Programado,
    EnJuego,
    Entretiempo,
    Terminado,
    /// Postergado, suspendido o cancelado.
    Suspendido,
}

impl EstadoPartido {
    pub fn en_curso(self) -> bool {
        matches!(self, EstadoPartido::EnJuego | EstadoPartido::Entretiempo)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoEventoPartido {
    Gol,
    /// Gol en contra: cuenta para el equipo de `equipo_id`, lo hizo un rival.
    GolEnContra,
    Expulsion,
}

/// Un gol o una expulsión.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventoPartido {
    /// "45'+1'"
    pub minuto: String,
    pub tipo: TipoEventoPartido,
    /// El equipo al que le suma el gol (o el del expulsado).
    pub equipo_id: String,
    pub jugador: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Partido {
    pub id: String,
    /// Identificador de la competencia en el proveedor (ej. "arg.1").
    pub liga: String,
    /// En castellano: "Liga Profesional", "Copa Argentina"...
    pub competencia: String,
    /// "octavos de final", "final"... si el proveedor lo dice.
    pub fase: Option<String>,
    /// ISO 8601 (en UTC, como viene del proveedor).
    pub fecha: String,
    pub local: EquipoBreve,
    pub visitante: EquipoBreve,
    pub estado: EstadoPartido,
    /// "34'" mientras se juega.
    pub minuto: Option<String>,
    pub goles_local: Option<u32>,
    pub goles_visitante: Option<u32>,
    pub penales_local: Option<u32>,
    pub penales_visitante: Option<u32>,
    /// Goles y expulsiones (solo cuando se pidió el detalle del partido).
    pub eventos: Vec<EventoPartido>,
}

impl Partido {
    pub fn juega(&self, equipo_id: &str) -> bool {
        self.local.id == equipo_id || self.visitante.id == equipo_id
    }

    /// Los goles del equipo `equipo_id` (0 si todavía no hay marcador).
    pub fn goles_de(&self, equipo_id: &str) -> u32 {
        let goles = if self.local.id == equipo_id { self.goles_local } else { self.goles_visitante };
        goles.unwrap_or(0)
    }

    pub fn rival_de(&self, equipo_id: &str) -> &EquipoBreve {
        if self.local.id == equipo_id { &self.visitante } else { &self.local }
    }
}

/// Una fila de la tabla de posiciones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaTabla {
    pub puesto: u32,
    pub equipo: EquipoBreve,
    pub puntos: u32,
    pub jugados: u32,
    pub ganados: u32,
    pub empatados: u32,
    pub perdidos: u32,
    pub diferencia: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrupoTabla {
    /// "Zona A"
    pub nombre: String,
    pub filas: Vec<FilaTabla>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tabla {
    pub torneo: String,
    pub grupos: Vec<GrupoTabla>,
}

/// Un partido de un jugador, con lo que hizo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartidoJugador {
    pub fecha: String,
    pub rival: String,
    /// "2-1", desde el lado del equipo del jugador.
    pub resultado: String,
    /// 'G', 'E' o 'P' (ganó, empató, perdió).
    pub desenlace: Option<String>,
    pub competencia: Option<String>,
    pub goles: u32,
    pub asistencias: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoJugador {
    pub id: String,
    pub nombre: String,
    pub club: Option<String>,
    pub ultimo: Option<PartidoJugador>,
    /// Totales de la temporada que informa el proveedor.
    pub partidos: u32,
    pub goles: u32,
    pub asistencias: u32,
}

/// Un resultado de "buscar jugador" en Ajustes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JugadorEncontrado {
    pub id: String,
    pub nombre: String,
    pub club: Option<String>,
}

/// Un jugador que sigo (se guarda en preferencias).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JugadorSeguido {
    pub id: String,
    pub nombre: String,
}

/// Los equipos que sigue Nova. Boca y la Selección son fijos (SPEC 4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipoSeguido {
    pub clave: &'static str,
    pub id: &'static str,
    pub nombre: &'static str,
    /// La liga de la que se muestra la tabla.
    pub liga_tabla: Option<&'static str>,
}

pub const BOCA: EquipoSeguido = EquipoSeguido { clave: "boca", id: "5", nombre: "Boca", liga_tabla: Some("arg.1") };
pub const SELECCION: EquipoSeguido =
    EquipoSeguido { clave: "seleccion", id: "202", nombre: "la Selección", liga_tabla: None };
pub const EQUIPOS: [EquipoSeguido; 2] = [BOCA, SELECCION];

/// Los jugadores que se siguen hasta que se cambien en Ajustes. Los ids son
/// los de ESPN (se buscaron con su buscador).
pub fn jugadores_por_defecto() -> Vec<JugadorSeguido> {
    [
        ("45843", "Lionel Messi"),
        ("400359", "Tomás Aranda"),
        ("153115", "Leandro Paredes"),
        ("400360", "Leonel Flores"),
        ("290542", "Lautaro Blanco"),
        ("320760", "Valentín Barco"),
        ("382978", "Milton Delgado"),
        ("132948", "Neymar"),
    ]
    .into_iter()
    .map(|(id, nombre)| JugadorSeguido { id: id.into(), nombre: nombre.into() })
    .collect()
}
