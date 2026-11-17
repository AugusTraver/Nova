import { Bloque } from "../components/Bloque";
import { diasEntre, etiquetaDia, formatearHora } from "../lib/fechas";
import type { Evento } from "../lib/tipos";
import { eventos } from "../mocks/datos";
import { etiquetaTipoEvento, tipoDestacado } from "../theme/eventos";

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

export function Calendario({ ahora }: { ahora: Date }) {
  const grupos = agruparPorDia([...eventos].sort((a, b) => a.inicio.localeCompare(b.inicio)), ahora);

  return (
    <Bloque etiqueta="CALENDARIO" meta={`${DIAS_VISIBLES} DÍAS`} className="flex-1">
      <div className="flex flex-col gap-3">
        {grupos.map(([dia, lista]) => (
          <div key={dia}>
            <p className="mb-1 font-mono text-[11px] uppercase tracking-[0.12em] text-texto-3">
              {etiquetaDia(new Date(lista[0].inicio), ahora)}
            </p>
            <ul>
              {lista.map((evento) => (
                <li key={evento.id} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[14px]">
                  <span className="w-11 shrink-0 font-mono text-[13px] text-texto-2">
                    {evento.todoElDia ? "——" : formatearHora(new Date(evento.inicio))}
                  </span>
                  <span className="min-w-0 flex-1 truncate">{evento.titulo}</span>
                  <span
                    className={`shrink-0 font-mono text-[10px] tracking-[0.1em] ${
                      tipoDestacado[evento.tipo] ? "text-acento" : "text-texto-3"
                    }`}
                  >
                    {etiquetaTipoEvento[evento.tipo]}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Bloque>
  );
}
