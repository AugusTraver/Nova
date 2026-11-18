import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { EASE_SALIDA } from "../app/animaciones";
import { AccionTitulo, ErrorWidget } from "../components/Formulario";
import { Seccion } from "../components/Seccion";
import { desdeFecha, etiquetaDia } from "../lib/fechas";
import { grupoDe, visibles } from "../lib/responsabilidades";
import type { Responsabilidad } from "../lib/tipos";
import { useDatos } from "../store/datos";
import { usePanel } from "../store/panel";
import { cambioDeVista } from "./animacionWidget";
import { FormularioResponsabilidad } from "./responsabilidades/FormularioResponsabilidad";

/** Cuánto se queda tachada una recién hecha antes de irse (y se puede deshacer). */
const PERMANENCIA_MS = 1800;
const MAXIMO_VISIBLES = 7;

function vencimiento(r: Responsabilidad, ahora: Date): { texto: string; clase: string } {
  if (!r.fechaLimite) return { texto: "", clase: "" };
  switch (grupoDe(r, ahora)) {
    case "vencidas":
      return { texto: "Venció", clase: "text-aviso" };
    case "hoy":
      return { texto: "Hoy", clase: "text-aviso" };
    default:
      return { texto: etiquetaDia(desdeFecha(r.fechaLimite), ahora), clase: "text-tinta-3" };
  }
}

export function Responsabilidades({ ahora }: { ahora: Date }) {
  const lista = useDatos((s) => s.responsabilidades);
  const fuente = useDatos((s) => s.fuentes.responsabilidades);
  const recargar = useDatos((s) => s.recargar);
  const marcar = useDatos((s) => s.marcarResponsabilidad);
  const editando = usePanel((s) => s.edicion === "responsabilidades");
  const editar = usePanel((s) => s.editar);
  const terminar = usePanel((s) => s.terminarEdicion);
  const reducido = useReducedMotion() ?? false;

  const [objetivo, setObjetivo] = useState<Responsabilidad | null>(null);
  const [recienHechas, setRecienHechas] = useState<ReadonlySet<number>>(new Set());
  const temporizadores = useRef(new Map<number, number>());
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const t = temporizadores.current;
    return () => t.forEach((id) => window.clearTimeout(id));
  }, []);

  const sacarDeRecientes = (id: number) => {
    window.clearTimeout(temporizadores.current.get(id));
    temporizadores.current.delete(id);
    setRecienHechas((s) => {
      const nuevo = new Set(s);
      nuevo.delete(id);
      return nuevo;
    });
  };

  async function alternar(r: Responsabilidad) {
    setError(null);
    const hecha = r.estado === "pendiente";
    if (hecha) {
      setRecienHechas((s) => new Set(s).add(r.id));
      temporizadores.current.set(
        r.id,
        window.setTimeout(() => sacarDeRecientes(r.id), PERMANENCIA_MS),
      );
    } else {
      sacarDeRecientes(r.id); // deshacer
    }
    const fallo = await marcar(r.id, hecha);
    if (fallo) {
      sacarDeRecientes(r.id);
      setError(fallo);
    }
  }

  const abrir = (r: Responsabilidad | null) => {
    setObjetivo(r);
    editar("responsabilidades");
  };

  const todas = visibles(lista, recienHechas, ahora);
  const mostradas = todas.slice(0, MAXIMO_VISIBLES);
  const pendientes = lista.filter((r) => r.estado === "pendiente").length;

  return (
    <Seccion
      titulo="Pendientes"
      detalle={fuente.cargada && !editando ? pendientes : undefined}
      className="shrink-0"
      accion={!editando && <AccionTitulo onClick={() => abrir(null)}>Agregar</AccionTitulo>}
    >
      <AnimatePresence mode="wait" initial={false}>
        {editando ? (
          <motion.div key={`form-${objetivo?.id ?? "nueva"}`} {...cambioDeVista}>
            <FormularioResponsabilidad responsabilidad={objetivo} ahora={ahora} alTerminar={terminar} />
          </motion.div>
        ) : (
          <motion.div key="lista" {...cambioDeVista}>
            {fuente.error && !fuente.cargada && (
              <ErrorWidget mensaje="No pude leer tus pendientes." alReintentar={() => void recargar("responsabilidades")} />
            )}
            {fuente.cargada && todas.length === 0 && <p className="text-[15px] text-tinta-3">No tenés nada pendiente.</p>}
            <ul className="flex flex-col">
              <AnimatePresence initial={false}>
                {mostradas.map((r) => (
                  <Fila
                    key={r.id}
                    r={r}
                    ahora={ahora}
                    reducido={reducido}
                    alMarcar={() => void alternar(r)}
                    alEditar={() => abrir(r)}
                  />
                ))}
              </AnimatePresence>
            </ul>
            {todas.length > MAXIMO_VISIBLES && (
              <p className="mt-1 text-[13px] text-tinta-3">y {todas.length - MAXIMO_VISIBLES} más</p>
            )}
            {error && <p className="mt-2 text-[14px] text-aviso">{error}</p>}
          </motion.div>
        )}
      </AnimatePresence>
    </Seccion>
  );
}

