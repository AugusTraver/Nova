// Voz (v0.5): cómo se traduce el estado del motor de Rust a la esfera y a
// los textos de debajo. Puro, con tests.
import type { EstadoEsfera } from "../store/panel";
import type { EstadoVoz } from "./tipos";

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

/** Las voces de Microsoft para elegir en Ajustes. */
export const VOCES_AZURE = [
  { valor: "es-MX-DaliaNeural", texto: "Dalia" },
  { valor: "es-MX-JorgeNeural", texto: "Jorge" },
  { valor: "es-ES-ElviraNeural", texto: "Elvira" },
  { valor: "es-US-PalomaNeural", texto: "Paloma" },
] as const;

/** Con qué voz va a hablar Nova según lo que está cargado. */
export function vozQueHabla(regionAzure: string, tieneKeyAzure: boolean): "azure" | "local" {
  return regionAzure.trim() !== "" && tieneKeyAzure ? "azure" : "local";
}
