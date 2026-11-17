import type { Transition, Variants } from "framer-motion";

/*
 * Movimiento del panel.
 * El panel se abre con un atajo muchas veces por día, así que la entrada es
 * corta (< 250 ms) y la salida todavía más (≈ 120 ms): nada que haga esperar.
 * El "wow" está en la esfera, no en la entrada.
 * Curvas: ease-out fuerte para entrar (responde al instante), sin ease-in.
 */
export const EASE_SALIDA = [0.23, 1, 0.32, 1] as const;

const salida: Transition = { duration: 0.12, ease: "easeOut" };

/** Fondo: aparece rápido; al cerrar se va último. */
export const varianteVelo: Variants = {
  oculto: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: 0.14, ease: EASE_SALIDA } },
  saliendo: { opacity: 0, transition: { duration: 0.14, ease: "easeOut", delay: 0.04 } },
};

/**
 * Cada zona (columna izquierda, derecha, barra) entra subiendo 6 px.
 * Retrasos no uniformes por zona: el ojo va primero al centro (esfera),
 * después a los costados y por último a la barra.
 */
export function varianteZona(retraso: number): Variants {
  return {
    oculto: { opacity: 0, y: 6 },
    visible: {
      opacity: 1,
      y: 0,
      transition: { duration: 0.2, ease: EASE_SALIDA, delay: retraso },
    },
    saliendo: { opacity: 0, transition: salida },
  };
}

/** La esfera crece con un resorte apenas elástico. */
export const varianteEsfera: Variants = {
  oculto: { opacity: 0, scale: 0.9 },
  visible: {
    opacity: 1,
    scale: 1,
    transition: { type: "spring", duration: 0.5, bounce: 0.18 },
  },
  saliendo: { opacity: 0, scale: 0.96, transition: salida },
};

/** Versión sin movimiento para "reducir movimiento" de Windows. */
export const varianteSoloOpacidad: Variants = {
  oculto: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: 0.15 } },
  saliendo: { opacity: 0, transition: { duration: 0.1 } },
};