interface FilaProps {
  r: Responsabilidad;
  ahora: Date;
  reducido: boolean;
  alMarcar: () => void;
  alEditar: () => void;
}

/**
 * Una responsabilidad. Al marcarla: el círculo se llena, una línea la tacha de
 * izquierda a derecha y el texto se apaga; al rato la fila se cierra.
 */
function Fila({ r, ahora, reducido, alMarcar, alEditar }: FilaProps) {
  const hecha = r.estado === "hecha";
  const v = vencimiento(r, ahora);
  const tachado = { duration: reducido ? 0 : 0.26, ease: EASE_SALIDA };

  return (
    <motion.li
      layout={!reducido}
      initial={{ opacity: 0, height: 0 }}
      animate={{ opacity: 1, height: "auto", transition: { duration: 0.2, ease: EASE_SALIDA } }}
      exit={{ opacity: 0, height: 0, transition: { duration: reducido ? 0.1 : 0.22, ease: EASE_SALIDA } }}
      className="overflow-hidden"
    >
      <div className="fila -mx-2 flex items-center gap-3 px-2 py-1 text-[16px]">
        <button
          type="button"
          role="checkbox"
          aria-checked={hecha}
          aria-label={hecha ? `Desmarcar ${r.titulo}` : `Marcar ${r.titulo} como hecha`}
          onClick={alMarcar}
          // Área de toque más grande que el círculo visible
          className="relative -m-1.5 grid h-6 w-6 shrink-0 place-items-center rounded-full transition-transform duration-150 active:scale-90"
        >
          <span
            className={`grid h-3.5 w-3.5 place-items-center rounded-full border-[1.5px] transition-colors duration-200 ${
              hecha ? "border-tinta-2 bg-tinta-2" : "border-tinta-3"
            }`}
          >
            <motion.svg viewBox="0 0 10 10" className="h-2 w-2" aria-hidden>
              <motion.path
                d="M2 5.2 4.2 7.3 8 2.8"
                fill="none"
                stroke="var(--color-cielo-alto)"
                strokeWidth={1.6}
                strokeLinecap="round"
                strokeLinejoin="round"
                initial={false}
                animate={{ pathLength: hecha ? 1 : 0, opacity: hecha ? 1 : 0 }}
                transition={{ ...tachado, delay: hecha && !reducido ? 0.05 : 0 }}
              />
            </motion.svg>
          </span>
        </button>

        <button
          type="button"
          onClick={alEditar}
          className="min-w-0 flex-1 truncate rounded-md text-left"
          aria-label={`Editar ${r.titulo}`}
        >
          {/* La línea mide lo mismo que el texto, no que la fila */}
          <span className={`relative transition-colors duration-200 ${hecha ? "text-tinta-3" : ""}`}>
            {r.titulo}
            <motion.span
              aria-hidden
              className="absolute inset-x-0 top-[55%] h-px origin-left bg-tinta-3"
              initial={false}
              animate={{ scaleX: hecha ? 1 : 0 }}
              transition={tachado}
            />
          </span>
        </button>

        {r.origen === "auto" && !hecha && <span className="shrink-0 text-[12px] text-tinta-3">auto</span>}
        <span className={`shrink-0 text-[14px] ${hecha ? "text-tinta-3" : v.clase}`}>{hecha ? "Hecha" : v.texto}</span>
      </div>
    </motion.li>
  );
}
