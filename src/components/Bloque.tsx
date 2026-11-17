import type { ReactNode } from "react";

interface BloqueProps {
  /** Etiqueta en mayúsculas monoespaciadas, ej. "CALENDARIO". */
  etiqueta: string;
  /** Dato chico a la derecha de la etiqueta, ej. "3 PENDIENTES". */
  meta?: ReactNode;
  className?: string;
  children: ReactNode;
}

/**
 * Sección de un widget: sin caja ni fondo, solo una etiqueta con una línea
 * fina debajo. Frena el clic para que tocar un widget no cierre el panel.
 */
export function Bloque({ etiqueta, meta, className = "", children }: BloqueProps) {
  return (
    <section onClick={(e) => e.stopPropagation()} className={`flex min-h-0 flex-col ${className}`}>
      <header className="mb-3 flex items-baseline justify-between border-b border-linea pb-2 font-mono text-[11px] tracking-[0.12em] text-texto-3">
        <h2>{etiqueta}</h2>
        {meta && <span>{meta}</span>}
      </header>
      <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
    </section>
  );
}
