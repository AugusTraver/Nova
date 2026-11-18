import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import { useAsistente } from "../store/asistente";

/** Escribir pausa la escucha (SPEC 5.1); vaciar la barra la retoma. */
function avisarEscritura(antes: string, ahora: string) {
  const pausar = antes.trim() === "" && ahora.trim() !== "";
  const reanudar = antes.trim() !== "" && ahora.trim() === "";
  if (pausar) void api.pausarEscucha().catch(() => {});
  if (reanudar) void api.reanudarEscucha().catch(() => {});
}

/**
 * Donde le escribís a Nova. Sin botón de micrófono: desde v0.5 Nova escucha
 * sola al abrir el panel (ver SPEC 5.1). Enter manda el mensaje; mientras
 * responde se puede seguir escribiendo el próximo, pero no mandarlo.
 */
export function BarraEntrada({ abierto }: { abierto: boolean }) {
  const [texto, setTexto] = useState("");
  const campo = useRef<HTMLInputElement>(null);
  const enviar = useAsistente((s) => s.enviar);
  const ocupado = useAsistente((s) => s.paso !== null);

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
        if (enviar(texto)) {
          avisarEscritura(texto, "");
          setTexto("");
        }
      }}
      className="w-full max-w-[30rem] cursor-text"
    >
      <input
        ref={campo}
        value={texto}
        onChange={(e) => {
          avisarEscritura(texto, e.target.value);
          setTexto(e.target.value);
        }}
        aria-label="Pedile algo a Nova"
        aria-busy={ocupado}
        maxLength={2000}
        enterKeyHint="send"
        placeholder="Pedile algo a Nova"
        spellCheck={false}
        className="w-full border-b border-tinta-4 bg-transparent pb-2.5 text-center font-display text-[24px] font-[350] text-tinta caret-tinta-2 outline-none transition-colors duration-200 placeholder:italic placeholder:text-tinta-3 focus:border-tinta-3 focus-visible:outline-none"
      />
    </form>
  );
}
