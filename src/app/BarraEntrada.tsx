import { motion } from "framer-motion";
import { useState } from "react";
import { varianteWidget } from "./animaciones";

/** Barra para escribir o hablar. En v0.1 todavía no hace nada. */
export function BarraEntrada() {
  const [texto, setTexto] = useState("");

  return (
    <motion.form
      variants={varianteWidget}
      onClick={(e) => e.stopPropagation()}
      onSubmit={(e) => {
        e.preventDefault();
        setTexto("");
      }}
      className="flex w-full max-w-2xl items-center gap-2 rounded-chip border border-borde bg-superficie p-1.5 pl-5 shadow-2xl shadow-black/30 backdrop-blur-(--blur-vidrio)"
    >
      <input
        autoFocus
        value={texto}
        onChange={(e) => setTexto(e.target.value)}
        placeholder="Pedile algo a Nova…"
        className="min-w-0 flex-1 bg-transparent text-base text-texto outline-none placeholder:text-texto-tenue"
      />
      <button
        type="button"
        aria-label="Hablar (disponible en v0.5)"
        title="Hablar (disponible en v0.5)"
        className="grid h-10 w-10 place-items-center rounded-chip bg-acento/20 text-acento transition-colors hover:bg-acento/30"
      >
        <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
          <rect x="9" y="3" width="6" height="12" rx="3" />
          <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
        </svg>
      </button>
    </motion.form>
  );
}
