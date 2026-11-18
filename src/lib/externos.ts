// El aviso de los widgets con datos de internet (SPEC 8, "Resiliente"):
// si el último intento falló, se muestra lo guardado y cuán viejo es.
import type { Cacheado } from "./tipos";

/** "hace 5 min", "hace 3 h", "de ayer", "de hace 4 días". */
export function antiguedad(iso: string, ahora: Date): string {
  const minutos = Math.max(0, Math.round((ahora.getTime() - new Date(iso).getTime()) / 60_000));
  if (minutos < 1) return "de recién";
  if (minutos < 60) return `de hace ${minutos} min`;
  const horas = Math.round(minutos / 60);
  if (horas < 24) return `de hace ${horas} h`;
  const dias = Math.round(horas / 24);
  return dias === 1 ? "de ayer" : `de hace ${dias} días`;
}

export type EstadoExterno =
  | { tipo: "cargando" }
  /** Nunca se pudo descargar: solo el error. */
  | { tipo: "sin_datos"; mensaje: string }
  /** Hay datos; `aviso` si son viejos porque falló el último intento. */
  | { tipo: "listo"; aviso: string | null };

export function estadoExterno<T>(c: Cacheado<T> | null, ahora: Date): EstadoExterno {
  if (!c || (c.datos === null && c.error === null)) return { tipo: "cargando" };
  if (c.datos === null) return { tipo: "sin_datos", mensaje: c.error ?? "Sin datos." };
  if (c.error === null) return { tipo: "listo", aviso: null };
  const cuando = c.obtenidoEn ? ` · datos ${antiguedad(c.obtenidoEn, ahora)}` : "";
  return { tipo: "listo", aviso: `${sinPunto(c.error)}${cuando}` };
}

function sinPunto(texto: string): string {
  return texto.replace(/\.$/, "");
}
