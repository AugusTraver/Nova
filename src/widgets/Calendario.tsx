import { Tarjeta } from "../components/Tarjeta";
import { diasEntre, etiquetaDia, formatearHora } from "../lib/fechas";
import type { Evento } from "../lib/tipos";
import { eventos } from "../mocks/datos";
import { colorEvento } from "../theme/eventos";

const DIAS_VISIBLES = 4;

function agruparPorDia(lista: Evento[], hoy: Date): Map<number, Evento[]> {
  const grupos = new Map<number, Evento[]>();
  for (const evento of lista) {
    const dia = diasEntre(hoy, new Date(evento.inicio));
    if (dia < 0 || dia >= DIAS_VISIBLES) continue;
    grupos.set(dia, [...(grupos.get(dia) ?? []), evento]);
  }
  return new Map([...grupos.entries()].sort(([a], [b]) => a - b));
}

export function Calendario() {
  const hoy = new Date();
  const grupos = agruparPorDia(
    [...eventos].sort((a, b) => a.inicio.localeCompare(b.inicio)),
    hoy,
  );

  return (
    <Tarjeta titulo="Calendario" className="flex-1">
      <div className="flex flex-col gap-4">
        {[...grupos.entries()].map(([dia, lista]) => (
          <div key={dia}>
            <p className="mb-1.5 text-sm font-semibold text-texto-suave">
              {etiquetaDia(new Date(lista[0].inicio), hoy)}
            </p>
            <ul className="flex flex-col gap-1.5">
              {lista.map((evento) => (
                <li key={evento.id} className="flex items-center gap-3 text-sm">
                  <span
                    className="h-8 w-1 shrink-0 rounded-chip"
                    style={{ background: colorEvento[evento.tipo] }}
                  />
                  <span className="w-12 shrink-0 tabular-nums text-texto-tenue">
                    {evento.todoElDia ? "Todo" : formatearHora(new Date(evento.inicio))}
                  </span>
                  <span className="truncate">{evento.titulo}</span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Tarjeta>
  );
}
