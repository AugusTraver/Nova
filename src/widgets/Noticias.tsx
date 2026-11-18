import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { api } from "../lib/api";
import { estadoExterno } from "../lib/externos";
import { useDatos } from "../store/datos";
import { cambioDeVista } from "./animacionWidget";

/** Una fila al pie (más alto taparía la barra); "Más" muestra las siguientes. */
const POR_TANDA = 4;

/** Los titulares elegidos según tus intereses, al pie del cielo. Clic → navegador. */
export function Noticias({ ahora }: { ahora: Date }) {
  const noticias = useDatos((s) => s.noticias);
  const [tanda, setTanda] = useState(0);
  const estado = estadoExterno(noticias, ahora);
  const todas = noticias?.datos ?? [];
  const tandas = Math.max(1, Math.ceil(todas.length / POR_TANDA));
  // Si llegan menos noticias que antes, se vuelve a la primera tanda.
  const actual = tanda % tandas;
  const lista = todas.slice(actual * POR_TANDA, actual * POR_TANDA + POR_TANDA);

  return (
    <section onClick={(e) => e.stopPropagation()} aria-label="Noticias">
      {estado.tipo === "cargando" && <p className="text-center text-[15px] text-tinta-3">Buscando noticias…</p>}
      {estado.tipo === "sin_datos" && <p className="text-center text-[15px] text-tinta-3">{estado.mensaje}</p>}
      {estado.tipo === "listo" && todas.length === 0 && (
        <p className="text-center text-[15px] text-tinta-3">Nada nuevo en tus feeds por ahora.</p>
      )}
      <AnimatePresence mode="wait" initial={false}>
        <motion.ul key={actual} {...cambioDeVista} className="grid grid-cols-4 gap-x-7">
          {lista.map((n) => (
            <li key={n.id} className="min-w-0">
              <a
                href={n.url}
                title={n.resumen || n.titulo}
                onClick={(e) => {
                  e.preventDefault();
                  void api.abrirEnlace(n.url).catch((error: unknown) => console.error(error));
                }}
                className="fila -mx-2 block px-2 py-1.5 outline-none focus-visible:bg-tinta-4"
              >
                <p className="line-clamp-2 text-[15px] leading-snug">{n.titulo}</p>
                {n.resumen && <p className="mt-0.5 line-clamp-1 text-[13px] text-tinta-2">{n.resumen}</p>}
                <p className="mt-0.5 text-[13px] text-tinta-3">{n.fuente}</p>
              </a>
            </li>
          ))}
        </motion.ul>
      </AnimatePresence>
      <div className="mt-1 flex items-baseline justify-center gap-4 text-[13px]">
        {estado.tipo === "listo" && estado.aviso && <span className="text-aviso">{estado.aviso}</span>}
        {tandas > 1 && (
          <button
            type="button"
            onClick={() => setTanda(actual + 1)}
            className="rounded-md text-tinta-3 transition-colors duration-150 hover:text-tinta-2"
          >
            {actual + 1 < tandas ? "Más noticias" : "Volver a las primeras"}
          </button>
        )}
      </div>
    </section>
  );
}
