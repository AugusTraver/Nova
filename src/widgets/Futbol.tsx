import { Seccion } from "../components/Seccion";
import { diasEntre, etiquetaDia, formatearHora } from "../lib/fechas";
import {
  jugadoresSeguidos,
  proximoPartidoBoca,
  proximoPartidoSeleccion,
  tablaBoca,
  ultimoResultadoBoca,
} from "../mocks/datos";

function cuando(iso: string, ahora: Date): string {
  const fecha = new Date(iso);
  return `${etiquetaDia(fecha, ahora)}, ${formatearHora(fecha)}`;
}

/** Boca de protagonista; abajo, la Selección y los jugadores que sigue. */
export function Futbol({ ahora }: { ahora: Date }) {
  const p = proximoPartidoBoca;
  const u = ultimoResultadoBoca;
  const verbo = u.golesBoca > u.golesRival ? "Le ganó" : u.golesBoca < u.golesRival ? "Perdió" : "Empató";
  const esHoy = diasEntre(ahora, new Date(p.fecha)) === 0;

  return (
    <Seccion titulo="Boca" detalle={`${tablaBoca.posicion}.º con ${tablaBoca.puntos} puntos`} className="shrink-0">
      <p className="font-display text-[38px] font-[380] leading-[1.05] tracking-[-0.015em]">
        {p.local ? "Boca" : p.rival} <span className="text-tinta-3">y</span> {p.local ? p.rival : "Boca"}
      </p>
      <p className="mt-1.5 text-[16px] text-tinta-2">
        <span className={esHoy ? "text-aviso" : ""}>{cuando(p.fecha, ahora)}</span>, {p.local ? "en la Bombonera" : "de visitante"}
      </p>
      <p className="mt-1 text-[14px] text-tinta-3">
        {verbo} {u.golesBoca} a {u.golesRival} {verbo === "Le ganó" ? "a" : "con"} {u.rival}
      </p>

      <ul className="mt-5 flex flex-col text-[15px]">
        <li className="fila -mx-2 flex items-baseline gap-3 px-2 py-1">
          <span className="min-w-0 flex-1 truncate">Argentina y {proximoPartidoSeleccion.rival}</span>
          <span className="shrink-0 text-[14px] text-tinta-3">{cuando(proximoPartidoSeleccion.fecha, ahora)}</span>
        </li>
        {jugadoresSeguidos.map((j) => (
          <li key={j.nombre} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1">
            <span className="min-w-0 flex-1 truncate">
              {j.nombre} <span className="text-tinta-3">en {j.equipo}</span>
            </span>
            <span className="shrink-0 text-[14px] text-tinta-3">{j.ultimaFecha}</span>
          </li>
        ))}
      </ul>
    </Seccion>
  );
}
