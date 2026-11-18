import type { ReactNode } from "react";

interface SeccionProps {
  titulo: string;
  /** Dato chico al lado del título, ej. "4 pendientes". */
  detalle?: ReactNode;
  className?: string;
  children: ReactNode;
}

/**
 * Un widget flotando sobre el cielo: sin caja ni fondo, solo un título en
 * itálica y su contenido. Frena el clic para que tocar un widget no cierre el panel.
 */
export function Seccion({ titulo, detalle, className = "", children }: SeccionProps) {
  return (
    <section onClick={(e) => e.stopPropagation()} className={`flex min-h-0 flex-col ${className}`}>
      <header className="mb-3 flex items-baseline gap-3">
        <h2 className="font-display text-[19px] italic text-tinta-2">{titulo}</h2>
        {detalle && <span className="text-[13px] text-tinta-3">{detalle}</span>}
      </header>
      <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
    </section>
  );
}
