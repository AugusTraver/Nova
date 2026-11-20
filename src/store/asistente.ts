import { create } from "zustand";
import { api, mensajeDelAsistente } from "../lib/api";
import { DURACION_ERROR_MS, textoDelPaso } from "../lib/asistente";
import type { Progreso, Turno, VistaCambio } from "../lib/tipos";
import { usePanel } from "./panel";

interface EstadoAsistente {
  /** Lo último que escribió el usuario (se muestra chico arriba de la respuesta). */
  pedido: string | null;
  /** Qué está haciendo ahora ("Anotando el evento…"); null si no trabaja. */
  paso: string | null;
  respuesta: string | null;
  error: string | null;
  /** "¿Borro «X»…?" esperando los botones o un "sí" escrito. */
  confirmacion: string | null;
  /** Si lo que espera el "sí" es un cambio de archivo: el antes y después. */
  cambio: VistaCambio | null;

  /**
   * Manda un mensaje. Devuelve false si todavía está respondiendo el anterior.
   * `porVoz`: lo dijiste hablando; la respuesta se lee en voz alta.
   */
  enviar(texto: string, porVoz?: boolean): boolean;
  /** Muestra algo que dice Nova sin que se lo pidan (el resumen al abrir). */
  mostrar(texto: string): void;
  confirmar(aceptar: boolean): void;
  /** Al cerrar el panel: la próxima vez arranca limpio. */
  limpiar(): void;
}

let temporizadorError: number | undefined;

export const useAsistente = create<EstadoAsistente>()((set, get) => {
  const alProgreso = (p: Progreso) => set({ paso: textoDelPaso(p) });

  /** Lee en voz alta (si falla, vuelve a escuchar igual). */
  function decir(texto: string) {
    api.hablar(texto).catch((e: unknown) => {
      console.error("No se pudo hablar", e);
      void api.reanudarEscucha().catch(() => {});
    });
  }

  async function turno(pedido: string | null, correr: () => Promise<Turno>, porVoz = false) {
    window.clearTimeout(temporizadorError);
    set({ pedido, paso: "Pensando…", respuesta: null, error: null, confirmacion: null, cambio: null });
    usePanel.getState().ponerEsfera("pensando");
    try {
      const { respuesta, confirmacion, cambio } = await correr();
      set({ paso: null, respuesta, confirmacion, cambio });
      usePanel.getState().ponerEsfera("reposo");
      // Si se lo pediste hablando, contesta hablando (la pregunta de
      // confirmación también: "¿Borro…?" se responde con la voz).
      if (porVoz) decir(confirmacion ?? respuesta);
    } catch (e) {
      console.error("El asistente falló", e);
      const error = mensajeDelAsistente(e);
      set({ paso: null, error });
      usePanel.getState().ponerEsfera("error");
      temporizadorError = window.setTimeout(() => usePanel.getState().ponerEsfera("reposo"), DURACION_ERROR_MS);
      if (porVoz) decir(error);
    }
  }

  return {
    pedido: null,
    paso: null,
    respuesta: null,
    error: null,
    confirmacion: null,
    cambio: null,

    enviar(texto, porVoz = false) {
      const limpio = texto.trim();
      if (!limpio || get().paso !== null) return false;
      void turno(limpio, () => api.enviarMensaje(limpio, alProgreso, porVoz), porVoz);
      return true;
    },

    mostrar(texto) {
      if (get().paso !== null) return;
      set({ pedido: null, respuesta: texto, error: null, confirmacion: null, cambio: null });
    },

    confirmar(aceptar) {
      if (get().paso !== null) return;
      void turno(null, () => api.confirmarAccion(aceptar, alProgreso));
    },

    limpiar() {
      // Si está respondiendo, se deja terminar (el resultado igual se guarda).
      if (get().paso !== null) return;
      // Un borrado (o un cambio de archivo) sin responder se cancela: un "sí" escrito mañana no
      // tiene que borrar algo que ya no está a la vista.
      if (get().confirmacion !== null) {
        api.confirmarAccion(false, () => {}).catch((e) => console.error("No se pudo cancelar", e));
      }
      window.clearTimeout(temporizadorError);
      set({ pedido: null, respuesta: null, error: null, confirmacion: null, cambio: null });
      usePanel.getState().ponerEsfera("reposo");
    },
  };
});
