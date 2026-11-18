import { diasEntre, fechaLarga, formatearHora } from "../lib/fechas";
import { eventos, proximoPartidoBoca, responsabilidades } from "../mocks/datos";
import { saludo } from "../theme/cielo";

function plural(n: number, uno: string, varios: string, ninguno: string): string {
  if (n === 0) return ninguno;
  return `${n} ${n === 1 ? uno : varios}`;
}

/** Saludo grande, la fecha y el día contado en una oración. */
export function ResumenDelDia({ ahora }: { ahora: Date }) {
  const eventosHoy = eventos
    .filter((e) => diasEntre(ahora, new Date(e.inicio)) === 0)
    .sort((a, b) => a.inicio.localeCompare(b.inicio));
  const paraHoy = responsabilidades.filter(
    (r) => r.estado === "pendiente" && r.fechaLimite !== undefined && diasEntre(ahora, new Date(`${r.fechaLimite}T00:00`)) <= 0,
  );
  const juegaBocaHoy = diasEntre(ahora, new Date(proximoPartidoBoca.fecha)) === 0;

  return (
    <section onClick={(e) => e.stopPropagation()} className="text-center">
      <h1 className="font-display text-[56px] font-[350] leading-[0.95] tracking-[-0.02em]">{saludo(ahora)}.</h1>
      <p className="mt-3 text-[17px] text-tinta-2">
        {fechaLarga(ahora)}, {formatearHora(ahora)}
      </p>
      <p className="mx-auto mt-4 max-w-[46ch] text-[17px] leading-relaxed text-tinta-2">
        Hoy tenés <span className="text-tinta">{plural(eventosHoy.length, "evento", "eventos", "ningún evento")}</span>
        {eventosHoy[0] && <> desde las {formatearHora(new Date(eventosHoy[0].inicio))}</>} y{" "}
        <span className="text-tinta">{plural(paraHoy.length, "cosa pendiente", "cosas pendientes", "nada pendiente")}</span> para hoy.
        {juegaBocaHoy && <> A las {formatearHora(new Date(proximoPartidoBoca.fecha))} juega Boca.</>}
      </p>
    </section>
  );
}
