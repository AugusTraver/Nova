// Voz (v0.5): cómo se traduce el estado del motor de Rust a la esfera y a
// los textos de debajo. Puro, con tests.
import type { EstadoEsfera } from "../store/panel";
import type { AjustesVoz, EstadoVoz, ProveedorVoz } from "./tipos";

/**
 * La esfera según el asistente y la voz. Lo del asistente manda (pensando,
 * error); si no hace nada, la voz decide si escucha o habla.
 */
export function esferaSegun(asistente: EstadoEsfera, voz: EstadoVoz): EstadoEsfera {
  if (asistente !== "reposo") return asistente;
  switch (voz) {
    case "escuchando":
    case "oyendo":
      return "escuchando";
    case "transcribiendo":
    case "pensando":
    case "preparando":
      return "pensando";
    case "hablando":
      return "hablando";
    default:
      return "reposo";
  }
}

/** La línea chica arriba de la respuesta cuando no hay un pedido escrito. */
export function pistaDeVoz(voz: EstadoVoz): string | null {
  switch (voz) {
    case "preparando":
      return "Preparando la voz…";
    case "escuchando":
      return "Te escucho";
    case "oyendo":
      return "Te escucho…";
    case "transcribiendo":
      return "Entendiendo…";
    case "pausada":
      return "No escucho mientras escribís";
    case "sin_modelos":
      return "Para hablarme, bajá la voz en Ajustes";
    default:
      return null;
  }
}

/** "312 de 1.340 MB". */
export function textoDescarga(bajados: number, totales: number): string {
  const formato = new Intl.NumberFormat("es-AR");
  return `${formato.format(bajados)} de ${formato.format(totales)} MB`;
}

/** Las voces de Microsoft para elegir en Ajustes. Las HD entonan solas
 * según lo que dicen, pero no cambian de velocidad. */
export const VOCES_AZURE = [
  { valor: "es-AR-ElenaNeural", texto: "Elena (argentina)" },
  { valor: "es-AR-TomasNeural", texto: "Tomás (argentino)" },
  { valor: "es-MX-Ximena:DragonHDLatestNeural", texto: "Ximena HD" },
  { valor: "es-MX-Tristan:DragonHDLatestNeural", texto: "Tristán HD" },
  { valor: "es-MX-DaliaNeural", texto: "Dalia" },
  { valor: "es-MX-JorgeNeural", texto: "Jorge" },
  { valor: "es-ES-ElviraNeural", texto: "Elvira" },
  { valor: "es-US-PalomaNeural", texto: "Paloma" },
] as const;

/** Las voces de OpenAI. Todas hablan con el acento que se les pide. */
export const VOCES_OPENAI = [
  { valor: "coral", texto: "Coral" },
  { valor: "marin", texto: "Marin" },
  { valor: "sage", texto: "Sage" },
  { valor: "shimmer", texto: "Shimmer" },
  { valor: "nova", texto: "Nova" },
  { valor: "ash", texto: "Ash" },
  { valor: "cedar", texto: "Cedar" },
  { valor: "ballad", texto: "Ballad" },
] as const;

export const NOMBRE_PROVEEDOR: Record<ProveedorVoz, string> = {
  azure: "Microsoft",
  elevenlabs: "ElevenLabs",
  openai: "OpenAI",
  local: "Local",
};

/** ¿Tiene todo lo que necesita para hablar? (La local, siempre.) */
export function proveedorListo(proveedor: ProveedorVoz, ajustes: AjustesVoz, conKey: readonly ProveedorVoz[]): boolean {
  if (proveedor === "local") return true;
  if (!conKey.includes(proveedor)) return false;
  if (proveedor === "azure") return ajustes.regionAzure.trim() !== "";
  if (proveedor === "elevenlabs") return ajustes.vozEleven !== "";
  return true;
}

/** Con qué voz va a hablar Nova: la elegida si está lista; si no, la local. */
export function vozQueHabla(ajustes: AjustesVoz, conKey: readonly ProveedorVoz[]): ProveedorVoz {
  return proveedorListo(ajustes.proveedor, ajustes, conKey) ? ajustes.proveedor : "local";
}

/** "Tardó 0,4 s" o "Tardó 850 ms". */
export function textoDemora(milisegundos: number): string {
  if (milisegundos < 1000) return `Tardó ${Math.round(milisegundos)} ms en empezar`;
  return `Tardó ${(milisegundos / 1000).toFixed(1).replace(".", ",")} s en empezar`;
}
