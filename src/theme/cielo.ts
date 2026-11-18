// El fondo del panel es un cielo que cambia según la hora del día.
// Acá solo se decide QUÉ momento del día es; los colores de cada momento
// están en tokens.css (bloques `:root[data-cielo="..."]`).

export const MOMENTOS = ["madrugada", "manana", "mediodia", "tarde", "anochecer", "noche"] as const;
export type Momento = (typeof MOMENTOS)[number];

/**
 * Momento del día para una hora local.
 * madrugada 0–5 · mañana 6–10 · mediodía 11–15 · tarde 16–18 · anochecer 19–20 · noche 21–23
 */
export function momentoDelDia(fecha: Date): Momento {
  const h = fecha.getHours();
  if (h < 6) return "madrugada";
  if (h < 11) return "manana";
  if (h < 16) return "mediodia";
  if (h < 19) return "tarde";
  if (h < 21) return "anochecer";
  return "noche";
}

export function esMomento(valor: string | null): valor is Momento {
  return valor !== null && (MOMENTOS as readonly string[]).includes(valor);
}

/** Saludo según la hora. */
export function saludo(fecha: Date): string {
  const h = fecha.getHours();
  if (h < 6) return "Buenas noches";
  if (h < 13) return "Buen día";
  if (h < 20) return "Buenas tardes";
  return "Buenas noches";
}
