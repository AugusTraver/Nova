// Agrupación de la agenda por día.
import { aFecha, sumarDias } from "./fechas";
import type { Ocurrencia } from "./tipos";

export interface DiaAgenda {
  fecha: string;
  ocurrencias: Ocurrencia[];
}

/**
 * Los próximos `dias` días a partir de hoy, solo los que tienen algo.
 * Un evento de varios días aparece en cada día que dura (dentro del rango).
 * Llegan ya ordenadas desde Rust (todo el día primero, después por hora).
 */
export function agruparPorDia(ocurrencias: readonly Ocurrencia[], hoy: Date, dias: number): DiaAgenda[] {
  const primero = aFecha(hoy);
  const ultimo = sumarDias(primero, dias - 1);
  const porDia = new Map<string, Ocurrencia[]>();

  for (const o of ocurrencias) {
    const finDia = o.fin ? o.fin.slice(0, 10) : o.fecha;
    // Para eventos con hora, el día de fin sale del texto ISO; alcanza porque
    // se guardan con la zona del sistema.
    for (let dia = o.fecha < primero ? primero : o.fecha; dia <= finDia && dia <= ultimo; dia = sumarDias(dia, 1)) {
      porDia.set(dia, [...(porDia.get(dia) ?? []), o]);
    }
  }
  return [...porDia.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([fecha, lista]) => ({ fecha, ocurrencias: lista }));
}
