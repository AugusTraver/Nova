import { formatearHora } from "../lib/fechas";

const DIAS = ["DOM", "LUN", "MAR", "MIÉ", "JUE", "VIE", "SÁB"];
const MESES = ["ENE", "FEB", "MAR", "ABR", "MAY", "JUN", "JUL", "AGO", "SEP", "OCT", "NOV", "DIC"];

/** Línea superior: marca, fecha y reloj en vivo. */
export function BarraEstado({ ahora }: { ahora: Date }) {
  const segundos = String(ahora.getSeconds()).padStart(2, "0");
  return (
    <div className="flex items-baseline justify-between font-mono text-[11px] tracking-[0.12em] text-texto-3">
      <span className="text-texto-2">NOVA</span>
      <span>
        {DIAS[ahora.getDay()]} {ahora.getDate()} {MESES[ahora.getMonth()]}
        <span className="mx-3 text-linea-fuerte">/</span>
        <span className="text-texto">{formatearHora(ahora)}</span>
        <span className="text-texto-3">:{segundos}</span>
      </span>
    </div>
  );
}
