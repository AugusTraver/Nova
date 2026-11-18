import { api } from "../lib/api";
import { aFecha, fechaLarga, formatearHora, horaDe } from "../lib/fechas";
import { grupoDe } from "../lib/responsabilidades";
import { climaCorto, futbolDeHoy } from "../lib/resumen";
import { useDatos } from "../store/datos";
import { saludo } from "../theme/cielo";

function plural(n: number, uno: string, varios: string, ninguno: string): string {
  if (n === 0) return ninguno;
  return `${n} ${n === 1 ? uno : varios}`;
}

/** Saludo grande, la fecha (con el clima) y el día contado en una oración. */
export function ResumenDelDia({ ahora }: { ahora: Date }) {
  const ocurrencias = useDatos((s) => s.ocurrencias);
  const responsabilidades = useDatos((s) => s.responsabilidades);
  const futbol = useDatos((s) => s.futbol);
  const noticias = useDatos((s) => s.noticias);
  const clima = useDatos((s) => s.clima);
  const hoy = aFecha(ahora);

  const eventosHoy = ocurrencias.filter((o) => o.fecha === hoy);
  const primeroConHora = eventosHoy.find((o) => !o.evento.todoElDia);
  const paraHoy = responsabilidades.filter((r) => r.estado === "pendiente" && ["vencidas", "hoy"].includes(grupoDe(r, ahora)));
  const partido = futbolDeHoy(futbol?.datos ?? null, ahora);
  const destacada = noticias?.datos?.[0] ?? null;
  const tiempo = clima?.mostrar && clima.clima.datos ? climaCorto(clima.clima.datos) : null;

  return (
    <section onClick={(e) => e.stopPropagation()} className="text-center">
      <h1 className="font-display text-[56px] font-[350] leading-[0.95] tracking-[-0.02em]">{saludo(ahora)}.</h1>
      <p className="mt-3 text-[17px] text-tinta-2">
        {fechaLarga(ahora)}, {formatearHora(ahora)}
        {tiempo && <span className="text-tinta-3"> · {tiempo}</span>}
      </p>
      <p className="mx-auto mt-4 max-w-[46ch] text-[17px] leading-relaxed text-tinta-2">
        Hoy tenés <span className="text-tinta">{plural(eventosHoy.length, "evento", "eventos", "ningún evento")}</span>
        {primeroConHora && <> desde las {horaDe(primeroConHora.inicio)}</>} y{" "}
        <span className="text-tinta">{plural(paraHoy.length, "cosa pendiente", "cosas pendientes", "nada pendiente")}</span> para hoy.
        {partido && <span className="text-aviso"> {partido}</span>}
      </p>
      {destacada && (
        <p className="mx-auto mt-2 max-w-[44ch] truncate text-[14px] text-tinta-3">
          Lo más destacado:{" "}
          <a
            href={destacada.url}
            onClick={(e) => {
              e.preventDefault();
              void api.abrirEnlace(destacada.url).catch((error: unknown) => console.error(error));
            }}
            className="rounded-sm text-tinta-2 underline decoration-tinta-4 underline-offset-4 transition-colors hover:text-tinta"
          >
            {destacada.titulo}
          </a>
        </p>
      )}
    </section>
  );
}
