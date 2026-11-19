import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import type { ReactNode } from "react";
import { Esfera } from "../esfera/Esfera";
import { useAhora } from "../lib/useAhora";
import { esferaSegun } from "../lib/voz";
import { useDatos } from "../store/datos";
import { useVoz } from "../store/voz";
import { type Edicion, usePanel } from "../store/panel";
import { useCielo } from "../theme/useCielo";
import { Calendario } from "../widgets/Calendario";
import { Facu } from "../widgets/Facu";
import { Futbol } from "../widgets/Futbol";
import { Noticias } from "../widgets/Noticias";
import { Responsabilidades } from "../widgets/Responsabilidades";
import { ResumenDelDia } from "../widgets/ResumenDelDia";
import { Ajustes } from "./Ajustes";
import { varianteEsfera, varianteHalo, varianteSoloOpacidad, varianteVelo, varianteZona } from "./animaciones";
import { BarraEntrada } from "./BarraEntrada";
import { Respuesta } from "./Respuesta";
import { cambioDeVista } from "../widgets/animacionWidget";
import { Orbita, usePunteroSuave } from "./Orbita";

interface PanelProps {
  abierto: boolean;
  onCerrar: () => void;
  /** Se llama cuando termina la animación de salida (para ocultar la ventana). */
  onSalidaCompleta: () => void;
}


/*
 * Geometría de la órbita. Todo se mide desde el centro de la esfera para que
 * los widgets la rodeen en arco: los que están a la altura del centro quedan
 * más cerca, los de arriba y abajo más lejos.
 */
const CENTRO_Y = "47%";
const RADIO = "min(20cqh, 230px)";
const ANCHO_WIDGET = "min(360px, 23cqw)";
const ANCHO_EN_VIVO = "min(420px, 27cqw)";
const aDistancia = (extra: string) => `calc(50% + ${RADIO} + ${extra})`;

/**
 * Layout en dirección "Atmósfera": el panel es un cielo translúcido según la
 * hora, la esfera es su fuente de luz y los widgets la orbitan (saludo arriba,
 * agenda y facu a la izquierda, Boca y pendientes a la derecha, noticias al pie).
 * Clic en el cielo vacío cierra.
 *
 * El panel queda siempre montado (no se desmonta al cerrar) para que abrir sea
 * instantáneo: no hay que recrear el contexto WebGL de la esfera cada vez.
 */
/**
 * Mientras se edita algo, el resto del panel se apaga y deja de responder al
 * mouse: un clic ahí cae en el cielo y cancela la edición.
 */
function Atenuable({ apagado, oculto, children }: { apagado: boolean; oculto: boolean; children: ReactNode }) {
  return (
    <div
      className={`transition-opacity duration-200 ${apagado ? "pointer-events-none" : ""}`}
      style={{ opacity: oculto ? 0 : apagado ? 0.16 : 1 }}
      aria-hidden={apagado || undefined}
      inert={apagado || undefined}
    >
      {children}
    </div>
  );
}

