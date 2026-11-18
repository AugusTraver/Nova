import { Seccion } from "../components/Seccion";
import { diasEntre, etiquetaDia, formatearHora } from "../lib/fechas";
import type { Evento } from "../lib/tipos";
import { eventos } from "../mocks/datos";
import { tipoDestacado } from "../theme/eventos";

const DIAS_VISIBLES = 4;

function agruparPorDia(lista: Evento[], hoy: Date): [number, Evento[]][] {
  const grupos = new Map<number, Evento[]>();
  for (const evento of lista) {
    const dia = diasEntre(hoy, new Date(evento.inicio));
    if (dia < 0 || dia >= DIAS_VISIBLES) continue;
    grupos.set(dia, [...(grupos.get(dia) ?? []), evento]);
  }
  return [...grupos.entries()].sort(([a], [b]) => a - b);
}

/** Los próximos días, agrupados: la hora en grande a la izquierda, como una agenda. */
export function Calendario({ ahora }: { ahora: Date }) {
  const grupos = agruparPorDia([...eventos].sort((a, b) => a.inicio.localeCompare(b.inicio)), ahora);

  return (
    <Seccion titulo="Agenda" className="shrink-0">
      <div className="flex flex-col gap-4">
        {grupos.map(([dia, lista]) => (
          <div key={dia} className="grid grid-cols-[5.5rem_1fr] gap-x-4">
            <p className="pt-1 text-[14px] text-tinta-3">{etiquetaDia(new Date(lista[0].inicio), ahora)}</p>
            <ul className="flex flex-col">
              {lista.map((evento) => (
                <li key={evento.id} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1">
                  <span className="w-12 shrink-0 font-display text-[18px] text-tinta-2">
                    {evento.todoElDia ? "·" : formatearHora(new Date(evento.inicio))}
                  </span>
                  <span
                    className={`min-w-0 flex-1 truncate text-[16px] ${tipoDestacado[evento.tipo] ? "text-aviso" : ""}`}
                  >
                    {evento.titulo}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Seccion>
  );
}
