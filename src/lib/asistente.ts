import type { Progreso } from "./tipos";

/** Qué dice debajo de la esfera mientras el asistente usa cada herramienta. */
const PASOS: Record<string, string> = {
  crear_evento: "Anotando el evento…",
  editar_evento: "Cambiando el evento…",
  borrar_evento: "Buscando el evento…",
  listar_eventos: "Mirando tu agenda…",
  crear_responsabilidad: "Anotándolo…",
  completar_responsabilidad: "Tachándolo de la lista…",
  listar_responsabilidades: "Mirando tus pendientes…",
  crear_materia: "Agregando la materia…",
  listar_materias: "Mirando tus materias…",
};

export function textoDelPaso(progreso: Progreso): string {
  if (progreso.tipo === "herramienta") return PASOS[progreso.nombre] ?? "Pensando…";
  return "Pensando…";
}

/** Cuánto queda la esfera en "error" antes de volver al reposo. */
export const DURACION_ERROR_MS = 1600;
