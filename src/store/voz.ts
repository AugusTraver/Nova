import { create } from "zustand";
import type { EstadoVoz, ProgresoDescarga } from "../lib/tipos";

/**
 * El nivel del audio (0 a 1) para la esfera. Cambia 30 veces por segundo,
 * así que no va en el store (re-renderizaría todo): la esfera lo lee en cada
 * cuadro directo de acá.
 */
export const nivelVoz = { valor: 0 };

interface EstadoDeVoz {
  estado: EstadoVoz;
  /** El error, si lo hay. */
  mensaje: string | null;
  /** Lo que vas diciendo mientras hablás. */
  parcial: string | null;
  descarga: ProgresoDescarga | null;
  ponerEstado(estado: EstadoVoz, mensaje: string | null): void;
  ponerParcial(texto: string | null): void;
  ponerDescarga(p: ProgresoDescarga | null): void;
}

export const useVoz = create<EstadoDeVoz>()((set) => ({
  estado: "apagada",
  mensaje: null,
  parcial: null,
  descarga: null,
  ponerEstado: (estado, mensaje) =>
    set((s) => ({
      estado,
      mensaje,
      // La transcripción en vivo solo tiene sentido mientras hablás.
      parcial: estado === "oyendo" || estado === "transcribiendo" ? s.parcial : null,
    })),
  ponerParcial: (parcial) => set({ parcial }),
  ponerDescarga: (descarga) => set({ descarga }),
}));
