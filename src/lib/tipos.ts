// Tipos del dominio, alineados con el modelo de datos de SPEC.md (sección 6).
// En v0.1 solo alimentan los datos de ejemplo; en v0.2 los usará SQLite.

export type TipoEvento =
  | "clase"
  | "examen"
  | "entrega"
  | "cumpleanos"
  | "partido"
  | "otro";

export interface Materia {
  id: number;
  nombre: string;
  color: string;
}

export interface Evento {
  id: number;
  titulo: string;
  tipo: TipoEvento;
  /** ISO 8601 con zona horaria */
  inicio: string;
  fin?: string;
  todoElDia: boolean;
  materiaId?: number;
  persona?: string;
}

export type Prioridad = "baja" | "media" | "alta";
export type EstadoResponsabilidad = "pendiente" | "hecha";
export type OrigenResponsabilidad = "manual" | "auto" | "asistente";

export interface Responsabilidad {
  id: number;
  titulo: string;
  /** Fecha (YYYY-MM-DD) */
  fechaLimite?: string;
  prioridad: Prioridad;
  estado: EstadoResponsabilidad;
  origen: OrigenResponsabilidad;
  materiaId?: number;
}

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
