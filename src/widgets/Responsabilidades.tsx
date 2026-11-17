import { Tarjeta } from "../components/Tarjeta";
import { diasEntre, etiquetaDia } from "../lib/fechas";
import type { Responsabilidad } from "../lib/tipos";
import { responsabilidades } from "../mocks/datos";

/** "2026-11-17" → Date a medianoche local (sin corrimiento de zona horaria). */
function desdeFecha(fecha: string): Date {
  const [a, m, d] = fecha.split("-").map(Number);
  return new Date(a, m - 1, d);
}

function etiquetaVencimiento(r: Responsabilidad, hoy: Date) {
  if (!r.fechaLimite) return { texto: "Sin fecha", clase: "text-texto-tenue" };
  const fecha = desdeFecha(r.fechaLimite);
  const dias = diasEntre(hoy, fecha);
  if (dias < 0) return { texto: "Vencida", clase: "text-alerta" };
  if (dias === 0) return { texto: "Hoy", clase: "text-aviso" };
  return { texto: etiquetaDia(fecha, hoy), clase: "text-texto-suave" };
}

export function Responsabilidades() {
  const hoy = new Date();
  // En v0.1 el orden viene dado por los datos de ejemplo; en v0.2 se ordena
  // según la regla de la especificación (vencidas → hoy → próximas → sin fecha).
  const pendientes = responsabilidades.filter((r) => r.estado === "pendiente");

  return (
    <Tarjeta
      titulo="Responsabilidades"
      className="shrink-0"
      accion={<span className="text-texto-tenue normal-case tracking-normal">{pendientes.length} pendientes</span>}
    >
      <ul className="flex flex-col gap-2">
        {pendientes.map((r) => {
          const venc = etiquetaVencimiento(r, hoy);
          return (
            <li key={r.id} className="flex items-center gap-3 text-sm">
              <span className="h-4 w-4 shrink-0 rounded-chip border-2 border-borde-fuerte" />
              <span className="flex-1 truncate">{r.titulo}</span>
              {r.origen === "auto" && (
                <span className="rounded-chip bg-acento/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-acento">
                  auto
                </span>
              )}
              <span className={`w-16 shrink-0 text-right text-xs ${venc.clase}`}>{venc.texto}</span>
            </li>
          );
        })}
      </ul>
    </Tarjeta>
  );
}
