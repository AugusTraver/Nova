import { Bloque } from "../components/Bloque";
import { Segmentos } from "../components/Segmentos";
import { diasEntre } from "../lib/fechas";
import { eventos, materias, progresoMaterias } from "../mocks/datos";
import { etiquetaTipoEvento } from "../theme/eventos";

/** "HOY", "1D", "12D" */
function faltan(dias: number): string {
  return dias === 0 ? "HOY" : `${dias}D`;
}

export function Facu({ ahora }: { ahora: Date }) {
  const proximos = eventos
    .filter((e) => (e.tipo === "examen" || e.tipo === "entrega") && diasEntre(ahora, new Date(e.inicio)) >= 0)
    .sort((a, b) => a.inicio.localeCompare(b.inicio))
    .slice(0, 3);

  return (
    <Bloque etiqueta="FACU" className="shrink-0">
      <ul className="mb-4">
        {proximos.map((e) => {
          const dias = diasEntre(ahora, new Date(e.inicio));
          return (
            <li key={e.id} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[14px]">
              <span className="w-16 shrink-0 font-mono text-[10px] tracking-[0.1em] text-texto-3">
                {etiquetaTipoEvento[e.tipo]}
              </span>
              <span className="min-w-0 flex-1 truncate">
                {materias.find((m) => m.id === e.materiaId)?.nombre ?? e.titulo}
              </span>
              <span className={`shrink-0 font-mono text-[13px] ${dias <= 1 ? "text-acento" : "text-texto-2"}`}>
                {faltan(dias)}
              </span>
            </li>
          );
        })}
      </ul>

      <div className="flex flex-col gap-2">
        {materias.map((m) => {
          const [hechas, total] = progresoMaterias[m.id] ?? [0, 0];
          return (
            <div key={m.id} className="flex items-center gap-3">
              <span className="min-w-0 flex-1 truncate text-[13px] text-texto-2">{m.nombre}</span>
              <Segmentos hechas={hechas} total={total} />
              <span className="w-8 shrink-0 text-right font-mono text-[12px] text-texto-3">
                {hechas}/{total}
              </span>
            </div>
          );
        })}
      </div>
    </Bloque>
  );
}
