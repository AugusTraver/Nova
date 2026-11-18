import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { pistaDeVoz } from "../lib/voz";
import { useAsistente } from "../store/asistente";
import { useVoz } from "../store/voz";

/**
 * Debajo de la esfera (SPEC 3.2): lo que pediste, qué está haciendo Nova y su
 * respuesta. Si quedó un borrado esperando, los botones para responderlo van
 * acá mismo, sin modales.
 *
 * La zona tiene alto fijo para que la barra de texto no salte cuando aparece
 * o se va una respuesta.
 */
export function Respuesta() {
  const pedido = useAsistente((s) => s.pedido);
  const paso = useAsistente((s) => s.paso);
  const respuesta = useAsistente((s) => s.respuesta);
  const error = useAsistente((s) => s.error);
  const confirmacion = useAsistente((s) => s.confirmacion);
  const confirmar = useAsistente((s) => s.confirmar);
  const reducido = useReducedMotion();

  // Si espera un "sí", se muestra la pregunta exacta que armó Rust (con el
  // título y la fecha reales), no la versión del modelo.
  const texto = confirmacion ?? respuesta;
  const voz = useVoz((s) => s.estado);
  const parcial = useVoz((s) => s.parcial);
  const errorDeVoz = useVoz((s) => (s.estado === "error" ? s.mensaje : null));
  // Mientras hablás, lo que vas diciendo ocupa el lugar del pedido.
  const oyendo = (voz === "oyendo" || voz === "transcribiendo") && parcial !== null;
  const lineaDeArriba = oyendo
    ? `«${parcial}»`
    : pedido && confirmacion === null
      ? `«${pedido}»`
      : paso === null && texto === null && error === null
        ? (errorDeVoz ?? pistaDeVoz(voz) ?? "")
        : "";
  // Una sola "vista" a la vez; la clave hace que se crucen con un fundido.
  const clave = paso !== null ? "paso" : error !== null ? "error" : texto !== null ? `r:${texto}` : "nada";
  const aparecer = reducido
    ? { initial: { opacity: 0 }, animate: { opacity: 1 }, exit: { opacity: 0 } }
    : {
        initial: { opacity: 0, y: 6, filter: "blur(4px)" },
        animate: { opacity: 1, y: 0, filter: "blur(0px)" },
        exit: { opacity: 0, y: -4, filter: "blur(4px)" },
      };

  return (
    <div className="flex h-[6.25rem] w-full flex-col items-center text-center">
      {/* Con una pregunta pendiente, la pregunta manda: el pedido se oculta. */}
      <p
        className={`h-5 max-w-full shrink-0 truncate text-[14px] transition-colors duration-200 ${
          oyendo ? "italic text-tinta-2" : errorDeVoz && !pedido ? "text-aviso" : "text-tinta-3"
        }`}
      >
        {lineaDeArriba}
      </p>

      {/* Lo que lee un lector de pantalla: el paso y después la respuesta. */}
      <div aria-live="polite" aria-atomic className="relative mt-1 w-full flex-1">
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={clave}
            {...aparecer}
            transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
            className="absolute inset-0 flex flex-col items-center"
          >
            {paso !== null && <p className="font-display text-[17px] italic text-tinta-2">{paso}</p>}
            {paso === null && error !== null && (
              <p className="font-display text-[17px] leading-snug text-aviso [text-wrap:balance]">{error}</p>
            )}
            {paso === null && error === null && texto !== null && (
              <p className="max-h-full overflow-y-auto font-display text-[17px] font-[380] leading-snug text-tinta [text-wrap:balance]">
                {texto}
              </p>
            )}
          </motion.div>
        </AnimatePresence>
      </div>

      <AnimatePresence>
        {paso === null && confirmacion !== null && (
          <motion.div
            key="confirmacion"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            role="group"
            aria-label={confirmacion}
            className="pointer-events-auto flex shrink-0 items-center gap-5 text-[15px]"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              type="button"
              onClick={() => confirmar(true)}
              className="rounded-md border-b border-tinta-3 pb-0.5 text-tinta transition-colors duration-150 hover:border-tinta"
            >
              Sí, borralo
            </button>
            <button
              type="button"
              onClick={() => confirmar(false)}
              className="rounded-md pb-0.5 text-tinta-2 transition-colors duration-150 hover:text-tinta"
            >
              No
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
