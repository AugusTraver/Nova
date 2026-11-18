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

// ---- Datos externos (de ejemplo hasta v0.4) ----

export interface Partido {
  rival: string;
  competencia: string;
  /** ISO 8601 */
  fecha: string;
  local: boolean;
}

export interface ResultadoPartido extends Partido {
  golesBoca: number;
  golesRival: number;
}

export interface Noticia {
  id: number;
  titulo: string;
  resumen: string;
  fuente: string;
  url: string;
}
