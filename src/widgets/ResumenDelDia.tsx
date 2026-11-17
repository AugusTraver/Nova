import { Bloque } from "../components/Bloque";
import { diasEntre, formatearHora } from "../lib/fechas";
import { eventos, noticias, proximoPartidoBoca, responsabilidades } from "../mocks/datos";

function saludo(hora: number): string {
  if (hora < 6) return "Buenas noches";
  if (hora < 13) return "Buen día";
  if (hora < 20) return "Buenas tardes";
  return "Buenas noches";
}

export function ResumenDelDia({ ahora }: { ahora: Date }) {
  const eventosHoy = eventos.filter((e) => diasEntre(ahora, new Date(e.inicio)) === 0);
  const urgentes = responsabilidades.filter(
    (r) => r.estado === "pendiente" && r.fechaLimite !== undefined && r.prioridad === "alta",
  );
  const juegaBocaHoy = diasEntre(ahora, new Date(proximoPartidoBoca.fecha)) === 0;

  return (
    <Bloque etiqueta="RESUMEN" className="shrink-0">
      <p className="text-[28px] font-medium leading-tight tracking-tight">{saludo(ahora.getHours())}.</p>
      <p className="mt-2 text-[15px] leading-relaxed text-texto-2">
        Hoy tenés <span className="text-texto">{eventosHoy.length} eventos</span>
        {eventosHoy[0] && <> desde las {formatearHora(new Date(eventosHoy[0].inicio))}</>} y{" "}
        <span className="text-texto">{urgentes.length} cosas urgentes</span>.
        {juegaBocaHoy && (
          <>
            {" "}
            Juega Boca a las{" "}
            <span className="text-acento">{formatearHora(new Date(proximoPartidoBoca.fecha))}</span>.
          </>
        )}
      </p>
      <p className="mt-3 truncate font-mono text-xs text-texto-3">↳ {noticias[0].titulo}</p>
    </Bloque>
  );
}
