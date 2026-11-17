import { Bloque } from "../components/Bloque";
import { diasEntre, etiquetaDia } from "../lib/fechas";
import type { Responsabilidad } from "../lib/tipos";
import { responsabilidades } from "../mocks/datos";

/** "2026-11-17" → Date a medianoche local (sin corrimiento de zona horaria). */
function desdeFecha(fecha: string): Date {
  const [a, m, d] = fecha.split("-").map(Number);
  return new Date(a, m - 1, d);
}

function vencimiento(r: Responsabilidad, ahora: Date): { texto: string; clase: string } {
  if (!r.fechaLimite) return { texto: "——", clase: "text-texto-3" };
  const fecha = desdeFecha(r.fechaLimite);
  const dias = diasEntre(ahora, fecha);
  if (dias < 0) return { texto: "VENCIDA", clase: "text-alerta" };
  if (dias === 0) return { texto: "HOY", clase: "text-acento" };
  return { texto: etiquetaDia(fecha, ahora).toUpperCase(), clase: "text-texto-2" };
}

export function Responsabilidades({ ahora }: { ahora: Date }) {
  // En v0.1 el orden viene de los datos de ejemplo; en v0.2 se ordena según
  // la regla de la especificación (vencidas → hoy → próximas → sin fecha).
  const pendientes = responsabilidades.filter((r) => r.estado === "pendiente");

  return (
    <Bloque etiqueta="PENDIENTES" meta={String(pendientes.length).padStart(2, "0")} className="shrink-0">
      <ul>
        {pendientes.map((r) => {
          const v = vencimiento(r, ahora);
          return (
            <li key={r.id} className="fila -mx-2 flex items-center gap-3 px-2 py-1 text-[14px]">
              {/* En v0.2 este cuadrado se marca con un clic */}
              <span className="h-3 w-3 shrink-0 border border-linea-fuerte" />
              <span className="min-w-0 flex-1 truncate">{r.titulo}</span>
              {r.origen === "auto" && (
                <span className="shrink-0 font-mono text-[10px] tracking-[0.1em] text-texto-3">AUTO</span>
              )}
              <span className={`w-16 shrink-0 text-right font-mono text-[12px] ${v.clase}`}>{v.texto}</span>
            </li>
          );
        })}
      </ul>
    </Bloque>
  );
}
