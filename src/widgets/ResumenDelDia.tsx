import { Tarjeta } from "../components/Tarjeta";
import { diasEntre, fechaLarga, formatearHora } from "../lib/fechas";
import { eventos, noticias, proximoPartidoBoca, responsabilidades } from "../mocks/datos";

function saludo(hora: number): string {
  if (hora < 6) return "Buenas noches";
  if (hora < 13) return "Buen día";
  if (hora < 20) return "Buenas tardes";
  return "Buenas noches";
}

export function ResumenDelDia() {
  const hoy = new Date();
  const eventosHoy = eventos.filter((e) => diasEntre(hoy, new Date(e.inicio)) === 0);
  const urgentes = responsabilidades.filter(
    (r) => r.estado === "pendiente" && r.fechaLimite !== undefined && r.prioridad === "alta",
  );
  const juegaBocaHoy = diasEntre(hoy, new Date(proximoPartidoBoca.fecha)) === 0;

  return (
    <Tarjeta titulo="Resumen del día" className="shrink-0">
      <p className="font-display text-2xl font-semibold leading-tight">{saludo(hoy.getHours())}</p>
      <p className="mb-3 text-sm text-texto-suave">{fechaLarga(hoy)}</p>
      <ul className="flex flex-col gap-1.5 text-sm text-texto-suave">
        <li>
          <span className="text-texto">{eventosHoy.length}</span> eventos hoy
          {eventosHoy[0] && ` · el primero a las ${formatearHora(new Date(eventosHoy[0].inicio))}`}
        </li>
        <li>
          <span className="text-texto">{urgentes.length}</span> responsabilidades urgentes
        </li>
        {juegaBocaHoy && (
          <li className="text-boca-oro">Juega Boca a las {formatearHora(new Date(proximoPartidoBoca.fecha))}</li>
        )}
        <li className="truncate">📰 {noticias[0].titulo}</li>
      </ul>
    </Tarjeta>
  );
}
