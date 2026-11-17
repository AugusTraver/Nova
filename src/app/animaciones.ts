import type { Variants } from "framer-motion";

/** Curva suave tipo "ease out expo" para todo el panel. */
export const EASE = [0.16, 1, 0.3, 1] as const;

/** Fondo velado: se oscurece al abrir. */
export const varianteVelo: Variants = {
  oculto: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: 0.35, ease: EASE } },
  saliendo: { opacity: 0, transition: { duration: 0.25, ease: "easeIn", delay: 0.1 } },
};

/** Contenedor de widgets: orquesta la cascada de entrada. */
export const varianteGrilla: Variants = {
  oculto: {},
  visible: { transition: { staggerChildren: 0.05, delayChildren: 0.12 } },
  saliendo: { transition: { staggerChildren: 0.025, staggerDirection: -1 } },
};

/** Cada widget entra subiendo y desenfocado. */
export const varianteWidget: Variants = {
  oculto: { opacity: 0, y: 24, scale: 0.97, filter: "blur(8px)" },
  visible: {
    opacity: 1,
    y: 0,
    scale: 1,
    filter: "blur(0px)",
    transition: { duration: 0.55, ease: EASE },
  },
  saliendo: {
    opacity: 0,
    y: 12,
    scale: 0.98,
    filter: "blur(6px)",
    transition: { duration: 0.2, ease: "easeIn" },
  },
};

/** La esfera se expande desde el centro. */
export const varianteEsfera: Variants = {
  oculto: { opacity: 0, scale: 0.4 },
  visible: { opacity: 1, scale: 1, transition: { duration: 0.7, ease: EASE } },
  saliendo: { opacity: 0, scale: 0.6, transition: { duration: 0.25, ease: "easeIn" } },
};

/** Duración total de la salida (ms): después de esto se oculta la ventana. */
export const DURACION_SALIDA_MS = 380;
