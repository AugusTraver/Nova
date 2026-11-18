import type { TipoEvento } from "../lib/tipos";

/** Cómo se nombra cada tipo de evento en la interfaz. */
export const nombreTipoEvento: Record<TipoEvento, string> = {
  clase: "Clase",
  examen: "Examen",
  entrega: "Entrega",
  cumpleanos: "Cumpleaños",
  partido: "Partido",
  otro: "Evento",
};

/** Tipos que merecen el color de aviso (lo urgente o lo importante). */
export const tipoDestacado: Record<TipoEvento, boolean> = {
  clase: false,
  examen: true,
  entrega: true,
  cumpleanos: false,
  partido: false,
  otro: false,
};
