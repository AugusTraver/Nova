// Reglas de orden y agrupación de responsabilidades (SPEC 4.2).
import { aFecha } from "./fechas";
import type { Prioridad, Responsabilidad } from "./tipos";

export type Grupo = "vencidas" | "hoy" | "proximas" | "sinFecha";

const ORDEN_GRUPO: Record<Grupo, number> = { vencidas: 0, hoy: 1, proximas: 2, sinFecha: 3 };
const ORDEN_PRIORIDAD: Record<Prioridad, number> = { alta: 0, media: 1, baja: 2 };

/** En qué grupo cae una responsabilidad según su fecha límite. */
export function grupoDe(r: Pick<Responsabilidad, "fechaLimite">, hoy: Date): Grupo {
  if (!r.fechaLimite) return "sinFecha";
  const dia = aFecha(hoy);
  if (r.fechaLimite < dia) return "vencidas";
  if (r.fechaLimite === dia) return "hoy";
  return "proximas";
}

/**
 * Orden para mostrar: vencidas → hoy → próximas → sin fecha.
 * Dentro de cada grupo, la fecha más cercana primero; a igual fecha, la de
 * mayor prioridad; y si no, la más vieja (por id), para que el orden no salte.
 */
export function ordenar(lista: readonly Responsabilidad[], hoy: Date): Responsabilidad[] {
  return [...lista].sort((a, b) => {
    const porGrupo = ORDEN_GRUPO[grupoDe(a, hoy)] - ORDEN_GRUPO[grupoDe(b, hoy)];
    if (porGrupo !== 0) return porGrupo;
    const porFecha = (a.fechaLimite ?? "").localeCompare(b.fechaLimite ?? "");
    if (porFecha !== 0) return porFecha;
    const porPrioridad = ORDEN_PRIORIDAD[a.prioridad] - ORDEN_PRIORIDAD[b.prioridad];
    if (porPrioridad !== 0) return porPrioridad;
    return a.id - b.id;
  });
}

/**
 * Las que se muestran en el widget: las pendientes, más las que marcaste
 * recién (`recienHechas`), que se quedan un momento tachadas antes de irse
 * (así se ve la animación y se puede deshacer).
 */
export function visibles(
  lista: readonly Responsabilidad[],
  recienHechas: ReadonlySet<number>,
  hoy: Date,
): Responsabilidad[] {
  return ordenar(
    lista.filter((r) => r.estado === "pendiente" || recienHechas.has(r.id)),
    hoy,
  );
}
