// Estado de los formularios y su conversión a los datos que espera Rust.
// Rust vuelve a validar todo; esto es para avisar antes y con mejores textos.
import { aFecha, aIsoLocal, horaDe } from "./fechas";
import type {
  DatosEvento,
  DatosResponsabilidad,
  Evento,
  Prioridad,
  Recurrencia,
  Responsabilidad,
  TipoEvento,
} from "./tipos";

export type Resultado<T> = { ok: true; datos: T } | { ok: false; error: string };

// ---------- Eventos ----------

export interface FormEvento {
  titulo: string;
  tipo: TipoEvento;
  /** "2026-11-20" */
  fecha: string;
  todoElDia: boolean;
  /** "09:00" */
  horaInicio: string;
  /** "" = sin hora de fin */
  horaFin: string;
  /** Solo para todo el día: último día incluido ("" = un solo día). */
  fechaFin: string;
  recurrencia: Recurrencia;
  /** "" = sin fin */
  repetirHasta: string;
  materiaId: number | null;
  persona: string;
  notas: string;
}

/** Tipos que pueden tener materia. */
export const TIPOS_CON_MATERIA: readonly TipoEvento[] = ["clase", "examen", "entrega"];

export function formEventoNuevo(dia: Date): FormEvento {
  return {
    titulo: "",
    tipo: "clase",
    fecha: aFecha(dia),
    todoElDia: false,
    horaInicio: "09:00",
    horaFin: "",
    fechaFin: "",
    recurrencia: "semanal",
    repetirHasta: "",
    materiaId: null,
    persona: "",
    notas: "",
  };
}

export function formDesdeEvento(e: Evento): FormEvento {
  return {
    titulo: e.titulo,
    tipo: e.tipo,
    fecha: e.inicio.slice(0, 10),
    todoElDia: e.todoElDia,
    horaInicio: e.todoElDia ? "09:00" : horaDe(e.inicio),
    horaFin: !e.todoElDia && e.fin ? horaDe(e.fin) : "",
    fechaFin: e.todoElDia && e.fin ? e.fin.slice(0, 10) : "",
    recurrencia: e.recurrencia,
    repetirHasta: e.repetirHasta ?? "",
    materiaId: e.materiaId,
    persona: e.persona ?? "",
    notas: e.notas ?? "",
  };
}

/**
 * Al elegir un tipo en un evento NUEVO, se proponen valores típicos:
 * las clases se repiten cada semana; los cumpleaños son de todo el día y anuales.
 * En uno que ya existe solo cambia el tipo.
 */
export function cambiarTipo(form: FormEvento, tipo: TipoEvento, esNuevo: boolean): FormEvento {
  if (!esNuevo) return { ...form, tipo };
  switch (tipo) {
    case "clase":
      return { ...form, tipo, todoElDia: false, recurrencia: "semanal" };
    case "cumpleanos":
      return { ...form, tipo, todoElDia: true, recurrencia: "anual" };
    default:
      return { ...form, tipo, recurrencia: "ninguna" };
  }
}

export function datosDesdeFormEvento(f: FormEvento): Resultado<DatosEvento> {
  const titulo = f.titulo.trim();
  if (!titulo) return { ok: false, error: "Ponele un título." };
  if (!/^\d{4}-\d{2}-\d{2}$/.test(f.fecha)) return { ok: false, error: "Elegí una fecha." };

  let inicio = f.fecha;
  let fin: string | null = null;
  if (f.todoElDia && f.fechaFin && f.fechaFin !== f.fecha) {
    if (f.fechaFin < f.fecha) return { ok: false, error: "Tiene que terminar después de empezar." };
    fin = f.fechaFin;
  }
  if (!f.todoElDia) {
    if (!/^\d{2}:\d{2}$/.test(f.horaInicio)) return { ok: false, error: "Elegí a qué hora empieza." };
    inicio = aIsoLocal(f.fecha, f.horaInicio);
    if (f.horaFin) {
      if (f.horaFin <= f.horaInicio) return { ok: false, error: "Tiene que terminar después de empezar." };
      fin = aIsoLocal(f.fecha, f.horaFin);
    }
  }

  const repetirHasta = f.recurrencia === "ninguna" || !f.repetirHasta ? null : f.repetirHasta;
  if (repetirHasta && repetirHasta < f.fecha) {
    return { ok: false, error: "La repetición no puede terminar antes de empezar." };
  }

  return {
    ok: true,
    datos: {
      titulo,
      tipo: f.tipo,
      inicio,
      fin,
      todoElDia: f.todoElDia,
      materiaId: TIPOS_CON_MATERIA.includes(f.tipo) ? f.materiaId : null,
      persona: f.tipo === "cumpleanos" && f.persona.trim() ? f.persona.trim() : null,
      notas: f.notas.trim() || null,
      recurrencia: f.recurrencia,
      repetirHasta,
    },
  };
}

// ---------- Responsabilidades ----------

export interface FormResponsabilidad {
  titulo: string;
  /** "" = sin fecha */
  fechaLimite: string;
  prioridad: Prioridad;
  materiaId: number | null;
}

export function formResponsabilidadNueva(): FormResponsabilidad {
  return { titulo: "", fechaLimite: "", prioridad: "media", materiaId: null };
}

export function formDesdeResponsabilidad(r: Responsabilidad): FormResponsabilidad {
  return { titulo: r.titulo, fechaLimite: r.fechaLimite ?? "", prioridad: r.prioridad, materiaId: r.materiaId };
}

/**
 * `anterior` es la que se está editando (si es una edición): se le conservan
 * la descripción y el evento vinculado, que el formulario no muestra.
 */
export function datosDesdeFormResponsabilidad(
  f: FormResponsabilidad,
  anterior?: Responsabilidad,
): Resultado<DatosResponsabilidad> {
  const titulo = f.titulo.trim();
  if (!titulo) return { ok: false, error: "¿Qué tenés que hacer?" };
  if (f.fechaLimite && !/^\d{4}-\d{2}-\d{2}$/.test(f.fechaLimite)) return { ok: false, error: "Esa fecha no es válida." };
  return {
    ok: true,
    datos: {
      titulo,
      descripcion: anterior?.descripcion ?? null,
      fechaLimite: f.fechaLimite || null,
      prioridad: f.prioridad,
      materiaId: f.materiaId,
      eventoId: anterior?.eventoId ?? null,
    },
  };
}
