import type { TipoEvento } from "../lib/tipos";

/** Color de cada tipo de evento (variables definidas en tokens.css). */
export const colorEvento: Record<TipoEvento, string> = {
  clase: "var(--color-evento-clase)",
  examen: "var(--color-evento-examen)",
  entrega: "var(--color-evento-entrega)",
  cumpleanos: "var(--color-evento-cumpleanos)",
  partido: "var(--color-evento-partido)",
  otro: "var(--color-evento-otro)",
};

export const nombreTipoEvento: Record<TipoEvento, string> = {
  clase: "Clase",
  examen: "Examen",
  entrega: "Entrega",
  cumpleanos: "Cumpleaños",
  partido: "Partido",
  otro: "Otro",
};
