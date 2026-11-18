import { create } from "zustand";
import { api, mensajeDelAsistente } from "../lib/api";
import { DURACION_ERROR_MS, textoDelPaso } from "../lib/asistente";
import type { Progreso, Turno } from "../lib/tipos";
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

  /** Manda un mensaje. Devuelve false si todavía está respondiendo el anterior. */
  enviar(texto: string): boolean;
  confirmar(aceptar: boolean): void;
  /** Al cerrar el panel: la próxima vez arranca limpio. */
  limpiar(): void;
}

let temporizadorError: number | undefined;

export const useAsistente = create<EstadoAsistente>()((set, get) => {
  const alProgreso = (p: Progreso) => set({ paso: textoDelPaso(p) });

  async function turno(pedido: string | null, correr: () => Promise<Turno>) {
    window.clearTimeout(temporizadorError);
    set({ pedido, paso: "Pensando…", respuesta: null, error: null, confirmacion: null });
    usePanel.getState().ponerEsfera("pensando");
    try {
      const { respuesta, confirmacion } = await correr();
      set({ paso: null, respuesta, confirmacion });
      usePanel.getState().ponerEsfera("reposo");
    } catch (e) {
      console.error("El asistente falló", e);
      set({ paso: null, error: mensajeDelAsistente(e) });
      usePanel.getState().ponerEsfera("error");
      temporizadorError = window.setTimeout(() => usePanel.getState().ponerEsfera("reposo"), DURACION_ERROR_MS);
    }
  }

  return {
    pedido: null,
    paso: null,
    respuesta: null,
    error: null,
    confirmacion: null,

    enviar(texto) {
      const limpio = texto.trim();
      if (!limpio || get().paso !== null) return false;
      void turno(limpio, () => api.enviarMensaje(limpio, alProgreso));
      return true;
    },

    confirmar(aceptar) {
      if (get().paso !== null) return;
      void turno(null, () => api.confirmarAccion(aceptar, alProgreso));
    },

    limpiar() {
      // Si está respondiendo, se deja terminar (el resultado igual se guarda).
      if (get().paso !== null) return;
      // Un borrado sin responder se cancela: un "sí" escrito mañana no
      // tiene que borrar algo que ya no está a la vista.
      if (get().confirmacion !== null) {
        api.confirmarAccion(false, () => {}).catch((e) => console.error("No se pudo cancelar", e));
      }
      window.clearTimeout(temporizadorError);
      set({ pedido: null, respuesta: null, error: null, confirmacion: null });
      usePanel.getState().ponerEsfera("reposo");
    },
  };
});
