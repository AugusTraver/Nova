import { create } from "zustand";

/** Estados visuales de la esfera (SPEC 3.3). Desde v0.3 se usan reposo, pensando y error. */
export type EstadoEsfera = "reposo" | "escuchando" | "pensando" | "hablando" | "error";

/** Qué parte del panel se está editando. Mientras tanto, el resto se atenúa. */
export type Edicion = "calendario" | "responsabilidades" | "facu" | "ajustes";

interface EstadoPanel {
  /** El panel está mostrándose (o entrando). */
  abierto: boolean;
  estadoEsfera: EstadoEsfera;
  edicion: Edicion | null;
  /** Sube con cada gol de Boca (la esfera festeja cuando cambia). */
  goles: number;
  abrir: () => void;
  cerrar: () => void;
  alternar: () => void;
  editar: (edicion: Edicion) => void;
  terminarEdicion: () => void;
  ponerEsfera: (estado: EstadoEsfera) => void;
  festejarGol: () => void;
}

export const usePanel = create<EstadoPanel>()((set) => ({
  abierto: false,
  estadoEsfera: "reposo",
  edicion: null,
  goles: 0,
  abrir: () => set({ abierto: true }),
  // Al cerrar se descarta cualquier edición a medio hacer.
  cerrar: () => set({ abierto: false, edicion: null }),
  alternar: () => set((s) => ({ abierto: !s.abierto, edicion: null })),
  editar: (edicion) => set({ edicion }),
  terminarEdicion: () => set({ edicion: null }),
  ponerEsfera: (estadoEsfera) => set({ estadoEsfera }),
  festejarGol: () => set((s) => ({ goles: s.goles + 1 })),
}));

// En desarrollo, el estado queda a mano en la consola del navegador
// (`__panel.getState().festejarGol()`), para probar la esfera sin Rust.
if (import.meta.env.DEV) {
  (window as unknown as { __panel: typeof usePanel }).__panel = usePanel;
}
