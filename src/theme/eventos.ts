import type { TipoEvento } from "../lib/tipos";

/** Etiqueta corta (monoespaciada) de cada tipo de evento. */
export const etiquetaTipoEvento: Record<TipoEvento, string> = {
  clase: "CLASE",
  examen: "EXAMEN",
  entrega: "ENTREGA",
  cumpleanos: "CUMPLE",
  partido: "PARTIDO",
  otro: "OTRO",
};

/** Tipos que merecen el acento (lo urgente o lo importante). */
export const tipoDestacado: Record<TipoEvento, boolean> = {
  clase: false,
  examen: true,
  entrega: true,
  cumpleanos: false,
  partido: false,
  otro: false,
};
