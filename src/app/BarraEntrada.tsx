import { useEffect, useRef, useState } from "react";

/**
 * Donde le escribís a Nova. Sin botón de micrófono: desde v0.5 Nova escucha
 * sola al abrir el panel (ver SPEC 5.1). En v0.1 todavía no hace nada con lo
 * que escribís.
 */
export function BarraEntrada({ abierto }: { abierto: boolean }) {
  const [texto, setTexto] = useState("");
  const campo = useRef<HTMLInputElement>(null);

  // Cada vez que se abre el panel, el foco va directo a la barra.
  useEffect(() => {
    if (abierto) campo.current?.focus();
  }, [abierto]);

  return (
    <form
      onClick={(e) => {
        e.stopPropagation();
        campo.current?.focus();
      }}
      onSubmit={(e) => {
        e.preventDefault();
        setTexto("");
      }}
      className="w-full max-w-[30rem] cursor-text"
    >
      <input
        ref={campo}
        value={texto}
        onChange={(e) => setTexto(e.target.value)}
        aria-label="Pedile algo a Nova"
        placeholder="Pedile algo a Nova"
        spellCheck={false}
        className="w-full border-b border-tinta-4 bg-transparent pb-2.5 text-center font-display text-[24px] font-[350] text-tinta caret-tinta-2 outline-none transition-colors duration-200 placeholder:italic placeholder:text-tinta-3 focus:border-tinta-3 focus-visible:outline-none"
      />
    </form>
  );
}
