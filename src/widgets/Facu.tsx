import { Tarjeta } from "../components/Tarjeta";
import { cuentaRegresiva, diasEntre } from "../lib/fechas";
import { eventos, materias, progresoMaterias } from "../mocks/datos";
import { colorEvento, nombreTipoEvento } from "../theme/eventos";

export function Facu() {
  const hoy = new Date();
  const proximos = eventos
    .filter((e) => (e.tipo === "examen" || e.tipo === "entrega") && diasEntre(hoy, new Date(e.inicio)) >= 0)
    .sort((a, b) => a.inicio.localeCompare(b.inicio))
    .slice(0, 3);

  return (
    <Tarjeta titulo="Facu" className="shrink-0">
      <ul className="mb-4 flex flex-col gap-2">
        {proximos.map((e) => (
          <li key={e.id} className="flex items-baseline justify-between gap-3 text-sm">
            <span className="truncate">
              <span className="mr-2 text-xs font-semibold" style={{ color: colorEvento[e.tipo] }}>
                {nombreTipoEvento[e.tipo]}
              </span>
              {materias.find((m) => m.id === e.materiaId)?.nombre ?? e.titulo}
            </span>
            <span className="shrink-0 text-xs text-texto-suave">{cuentaRegresiva(new Date(e.inicio), hoy)}</span>
          </li>
        ))}
      </ul>

      <div className="flex flex-col gap-2.5">
        {materias.map((m) => {
          const [hechas, total] = progresoMaterias[m.id] ?? [0, 0];
          const porcentaje = total === 0 ? 0 : (hechas / total) * 100;
          return (
            <div key={m.id}>
              <div className="mb-1 flex justify-between text-xs text-texto-suave">
                <span className="truncate">{m.nombre}</span>
                <span className="tabular-nums">
                  {hechas}/{total}
                </span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-chip bg-superficie-hover">
                <div className="h-full rounded-chip" style={{ width: `${porcentaje}%`, background: m.color }} />
              </div>
            </div>
          );
        })}
      </div>
    </Tarjeta>
  );
}
