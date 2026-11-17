import { Tarjeta } from "../components/Tarjeta";
import { etiquetaDia, formatearHora } from "../lib/fechas";
import {
  jugadoresSeguidos,
  proximoPartidoBoca,
  proximoPartidoSeleccion,
  tablaBoca,
  ultimoResultadoBoca,
} from "../mocks/datos";

function cuando(iso: string, hoy: Date) {
  const fecha = new Date(iso);
  return `${etiquetaDia(fecha, hoy)} · ${formatearHora(fecha)}`;
}

export function Futbol() {
  const hoy = new Date();
  const ultimo = ultimoResultadoBoca;
  const gano = ultimo.golesBoca > ultimo.golesRival;

  return (
    <Tarjeta titulo="Fútbol" className="shrink-0">
      {/* Boca: protagonista */}
      <div className="mb-3 rounded-xl border border-boca-oro/25 bg-gradient-to-br from-boca-azul/40 to-boca-azul/10 p-3">
        <div className="flex items-center justify-between text-[11px] font-semibold uppercase tracking-wider text-boca-oro">
          <span>Boca · {proximoPartidoBoca.competencia}</span>
          <span>{tablaBoca.posicion}° · {tablaBoca.puntos} pts</span>
        </div>
        <p className="mt-1 font-display text-lg font-semibold">
          {proximoPartidoBoca.local ? `Boca vs. ${proximoPartidoBoca.rival}` : `${proximoPartidoBoca.rival} vs. Boca`}
        </p>
        <p className="text-sm text-texto-suave">{cuando(proximoPartidoBoca.fecha, hoy)}</p>
        <p className="mt-2 text-xs text-texto-suave">
          Último:{" "}
          <span className={gano ? "text-exito" : "text-texto"}>
            Boca {ultimo.golesBoca} – {ultimo.golesRival} {ultimo.rival}
          </span>
        </p>
      </div>

      <div className="mb-3 flex items-baseline justify-between text-sm">
        <span>
          <span className="mr-2 text-xs font-semibold text-acento">Selección</span>
          vs. {proximoPartidoSeleccion.rival}
        </span>
        <span className="text-xs text-texto-suave">{cuando(proximoPartidoSeleccion.fecha, hoy)}</span>
      </div>

      <ul className="flex flex-col gap-1 text-xs">
        {jugadoresSeguidos.map((j) => (
          <li key={j.nombre} className="flex justify-between gap-2">
            <span className="truncate text-texto-suave">
              {j.nombre} <span className="text-texto-tenue">· {j.equipo}</span>
            </span>
            <span className="shrink-0">{j.ultimaFecha}</span>
          </li>
        ))}
      </ul>
    </Tarjeta>
  );
}
