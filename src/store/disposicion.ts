import { create } from "zustand";
import { api } from "../lib/api";
import type { Corrimiento, Disposicion, IdWidget } from "../lib/tipos";

/** Dónde dejó cada widget (v0.7.1). Se guarda en Rust, en las preferencias. */
interface EstadoDisposicion {
  disposicion: Disposicion;
  cargar: () => Promise<void>;
  mover: (widget: IdWidget, corrimiento: Corrimiento) => Promise<void>;
  /** Todos a su lugar de siempre. */
  restablecer: () => Promise<void>;
}

export const useDisposicion = create<EstadoDisposicion>()((set, get) => {
  /** Se muestra enseguida y se guarda atrás; si Rust lo rechaza, vuelve lo guardado. */
  async function guardar(nueva: Disposicion) {
    const antes = get().disposicion;
    set({ disposicion: nueva });
    try {
      set({ disposicion: await api.guardarDisposicion(nueva) });
    } catch (e) {
      console.error("No se pudo guardar dónde quedaron los widgets", e);
      set({ disposicion: antes });
    }
  }

  return {
    disposicion: {},
    cargar: async () => {
      try {
        set({ disposicion: await api.obtenerDisposicion() });
      } catch (e) {
        // Sin lo guardado, cada widget en su lugar de siempre.
        console.error("No se pudo leer dónde quedaron los widgets", e);
      }
    },
    mover: (widget, corrimiento) => guardar({ ...get().disposicion, [widget]: corrimiento }),
    restablecer: () => guardar({}),
  };
});
