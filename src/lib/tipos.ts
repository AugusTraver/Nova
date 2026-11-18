// Tipos del dominio. Los de datos locales son espejo de los structs de Rust
// (src-tauri/src/repos/*.rs, con serde en camelCase): si cambia uno, cambia el otro.

export type TipoEvento = "clase" | "examen" | "entrega" | "cumpleanos" | "partido" | "otro";
export type Recurrencia = "ninguna" | "semanal" | "anual";
export type Prioridad = "baja" | "media" | "alta";
export type EstadoResponsabilidad = "pendiente" | "hecha";
export type OrigenResponsabilidad = "manual" | "auto" | "asistente";

/** Paleta de materias (tokens `--color-materia-*`). Igual a `PALETA` en materias.rs. */
export const PALETA_MATERIAS = ["celeste", "coral", "menta", "lavanda", "ambar", "rosa"] as const;
export type ColorMateria = (typeof PALETA_MATERIAS)[number];

export interface Materia {
  id: number;
  nombre: string;
  color: ColorMateria;
}

export interface DatosMateria {
  nombre: string;
  color: ColorMateria;
}

export interface Evento {
  id: number;
  titulo: string;
  tipo: TipoEvento;
  /** ISO 8601 con zona ("2026-11-20T09:00:00-03:00") o fecha sola si es de todo el día. */
  inicio: string;
  fin: string | null;
  todoElDia: boolean;
  materiaId: number | null;
  persona: string | null;
  notas: string | null;
  recurrencia: Recurrencia;
  /** Fecha sola, incluida. */
  repetirHasta: string | null;
  /** Fechas salteadas de la serie. */
  excepciones: string[];
}

export type DatosEvento = Omit<Evento, "id" | "excepciones">;

/** Una repetición concreta de un evento. */
export interface Ocurrencia {
  evento: Evento;
  /** Día en que empieza ("2026-11-20"). */
  fecha: string;
  inicio: string;
  fin: string | null;
}

export interface Responsabilidad {
  id: number;
  titulo: string;
  descripcion: string | null;
  /** Fecha sola ("2026-11-20"). */
  fechaLimite: string | null;
  prioridad: Prioridad;
  estado: EstadoResponsabilidad;
  hechaEn: string | null;
  eventoId: number | null;
  materiaId: number | null;
  origen: OrigenResponsabilidad;
  creadoEn: string;
}

export interface DatosResponsabilidad {
  titulo: string;
  descripcion: string | null;
  fechaLimite: string | null;
  prioridad: Prioridad;
  materiaId: number | null;
  eventoId: number | null;
}

export interface Regla {
  activa: boolean;
  diasAntes: number;
  /** Con marcadores {materia}, {entrega}, {persona}, {titulo}. */
  plantilla: string;
}

export interface Reglas {
  examen: Regla;
  entrega: Regla;
  cumpleanos: Regla;
}

export interface Ajustes {
  /** "Ctrl+Shift+Space": modificadores + `KeyboardEvent.code`. */
  atajo: string;
  reglas: Reglas;
}

/** Error que devuelven los comandos de Rust (ver src-tauri/src/error.rs). */
export interface ErrorNova {
  tipo: "base" | "validacion" | "no_encontrado" | "sistema";
  mensaje: string;
}

// ---- Asistente (ver src-tauri/src/asistente) ----

/** Qué datos cambió el asistente; los widgets recargan solo eso. */
export type FuenteDatos = "agenda" | "responsabilidades" | "materias" | "futbol" | "noticias" | "clima";

/** Cómo terminó un turno con el asistente. */
export interface Turno {
  respuesta: string;
  /** Si quedó un borrado esperando el "sí": la pregunta para los botones. */
  confirmacion: string | null;
}

/** Avisos mientras el asistente trabaja. */
export type Progreso = { tipo: "pensando" } | { tipo: "herramienta"; nombre: string };

export type TipoProveedor = "groq" | "ollama";

export interface AjustesIa {
  proveedor: TipoProveedor;
  modeloGroq: string;
  modeloOllama: string;
  urlOllama: string;
}

/** Ajustes de IA y si hay una API key guardada (la key nunca sale de Rust). */
export interface VistaAjustesIa {
  ajustes: AjustesIa;
  tieneKey: boolean;
}

// ---- Datos de internet (ver src-tauri/src/futbol, noticias, clima) ----

/** Un dato de internet guardado en la caché de Rust. */
export interface Cacheado<T> {
  /** `null` si todavía no se pudo descargar nunca. */
  datos: T | null;
  /** Cuándo se obtuvo lo que se muestra (ISO 8601). */
  obtenidoEn: string | null;
  /** El error del último intento; con `datos`, se muestra lo viejo con un aviso. */
  error: string | null;
}

export interface EquipoBreve {
  id: string;
  nombre: string;
  abreviatura: string;
  escudo: string | null;
}

export type EstadoPartido = "programado" | "en_juego" | "entretiempo" | "terminado" | "suspendido";

export interface EventoPartido {
  /** "45'+1'" */
  minuto: string;
  tipo: "gol" | "gol_en_contra" | "expulsion";
  /** El equipo al que le suma el gol, o el del expulsado. */
  equipoId: string;
  jugador: string | null;
}

export interface Partido {
  id: string;
  liga: string;
  /** "Liga Profesional", "Sudamericana"… */
  competencia: string;
  /** "final", "octavos de final"… */
  fase: string | null;
  /** ISO 8601 (UTC) */
  fecha: string;
  local: EquipoBreve;
  visitante: EquipoBreve;
  estado: EstadoPartido;
  /** "34'" mientras se juega. */
  minuto: string | null;
  golesLocal: number | null;
  golesVisitante: number | null;
  penalesLocal: number | null;
  penalesVisitante: number | null;
  eventos: EventoPartido[];
}

export interface FilaTabla {
  puesto: number;
  equipo: EquipoBreve;
  puntos: number;
  jugados: number;
  ganados: number;
  empatados: number;
  perdidos: number;
  diferencia: number;
}

export interface Posicion {
  torneo: string;
  /** "Zona A" */
  zona: string;
  fila: FilaTabla;
  equiposEnZona: number;
}

export interface ResumenEquipo {
  clave: "boca" | "seleccion";
  id: string;
  nombre: string;
  proximo: Partido | null;
  ultimo: Partido | null;
  enVivo: Partido | null;
  posicion: Posicion | null;
}

export interface PartidoJugador {
  fecha: string;
  rival: string;
  /** "2-1", desde el lado del jugador. */
  resultado: string;
  /** Ganó, empató o perdió. */
  desenlace: "G" | "E" | "P" | null;
  competencia: string | null;
  goles: number;
  asistencias: number;
}

export interface EstadoJugador {
  id: string;
  nombre: string;
  club: string | null;
  ultimo: PartidoJugador | null;
  partidos: number;
  goles: number;
  asistencias: number;
}

export interface VistaFutbol {
  /** Boca primero, después la Selección. */
  equipos: ResumenEquipo[];
  jugadores: EstadoJugador[];
}

/** Un jugador que sigo (Ajustes). */
export interface JugadorSeguido {
  id: string;
  nombre: string;
}

/** Un resultado del buscador de jugadores. */
export interface JugadorEncontrado {
  id: string;
  nombre: string;
  club: string | null;
}

/** Evento `futbol:gol` de Rust. */
export interface AvisoGol {
  equipo: "boca" | "seleccion";
  goles: number;
}

export interface Noticia {
  id: number;
  titulo: string;
  resumen: string;
  fuente: string;
  url: string;
}
