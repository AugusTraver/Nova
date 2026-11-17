import { Bloque } from "../components/Bloque";
import { etiquetaDia, formatearHora } from "../lib/fechas";
import {
  jugadoresSeguidos,
  proximoPartidoBoca,
  proximoPartidoSeleccion,
  tablaBoca,
  ultimoResultadoBoca,
} from "../mocks/datos";

function cuando(iso: string, ahora: Date): string {
  const fecha = new Date(iso);
  return `${etiquetaDia(fecha, ahora).toUpperCase()} ${formatearHora(fecha)}`;
}

export function Futbol({ ahora }: { ahora: Date }) {
  const p = proximoPartidoBoca;
  const u = ultimoResultadoBoca;
  const resultado = u.golesBoca > u.golesRival ? "G" : u.golesBoca < u.golesRival ? "P" : "E";

  return (
    <Bloque etiqueta="FÚTBOL" meta={`${tablaBoca.posicion}° · ${tablaBoca.puntos} PTS`} className="shrink-0">
      {/* Boca: protagonista */}
      <p className="font-mono text-[11px] tracking-[0.12em] text-acento">
        BOCA · {p.competencia.toUpperCase()} · {p.local ? "LOCAL" : "VISITANTE"}
      </p>
      <div className="mt-1 flex items-baseline justify-between gap-3">
        <p className="text-[22px] font-medium tracking-tight">
          {p.local ? "Boca" : p.rival} <span className="text-texto-3">—</span> {p.local ? p.rival : "Boca"}
        </p>
        <p className="shrink-0 font-mono text-[13px] text-texto-2">{cuando(p.fecha, ahora)}</p>
      </div>
      <p className="mt-1 font-mono text-[12px] text-texto-3">
        ÚLTIMO <span className="text-texto-2">{resultado} {u.golesBoca}–{u.golesRival}</span> vs {u.rival}
      </p>

      <div className="mt-4 flex flex-col">
        <div className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[14px]">
          <span className="w-16 shrink-0 font-mono text-[10px] tracking-[0.1em] text-texto-3">ARG</span>
          <span className="min-w-0 flex-1 truncate">vs {proximoPartidoSeleccion.rival}</span>
          <span className="shrink-0 font-mono text-[12px] text-texto-2">{cuando(proximoPartidoSeleccion.fecha, ahora)}</span>
        </div>
        {jugadoresSeguidos.map((j) => (
          <div key={j.nombre} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[14px]">
            <span className="w-16 shrink-0 truncate font-mono text-[10px] tracking-[0.1em] text-texto-3">
              {j.equipo.split(" ")[0].toUpperCase()}
            </span>
            <span className="min-w-0 flex-1 truncate">{j.nombre}</span>
            <span className="shrink-0 font-mono text-[12px] text-texto-2">{j.ultimaFecha}</span>
          </div>
        ))}
      </div>
    </Bloque>
  );
}
