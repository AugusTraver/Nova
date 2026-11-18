import { EASE_SALIDA } from "../app/animaciones";

/**
 * Cambio entre la lista y el formulario de un widget: un fundido corto con un
 * desplazamiento mínimo. Rápido porque se usa muchas veces seguidas.
 */
export const cambioDeVista = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: { duration: 0.16, ease: EASE_SALIDA } },
  exit: { opacity: 0, transition: { duration: 0.08, ease: "easeOut" as const } },
};
