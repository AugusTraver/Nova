import { Seccion } from "../components/Seccion";
import { cuentaRegresiva, diasEntre } from "../lib/fechas";
import { eventos, materias, progresoMaterias } from "../mocks/datos";
import { nombreTipoEvento } from "../theme/eventos";

/** Lo próximo que se rinde o se entrega, y cuánto se avanzó en cada materia. */
export function Facu({ ahora }: { ahora: Date }) {
  const proximos = eventos
    .filter((e) => (e.tipo === "examen" || e.tipo === "entrega") && diasEntre(ahora, new Date(e.inicio)) >= 0)
    .sort((a, b) => a.inicio.localeCompare(b.inicio))
    .slice(0, 3);

  return (
    <Seccion titulo="Facu" className="shrink-0">
      <ul className="mb-5 flex flex-col">
        {proximos.map((e) => {
          const fecha = new Date(e.inicio);
          const cerca = diasEntre(ahora, fecha) <= 1;
          return (
            <li key={e.id} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[16px]">
              <span className="min-w-0 flex-1 truncate">
                {nombreTipoEvento[e.tipo]} de {materias.find((m) => m.id === e.materiaId)?.nombre ?? e.titulo}
              </span>
              <span className={`shrink-0 text-[14px] ${cerca ? "text-aviso" : "text-tinta-3"}`}>
                {cuentaRegresiva(fecha, ahora)}
              </span>
            </li>
          );
        })}
      </ul>

      <div className="flex flex-col gap-2.5">
        {materias.map((m) => {
          const [hechas, total] = progresoMaterias[m.id] ?? [0, 0];
          const porcentaje = total === 0 ? 0 : (hechas / total) * 100;
          return (
            <div key={m.id} className="grid grid-cols-[1fr_7rem_2.5rem] items-center gap-3 text-[14px] text-tinta-2">
              <span className="truncate">{m.nombre}</span>
              <span
                className="h-[3px] overflow-hidden rounded-full bg-tinta-4"
                role="progressbar"
                aria-valuenow={hechas}
                aria-valuemax={total}
                aria-label={`${m.nombre}: ${hechas} de ${total}`}
              >
                <span className="block h-full rounded-full bg-tinta-2" style={{ width: `${porcentaje}%` }} />
              </span>
              <span className="text-right text-tinta-3">
                {hechas}/{total}
              </span>
            </div>
          );
        })}
      </div>
    </Seccion>
  );
}
