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
  // La compu (v0.7.1)
  abrir_pagina: "Abriendo la página…",
  buscar_en_google: "Abriendo Google…",
  poner_video: "Buscando el video…",
  abrir_programa: "Abriendo el programa…",
  abrir_archivo: "Abriéndolo…",
  leer_archivo: "Leyendo el archivo…",
  cambiar_archivo: "Preparando el cambio…",
  crear_archivo: "Preparando el archivo…",
  borrar_archivo: "Buscando el archivo…",
  deshacer_cambio: "Buscando el último cambio…",
  controlar_musica: "Tocando la música…",
};

export function textoDelPaso(progreso: Progreso): string {
  if (progreso.tipo === "herramienta") return PASOS[progreso.nombre] ?? "Pensando…";
  return "Pensando…";
}

/** Cuánto queda la esfera en "error" antes de volver al reposo. */
export const DURACION_ERROR_MS = 1600;
