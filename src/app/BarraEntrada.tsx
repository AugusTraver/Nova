import { useEffect, useRef, useState } from "react";

/**
 * Barra de texto tipo línea de comandos. Sin botón de micrófono: desde v0.5
 * Nova escucha sola al abrir el panel (ver SPEC 5.1). En v0.1 todavía no
 * hace nada con lo que escribís.
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
      className="flex w-full max-w-3xl cursor-text items-baseline gap-3 border-b border-linea-fuerte pb-3 font-mono text-[15px]"
    >
      <span className="text-acento">&gt;</span>
      <div className="relative min-w-0 flex-1">
        <input
          ref={campo}
          value={texto}
          onChange={(e) => setTexto(e.target.value)}
          aria-label="Pedile algo a Nova"
          className={`w-full bg-transparent text-texto outline-none ${texto === "" ? "caret-transparent" : "caret-acento"}`}
          spellCheck={false}
        />
        {/* Mientras está vacío: cursor de bloque que parpadea + texto de ayuda
            (el cursor nativo se oculta para no ver dos) */}
        {texto === "" && (
          <span className="pointer-events-none absolute inset-0 flex items-baseline text-texto-3">
            <span className="cursor-parpadeo mr-2 inline-block h-[1.05em] w-[0.6em] translate-y-[0.15em] bg-acento" />
            pedile algo a nova
          </span>
        )}
      </div>
      <span className="shrink-0 text-[11px] tracking-[0.12em] text-texto-3">ESC CIERRA</span>
    </form>
  );
}
