// El "imán suave" de los widgets que se arrastran (v0.7.1): al soltar uno,
// se acomoda lo mínimo para quedar adentro del panel, sin tapar la esfera
// (ni la respuesta o la barra) y sin pisar a otro widget.
import type { Corrimiento } from "./tipos";

/** Un rectángulo en píxeles: esquina de arriba a la izquierda y tamaño. */
export interface Caja {
  x: number;
  y: number;
  ancho: number;
  alto: number;
}

/** Aire que se deja entre un widget y lo demás, y contra el borde. */
export const AIRE = 16;

const seTocan = (a: Caja, b: Caja, aire: number) =>
  a.x < b.x + b.ancho + aire && b.x < a.x + a.ancho + aire && a.y < b.y + b.alto + aire && b.y < a.y + a.alto + aire;

/** Lo mete adentro del panel. Si es más grande que el panel, queda pegado arriba a la izquierda. */
function adentro(caja: Caja, panel: Caja, aire: number): Caja {
  const x = Math.max(panel.x + aire, Math.min(caja.x, panel.x + panel.ancho - aire - caja.ancho));
  const y = Math.max(panel.y + aire, Math.min(caja.y, panel.y + panel.alto - aire - caja.alto));
  return { ...caja, x, y };
}

/**
 * Dónde queda un widget soltado en `caja`. Lo empuja hacia el lado más
 * cercano de cada cosa que toca, y repite unas veces por si al salir de una
 * cae sobre otra. Si no hay lugar libre, queda donde menos pisa (no se pierde).
 */
export function acomodar(caja: Caja, obstaculos: Caja[], panel: Caja, aire = AIRE): Caja {
  let actual = adentro(caja, panel, aire);
  for (let vuelta = 0; vuelta < 12; vuelta++) {
    const choque = obstaculos.find((o) => seTocan(actual, o, aire));
    if (!choque) return actual;
    const opciones: Caja[] = [
      { ...actual, x: choque.x - aire - actual.ancho },
      { ...actual, x: choque.x + choque.ancho + aire },
      { ...actual, y: choque.y - aire - actual.alto },
      { ...actual, y: choque.y + choque.alto + aire },
    ]
      .map((c) => adentro(c, panel, aire))
      .filter((c) => !seTocan(c, choque, aire));
    if (opciones.length === 0) return actual;
    // La que menos lo mueve, y entre esas la que no cae sobre otro.
    const distancia = (c: Caja) => Math.hypot(c.x - actual.x, c.y - actual.y);
    const libres = opciones.filter((c) => !obstaculos.some((o) => seTocan(c, o, aire)));
    actual = (libres.length > 0 ? libres : opciones).sort((a, b) => distancia(a) - distancia(b))[0];
  }
  return actual;
}

/** De píxeles a fracción del panel, para guardar. */
export const aFraccion = (px: { x: number; y: number }, panel: { ancho: number; alto: number }): Corrimiento => ({
  x: panel.ancho > 0 ? px.x / panel.ancho : 0,
  y: panel.alto > 0 ? px.y / panel.alto : 0,
});

/** De fracción del panel a píxeles, para mostrar. */
export const aPixeles = (c: Corrimiento | undefined, panel: { ancho: number; alto: number }) => ({
  x: (c?.x ?? 0) * panel.ancho,
  y: (c?.y ?? 0) * panel.alto,
});
