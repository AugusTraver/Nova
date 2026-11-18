import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { AccionTitulo, ErrorWidget } from "../components/Formulario";
import { Seccion } from "../components/Seccion";
import { progresoPorMateria, proximasEvaluaciones } from "../lib/facu";
import { cuentaRegresiva, diasEntre, momentoADate } from "../lib/fechas";
import type { Materia } from "../lib/tipos";
import { useDatos } from "../store/datos";
import { usePanel } from "../store/panel";
import { nombreTipoEvento } from "../theme/eventos";
import { cambioDeVista } from "./animacionWidget";
import { FormularioMateria } from "./facu/FormularioMateria";

/** Lo próximo que se rinde o se entrega, y cuánto se avanzó en cada materia. */
export function Facu({ ahora }: { ahora: Date }) {
  const materias = useDatos((s) => s.materias);
  const ocurrencias = useDatos((s) => s.ocurrencias);
  const responsabilidades = useDatos((s) => s.responsabilidades);
  const fuente = useDatos((s) => s.fuentes.materias);
  const recargar = useDatos((s) => s.recargar);
  const editando = usePanel((s) => s.edicion === "facu");
  const editar = usePanel((s) => s.editar);
  const terminar = usePanel((s) => s.terminarEdicion);
  const [objetivo, setObjetivo] = useState<Materia | null>(null);

  const abrir = (m: Materia | null) => {
    setObjetivo(m);
    editar("facu");
  };

  const proximas = proximasEvaluaciones(ocurrencias, ahora, 3);
  const progreso = progresoPorMateria(materias, responsabilidades);
  const nombreMateria = (id: number | null) => materias.find((m) => m.id === id)?.nombre;

  return (
    <Seccion
      titulo="Facu"
      className="shrink-0"
      accion={!editando && <AccionTitulo onClick={() => abrir(null)}>Agregar materia</AccionTitulo>}
    >
      <AnimatePresence mode="wait" initial={false}>
        {editando ? (
          <motion.div key={`form-${objetivo?.id ?? "nueva"}`} {...cambioDeVista}>
            <FormularioMateria materia={objetivo} alTerminar={terminar} />
          </motion.div>
        ) : (
          <motion.div key="lista" {...cambioDeVista}>
            {proximas.length > 0 && (
              <ul className="mb-5 flex flex-col">
                {proximas.map((o) => {
                  const fecha = momentoADate(o.inicio);
                  const cerca = diasEntre(ahora, fecha) <= 1;
                  const materia = nombreMateria(o.evento.materiaId);
                  return (
                    <li key={`${o.evento.id}-${o.fecha}`} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1 text-[16px]">
                      <span className="min-w-0 flex-1 truncate">
                        {materia ? `${nombreTipoEvento[o.evento.tipo]} de ${materia}` : o.evento.titulo}
                      </span>
                      <span className={`shrink-0 text-[14px] ${cerca ? "text-aviso" : "text-tinta-3"}`}>
                        {cuentaRegresiva(fecha, ahora)}
                      </span>
                    </li>
                  );
                })}
              </ul>
            )}

            {fuente.error && !fuente.cargada && (
              <ErrorWidget mensaje="No pude leer tus materias." alReintentar={() => void recargar("materias")} />
            )}
            {fuente.cargada && materias.length === 0 && (
              <p className="text-[15px] text-tinta-3">Agregá tus materias para ver el progreso de cada una.</p>
            )}

            <ul className="flex flex-col">
              {materias.map((m) => {
                const { hechas, total } = progreso.get(m.id) ?? { hechas: 0, total: 0 };
                const porcentaje = total === 0 ? 0 : (hechas / total) * 100;
                return (
                  <li key={m.id}>
                    <button
                      type="button"
                      onClick={() => abrir(m)}
                      aria-label={`Editar ${m.nombre}: ${hechas} de ${total} hechas`}
                      className="fila -mx-2 grid w-[calc(100%+1rem)] grid-cols-[1fr_7rem_2.5rem] items-center gap-3 px-2 py-1 text-left text-[14px] text-tinta-2"
                    >
                      <span className="truncate">{m.nombre}</span>
                      <span className="h-[3px] overflow-hidden rounded-full bg-tinta-4" aria-hidden>
                        <motion.span
                          className="block h-full rounded-full"
                          style={{ backgroundColor: `var(--color-materia-${m.color})` }}
                          initial={false}
                          animate={{ width: `${porcentaje}%` }}
                          transition={{ duration: 0.4, ease: [0.23, 1, 0.32, 1] }}
                        />
                      </span>
                      <span className="text-right text-tinta-3">
                        {hechas}/{total}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </motion.div>
        )}
      </AnimatePresence>
    </Seccion>
  );
}
