import { Seccion } from "../components/Seccion";
import { diasEntre, etiquetaDia } from "../lib/fechas";
import type { Responsabilidad } from "../lib/tipos";
import { responsabilidades } from "../mocks/datos";

/** "2026-11-17" → Date a medianoche local (sin corrimiento de zona horaria). */
function desdeFecha(fecha: string): Date {
  const [a, m, d] = fecha.split("-").map(Number);
  return new Date(a, m - 1, d);
}

function vencimiento(r: Responsabilidad, ahora: Date): { texto: string; clase: string } {
  if (!r.fechaLimite) return { texto: "", clase: "" };
  const fecha = desdeFecha(r.fechaLimite);
  const dias = diasEntre(ahora, fecha);
  if (dias < 0) return { texto: "Venció", clase: "text-aviso" };
  if (dias === 0) return { texto: "Hoy", clase: "text-aviso" };
  return { texto: etiquetaDia(fecha, ahora), clase: "text-tinta-3" };
}

export function Responsabilidades({ ahora }: { ahora: Date }) {
  // En v0.1 el orden viene de los datos de ejemplo; en v0.2 se ordena según
  // la regla de la especificación (vencidas → hoy → próximas → sin fecha).
  const pendientes = responsabilidades.filter((r) => r.estado === "pendiente");

  return (
    <Seccion titulo="Pendientes" detalle={pendientes.length} className="shrink-0">
      <ul className="flex flex-col">
        {pendientes.map((r) => {
          const v = vencimiento(r, ahora);
          return (
            <li key={r.id} className="fila -mx-2 flex items-center gap-3 px-2 py-1 text-[16px]">
              {/* En v0.2 este círculo se marca con un clic */}
              <span className="h-3.5 w-3.5 shrink-0 rounded-full border-[1.5px] border-tinta-3" />
              <span className="min-w-0 flex-1 truncate">{r.titulo}</span>
              <span className={`shrink-0 text-[14px] ${v.clase}`}>{v.texto}</span>
            </li>
          );
        })}
      </ul>
    </Seccion>
  );
}
