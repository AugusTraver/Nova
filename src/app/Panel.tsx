import { motion, useReducedMotion } from "framer-motion";
import { Esfera } from "../esfera/Esfera";
import { useAhora } from "../lib/useAhora";
import { usePanel } from "../store/panel";
import { useCielo } from "../theme/useCielo";
import { Calendario } from "../widgets/Calendario";
import { Facu } from "../widgets/Facu";
import { Futbol } from "../widgets/Futbol";
import { Noticias } from "../widgets/Noticias";
import { Responsabilidades } from "../widgets/Responsabilidades";
import { ResumenDelDia } from "../widgets/ResumenDelDia";
import { varianteEsfera, varianteHalo, varianteSoloOpacidad, varianteVelo, varianteZona } from "./animaciones";
import { BarraEntrada } from "./BarraEntrada";
import { Orbita, usePunteroSuave } from "./Orbita";

interface PanelProps {
  abierto: boolean;
  onCerrar: () => void;
  /** Se llama cuando termina la animación de salida (para ocultar la ventana). */
  onSalidaCompleta: () => void;
}

/** Qué dice debajo de la esfera. En reposo no dice nada. */
const ESTADOS_ESFERA = {
  reposo: "",
  escuchando: "Te escucho",
  pensando: "Pensando…",
  hablando: "",
  error: "Algo falló. Probá de nuevo.",
} as const;

/*
 * Geometría de la órbita. Todo se mide desde el centro de la esfera para que
 * los widgets la rodeen en arco: los que están a la altura del centro quedan
 * más cerca, los de arriba y abajo más lejos.
 */
const CENTRO_Y = "47%";
const RADIO = "min(20cqh, 230px)";
const ANCHO_WIDGET = "min(360px, 23cqw)";
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
export function Panel({ abierto, onCerrar, onSalidaCompleta }: PanelProps) {
  const ahora = useAhora();
  const momento = useCielo(abierto);
  const estadoEsfera = usePanel((s) => s.estadoEsfera);
  const reducido = useReducedMotion() ?? false;
  const puntero = usePunteroSuave(abierto && !reducido);
  const orbita = { puntero, reducido };

  return (
    <motion.div
      className={`cielo fixed inset-0 overflow-hidden ${abierto ? "" : "pausado pointer-events-none"}`}
      variants={varianteVelo}
      initial="oculto"
      animate={abierto ? "visible" : "saliendo"}
      onAnimationComplete={(variante) => {
        if (variante === "saliendo") onSalidaCompleta();
      }}
      onClick={onCerrar}
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
        <ResumenDelDia ahora={ahora} />
      </Orbita>

      {/* Izquierda */}
      <Orbita
        {...orbita}
        lado="izquierda"
        posicion={{ top: "25%", right: aDistancia("3cqw"), width: ANCHO_WIDGET }}
        profundidad={14}
        retraso={0.07}
        vaiven={9}
      >
        <Calendario ahora={ahora} />
      </Orbita>
      <Orbita
        {...orbita}
        lado="izquierda"
        posicion={{ top: "58%", right: aDistancia("8cqw"), width: ANCHO_WIDGET }}
        profundidad={10}
        retraso={0.12}
        vaiven={12}
      >
        <Facu ahora={ahora} />
      </Orbita>

      {/* Derecha */}
      <Orbita
        {...orbita}
        lado="derecha"
        posicion={{ top: "19%", left: aDistancia("6cqw"), width: ANCHO_WIDGET }}
        profundidad={11}
        retraso={0.09}
        vaiven={10}
      >
        <Futbol ahora={ahora} />
      </Orbita>
      <Orbita
        {...orbita}
        lado="derecha"
        posicion={{ top: "53%", left: aDistancia("3cqw"), width: ANCHO_WIDGET }}
        profundidad={15}
        retraso={0.11}
        vaiven={8}
      >
        <Responsabilidades ahora={ahora} />
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
        <Noticias />
      </Orbita>

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
          <Esfera activa={abierto} momento={momento} />
        </motion.div>
      </div>

      <div
        className="absolute left-1/2 flex w-[min(30rem,40cqw)] -translate-x-1/2 flex-col items-center"
        style={{ top: `calc(${CENTRO_Y} + ${RADIO} + 8px)` }}
      >
        <p aria-live="polite" className="h-6 font-display text-[17px] italic text-tinta-2">
          {ESTADOS_ESFERA[estadoEsfera]}
        </p>
        <motion.div className="mt-6 flex w-full justify-center" variants={reducido ? varianteSoloOpacidad : varianteZona(0.1)}>
          <BarraEntrada abierto={abierto} />
        </motion.div>
      </div>
    </motion.div>
  );
}
