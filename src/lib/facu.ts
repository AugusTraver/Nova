// Cálculos del widget Facu.
import { aFecha } from "./fechas";
import type { Materia, Ocurrencia, Responsabilidad } from "./tipos";

export interface Progreso {
  hechas: number;
  total: number;
}

/** Cuántas responsabilidades de cada materia están hechas, sobre el total. */
export function progresoPorMateria(
  materias: readonly Materia[],
  responsabilidades: readonly Responsabilidad[],
): Map<number, Progreso> {
  const progreso = new Map<number, Progreso>(materias.map((m) => [m.id, { hechas: 0, total: 0 }]));
  for (const r of responsabilidades) {
    const p = r.materiaId === null ? undefined : progreso.get(r.materiaId);
    if (!p) continue;
    p.total += 1;
    if (r.estado === "hecha") p.hechas += 1;
  }
  return progreso;
}

/** Los próximos exámenes y entregas (de hoy en adelante), en orden. */
export function proximasEvaluaciones(ocurrencias: readonly Ocurrencia[], hoy: Date, cuantas: number): Ocurrencia[] {
  const dia = aFecha(hoy);
  return ocurrencias
    .filter((o) => (o.evento.tipo === "examen" || o.evento.tipo === "entrega") && o.fecha >= dia)
    .sort((a, b) => a.inicio.localeCompare(b.inicio))
    .slice(0, cuantas);
}
