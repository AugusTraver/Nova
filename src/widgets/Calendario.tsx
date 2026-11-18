import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { AccionTitulo, ErrorWidget } from "../components/Formulario";
import { Seccion } from "../components/Seccion";
import { agruparPorDia } from "../lib/calendario";
import { desdeFecha, etiquetaDia, horaDe } from "../lib/fechas";
import type { Ocurrencia } from "../lib/tipos";
import { useDatos } from "../store/datos";
import { usePanel } from "../store/panel";
import { tipoDestacado } from "../theme/eventos";
import { IconoEvento } from "../theme/IconoEvento";
import { cambioDeVista } from "./animacionWidget";
import { FormularioEvento } from "./calendario/FormularioEvento";

const DIAS_VISIBLES = 4;

/** Los próximos días, agrupados: la hora a la izquierda, como una agenda. Clic en un evento lo edita. */
export function Calendario({ ahora }: { ahora: Date }) {
  const ocurrencias = useDatos((s) => s.ocurrencias);
  const materias = useDatos((s) => s.materias);
  const fuente = useDatos((s) => s.fuentes.agenda);
  const recargar = useDatos((s) => s.recargar);
  const editando = usePanel((s) => s.edicion === "calendario");
  const editar = usePanel((s) => s.editar);
  const terminar = usePanel((s) => s.terminarEdicion);
  // Qué se edita: una repetición, o `null` si es un evento nuevo.
  const [objetivo, setObjetivo] = useState<Ocurrencia | null>(null);

  const abrir = (o: Ocurrencia | null) => {
    setObjetivo(o);
    editar("calendario");
  };

  const grupos = agruparPorDia(ocurrencias, ahora, DIAS_VISIBLES);
  const colorMateria = (id: number | null) => {
    const m = materias.find((x) => x.id === id);
    return m ? `var(--color-materia-${m.color})` : undefined;
  };

  return (
    <Seccion
      titulo="Agenda"
      className="shrink-0"
      accion={!editando && <AccionTitulo onClick={() => abrir(null)}>Agregar</AccionTitulo>}
    >
      <AnimatePresence mode="wait" initial={false}>
        {editando ? (
          <motion.div key={`form-${objetivo?.evento.id ?? "nuevo"}`} {...cambioDeVista}>
            <FormularioEvento ocurrencia={objetivo} ahora={ahora} alTerminar={terminar} />
          </motion.div>
        ) : (
          <motion.div key="lista" {...cambioDeVista} className="flex flex-col gap-4">
            {fuente.error && !fuente.cargada && (
              <ErrorWidget mensaje="No pude leer tu agenda." alReintentar={() => void recargar("agenda")} />
            )}
            {fuente.cargada && grupos.length === 0 && (
              <p className="text-[15px] text-tinta-3">Nada en los próximos días.</p>
            )}
            {grupos.map(({ fecha, ocurrencias: lista }) => (
              <div key={fecha} className="grid grid-cols-[5.5rem_1fr] gap-x-4">
                <p className="pt-1 text-[14px] text-tinta-3">{etiquetaDia(desdeFecha(fecha), ahora)}</p>
                <ul className="flex min-w-0 flex-col">
                  {lista.map((o) => (
                    <li key={`${o.evento.id}-${o.fecha}`}>
                      <button
                        type="button"
                        onClick={() => abrir(o)}
                        className="fila -mx-2 flex w-[calc(100%+1rem)] items-baseline gap-3 px-2 py-1 text-left"
                        aria-label={`Editar ${o.evento.titulo}`}
                      >
                        <span className="w-12 shrink-0 font-display text-[18px] text-tinta-2">
                          {o.evento.todoElDia ? "·" : horaDe(o.inicio)}
                        </span>
                        {/* El tipo por la forma del ícono; la materia, por su color */}
                        <IconoEvento tipo={o.evento.tipo} color={colorMateria(o.evento.materiaId)} className="self-center" />
                        <span className={`min-w-0 flex-1 truncate text-[16px] ${tipoDestacado[o.evento.tipo] ? "text-aviso" : ""}`}>
                          {o.evento.titulo}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
            {fuente.error && fuente.cargada && <p className="text-[13px] text-tinta-3">Puede que no esté al día.</p>}
          </motion.div>
        )}
      </AnimatePresence>
    </Seccion>
  );
}
