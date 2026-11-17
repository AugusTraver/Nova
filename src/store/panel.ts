import { create } from "zustand";

/** Estados visuales de la esfera (SPEC 3.3). En v0.1 solo se usa "reposo". */
export type EstadoEsfera = "reposo" | "escuchando" | "pensando" | "hablando" | "error";

interface EstadoPanel {
  /** El panel está mostrándose (o entrando). */
  abierto: boolean;
  estadoEsfera: EstadoEsfera;
  abrir: () => void;
  cerrar: () => void;
  alternar: () => void;
}

export const usePanel = create<EstadoPanel>()((set) => ({
  abierto: false,
  estadoEsfera: "reposo",
  abrir: () => set({ abierto: true }),
  cerrar: () => set({ abierto: false }),
  alternar: () => set((s) => ({ abierto: !s.abierto })),
}));
