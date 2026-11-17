import { motion } from "framer-motion";
import type { ReactNode } from "react";
import { varianteWidget } from "../app/animaciones";

interface TarjetaProps {
  titulo: string;
  icono?: ReactNode;
  accion?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** Contenedor de vidrio que usan todos los widgets. */
export function Tarjeta({ titulo, icono, accion, className = "", children }: TarjetaProps) {
  return (
    <motion.section
      variants={varianteWidget}
      onClick={(e) => e.stopPropagation()}
      className={`flex min-h-0 flex-col rounded-tarjeta border border-borde bg-superficie p-5 shadow-2xl shadow-black/30 backdrop-blur-(--blur-vidrio) ${className}`}
    >
      <header className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.14em] text-texto-tenue">
        {icono}
        <h2 className="flex-1">{titulo}</h2>
        {accion}
      </header>
      <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
    </motion.section>
  );
}
