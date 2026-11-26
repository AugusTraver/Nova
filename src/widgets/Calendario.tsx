import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { AccionTitulo, ErrorWidget } from "../components/Formulario";
import { Seccion } from "../components/Seccion";
import { agruparPorDia } from "../lib/calendario";
import { desdeFecha, etiquetaDia, horaDe } from "../lib/fechas";
import { avisoCalendar, avisoCortoCalendar } from "../lib/google";
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
  const google = useDatos((s) => s.google);
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
  const avisoGoogle = avisoCalendar(google, ahora);
  const avisoCorto = avisoCortoCalendar(google);
  const colorMateria = (id: number | null) => {
    const m = materias.find((x) => x.id === id);
    return m ? `var(--color-materia-${m.color})` : undefined;
  };

  return (
    <Seccion
      titulo="Agenda"
      className="shrink-0"
      accion={
        !editando && (
          <span className="flex items-baseline gap-3">
            {/* Si Google falla, un aviso chico en el renglón del título (el detalle, al pasar el mouse). */}
            {avisoCorto && (
              <span className="text-[12px] text-tinta-3" title={avisoGoogle ?? undefined}>
                {avisoCorto}
                <span className="sr-only">. {avisoGoogle}</span>
              </span>
            )}
            <AccionTitulo onClick={() => abrir(null)}>Agregar</AccionTitulo>
          </span>
        )
      }
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
                      {/* Los de solo lectura (una serie de Google que Nova no sabe repetir) no abren el formulario. */}
                      <button
                        type="button"
                        onClick={() => !o.evento.soloLectura && abrir(o)}
                        aria-disabled={o.evento.soloLectura || undefined}
                        title={o.evento.soloLectura ? "Se edita desde Google Calendar" : undefined}
                        className={`fila -mx-2 flex w-[calc(100%+1rem)] items-baseline gap-3 px-2 py-1 text-left ${o.evento.soloLectura ? "cursor-default" : ""}`}
                        aria-label={
                          o.evento.soloLectura
                            ? `${o.evento.titulo}, se edita desde Google Calendar`
                            : `Editar ${o.evento.titulo}${o.evento.deGoogle ? ", de Google Calendar" : ""}`
                        }
                      >
                        <span className="w-12 shrink-0 font-display text-[18px] text-tinta-2">
                          {o.evento.todoElDia ? "·" : horaDe(o.inicio)}
                        </span>
                        {/* El tipo por la forma del ícono; la materia, por su color */}
                        <IconoEvento tipo={o.evento.tipo} color={colorMateria(o.evento.materiaId)} className="self-center" />
                        <span className={`min-w-0 truncate text-[16px] ${tipoDestacado[o.evento.tipo] ? "text-aviso" : ""}`}>
                          {o.evento.titulo}
                        </span>
                        {/* Vino de Google Calendar: un anillito tenue, sin etiqueta. Fuera del
                            título, así no se lo come el recorte de los títulos largos. */}
                        {o.evento.deGoogle && (
                          <span aria-hidden className="-ml-1.5 size-[5px] shrink-0 -translate-y-[3px] rounded-full border border-tinta-3" />
                        )}
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