export function Panel({ abierto, onCerrar, onSalidaCompleta }: PanelProps) {
  const ahora = useAhora();
  const momento = useCielo(abierto);
  // Lo del asistente (pensando, error) manda; si no, la voz (escucha o habla).
  const estadoEsfera = esferaSegun(
    usePanel((s) => s.estadoEsfera),
    useVoz((s) => s.estado),
  );
  const goles = usePanel((s) => s.goles);
  const despertares = usePanel((s) => s.despertares);
  // En vivo, el widget de fútbol se ensancha un poco (hacia afuera de la órbita).
  const futbolEnVivo = useDatos((s) => s.futbol?.datos?.equipos.some((e) => e.enVivo !== null) ?? false);
  const reducido = useReducedMotion() ?? false;
  const puntero = usePunteroSuave(abierto && !reducido);
  const orbita = { puntero, reducido };
  const edicion = usePanel((s) => s.edicion);
  const editar = usePanel((s) => s.editar);
  const terminarEdicion = usePanel((s) => s.terminarEdicion);
  /** Props para el envoltorio de cada widget según qué se está editando. */
  const estado = (propio?: Edicion) => ({
    apagado: edicion !== null && edicion !== propio,
    oculto: edicion === "ajustes",
  });
  const alFrente = (propio: Edicion) => (edicion === propio ? "z-10" : "");

  return (
    <motion.div
      className={`cielo fixed inset-0 overflow-hidden ${abierto ? "" : "pausado pointer-events-none"}`}
      variants={varianteVelo}
      initial="oculto"
      animate={abierto ? "visible" : "saliendo"}
      onAnimationComplete={(variante) => {
        if (variante === "saliendo") onSalidaCompleta();
      }}
      onClick={edicion ? terminarEdicion : onCerrar}
      // Contenedor de tamaño: las medidas cqw/cqh de la órbita son relativas al panel
      style={{ containerType: "size" }}
    >
      {/* Arriba: el saludo */}
      <Orbita
        {...orbita}
        lado="arriba"
        posicion={{ top: "6cqh", left: "50%", width: "min(760px, 60cqw)", marginLeft: "max(-380px, -30cqw)" }}
        profundidad={6}
        retraso={0.04}
        vaiven={11}
      >
        <Atenuable {...estado()}>
          <ResumenDelDia ahora={ahora} />
        </Atenuable>
      </Orbita>

      {/* Izquierda */}
      <Orbita
        {...orbita}
        className={alFrente("calendario")}
        lado="izquierda"
        posicion={{ top: "25%", right: aDistancia("3cqw"), width: ANCHO_WIDGET }}
        profundidad={14}
        retraso={0.07}
        vaiven={9}
      >
        <Atenuable {...estado("calendario")}>
          <Calendario ahora={ahora} />
        </Atenuable>
      </Orbita>
      <Orbita
        {...orbita}
        className={alFrente("facu")}
        lado="izquierda"
        posicion={{ top: "58%", right: aDistancia("8cqw"), width: ANCHO_WIDGET }}
        profundidad={10}
        retraso={0.12}
        vaiven={12}
      >
        <Atenuable {...estado("facu")}>
          <Facu ahora={ahora} />
        </Atenuable>
      </Orbita>

      {/* Derecha */}
      <Orbita
        {...orbita}
        lado="derecha"
        posicion={{
          top: "19%",
          left: aDistancia("6cqw"),
          width: futbolEnVivo ? ANCHO_EN_VIVO : ANCHO_WIDGET,
          transition: "width 450ms cubic-bezier(0.22, 1, 0.36, 1)",
        }}
        profundidad={11}
        retraso={0.09}
        vaiven={10}
      >
        <Atenuable {...estado()}>
          <Futbol ahora={ahora} />
        </Atenuable>
      </Orbita>
      <Orbita
        {...orbita}
        className={alFrente("responsabilidades")}
        lado="derecha"
        posicion={{ top: "53%", left: aDistancia("3cqw"), width: ANCHO_WIDGET }}
        profundidad={15}
        retraso={0.11}
        vaiven={8}
      >
        <Atenuable {...estado("responsabilidades")}>
          <Responsabilidades ahora={ahora} />
        </Atenuable>
      </Orbita>

      {/* Abajo: noticias */}
      <Orbita
        {...orbita}
        lado="abajo"
        posicion={{ bottom: "5cqh", left: "50%", width: "min(820px, 56cqw)", marginLeft: "max(-410px, -28cqw)" }}
        profundidad={5}
        retraso={0.15}
        vaiven={13}
      >
        <Atenuable {...estado()}>
          <Noticias ahora={ahora} />
        </Atenuable>
      </Orbita>

      {/* Ajustes: ocupan el lugar de la columna izquierda mientras el resto se apaga */}
      <AnimatePresence>
        {edicion === "ajustes" && (
          <motion.div
            key="ajustes"
            {...cambioDeVista}
            className="absolute z-10 overflow-y-auto overflow-x-hidden px-2"
            style={{ top: "22%", right: aDistancia("3cqw"), width: "min(440px, 30cqw)", maxHeight: "70cqh" }}
            onClick={(e) => e.stopPropagation()}
          >
            <Ajustes alTerminar={terminarEdicion} />
          </motion.div>
        )}
      </AnimatePresence>

      {edicion === null && (
        <motion.button
          type="button"
          variants={reducido ? varianteSoloOpacidad : varianteZona(0.2)}
          onClick={(e) => {
            e.stopPropagation();
            editar("ajustes");
          }}
          className="absolute rounded-md text-[14px] text-tinta-3 transition-colors duration-150 hover:text-tinta-2"
          style={{ left: "var(--spacing-margen)", bottom: "5cqh" }}
        >
          Ajustes
        </motion.button>
      )}

      {/* Centro: la esfera, su estado y la barra para escribirle */}
      <div
        className="pointer-events-none absolute left-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center"
        style={{ top: CENTRO_Y, width: `calc(${RADIO} * 2)`, height: `calc(${RADIO} * 2)` }}
      >
        <motion.div
          aria-hidden
          className="halo absolute -inset-[70%]"
          variants={reducido ? varianteSoloOpacidad : varianteHalo}
        />
        <motion.div
          variants={reducido ? varianteSoloOpacidad : varianteEsfera}
          className="pointer-events-auto absolute inset-0"
          onClick={(e) => e.stopPropagation()}
        >
          <Esfera activa={abierto} momento={momento} estado={estadoEsfera} goles={goles} despertares={despertares} />
        </motion.div>
      </div>

      <div
        className="pointer-events-none absolute left-1/2 flex -translate-x-1/2 flex-col items-center"
        style={{ top: `calc(${CENTRO_Y} + ${RADIO} + 8px)`, width: `min(30rem, calc(2 * ${RADIO} + 4cqw))` }}
      >
        <Respuesta />
        <motion.div className="pointer-events-auto mt-1 flex w-full justify-center" variants={reducido ? varianteSoloOpacidad : varianteZona(0.1)}>
          <BarraEntrada abierto={abierto} />
        </motion.div>
      </div>
    </motion.div>
  );
}
