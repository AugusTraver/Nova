// Las piezas de la tarjeta "Resumen del día" que salen de internet. La
// agenda y los pendientes se cuentan en el widget (ResumenDelDia.tsx).
import { esHoy, golesDe, nombreCorto, rivalDe } from "./futbol";
import { formatearHora } from "./fechas";
import type { Clima, VistaFutbol } from "./tipos";

/** "A las 21:30 juega Boca con Racing." o "Boca está jugando: va 2 a 1 con Racing." */
export function futbolDeHoy(vista: VistaFutbol | null, ahora: Date): string | null {
  if (!vista) return null;
  const frases: string[] = [];
  for (const e of vista.equipos) {
    const nombre = e.clave === "boca" ? "Boca" : "la Selección";
    if (e.enVivo) {
      const rival = rivalDe(e.enVivo, e.id);
      frases.push(`${mayuscula(nombre)} está jugando: va ${golesDe(e.enVivo, e.id)} a ${golesDe(e.enVivo, rival.id)} con ${nombreCorto(rival)}.`);
    } else if (e.proximo && esHoy(e.proximo.fecha, ahora)) {
      const rival = rivalDe(e.proximo, e.id);
      frases.push(`A las ${formatearHora(new Date(e.proximo.fecha))} juega ${nombre} con ${nombreCorto(rival)}.`);
    }
  }
  return frases.length > 0 ? frases.join(" ") : null;
}

/** "22°, casi despejado". */
export function climaCorto(c: Clima): string {
  return `${Math.round(c.temperatura)}°, ${c.descripcion}`;
}

function mayuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}
