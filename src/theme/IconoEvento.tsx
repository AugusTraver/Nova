import type { TipoEvento } from "../lib/tipos";

/*
 * Un ícono de línea fina por tipo de evento (SPEC 4.1). Van en el tono de la
 * tinta, o en el color de la materia si el evento tiene una: así el tipo se
 * lee por la forma y la materia por el color, sin competir entre sí.
 */
const TRAZOS: Record<TipoEvento, string[]> = {
  // Libro abierto
  clase: ["M8 4.5C6.6 3.6 4.7 3.3 2.5 3.5v8.5c2.2-.2 4.1.1 5.5 1", "M8 4.5c1.4-.9 3.3-1.2 5.5-1v8.5c-2.2-.2-4.1.1-5.5 1", "M8 4.5V13"],
  // Lápiz
  examen: ["M10.6 2.9l2.5 2.5L6 12.5l-3.2.7.7-3.2z", "M9.2 4.3l2.5 2.5"],
  // Hoja con la esquina doblada
  entrega: ["M4 2.5h5.5L12.5 5.5V13.5H4z", "M9.5 2.5v3h3"],
  // Regalo
  cumpleanos: ["M2.8 6h10.4v2.3H2.8z", "M3.8 8.3v5.2h8.4V8.3", "M8 6v7.5", "M8 6C6.2 6 4.8 5 5.4 3.9 6 2.8 7.6 4 8 6c.4-2 2-3.2 2.6-2.1C11.2 5 9.8 6 8 6"],
  // Pelota
  partido: ["M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11z", "M8 5.4l2.2 1.6-.8 2.6H6.6l-.8-2.6z", "M8 5.4V2.6", "M10.2 7l2.6-.9", "M9.4 9.6l1.6 2.3", "M6.6 9.6 5 11.9", "M5.8 7l-2.6-.9"],
  // Punto
  otro: ["M8 6.6a1.4 1.4 0 1 0 0 2.8 1.4 1.4 0 0 0 0-2.8z"],
};

export function IconoEvento({ tipo, color, className = "" }: { tipo: TipoEvento; color?: string; className?: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      aria-hidden
      className={`h-[15px] w-[15px] shrink-0 ${className}`}
      fill="none"
      stroke={color ?? "var(--color-tinta-3)"}
      strokeWidth={1.3}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {TRAZOS[tipo].map((d) => (
        <path key={d} d={d} fill={tipo === "otro" ? (color ?? "var(--color-tinta-3)") : "none"} />
      ))}
    </svg>
  );
}
