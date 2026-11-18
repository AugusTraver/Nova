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

/**
 * Layout de la sección 3.2 de la especificación, en dirección "Atmósfera":
 * el panel es un cielo según la hora, la esfera es su fuente de luz en el
 * centro y los widgets flotan a los costados sin tarjetas. Abajo, al centro,
 * la barra para escribirle. Clic en el cielo vacío cierra.
 *
 * El panel queda siempre montado (no se desmonta al cerrar) para que abrir sea
 * instantáneo: no hay que recrear el contexto WebGL de la esfera cada vez.
 */
export function Panel({ abierto, onCerrar, onSalidaCompleta }: PanelProps) {
  const ahora = useAhora();
  const momento = useCielo(abierto);
  const estadoEsfera = usePanel((s) => s.estadoEsfera);
  const reducido = useReducedMotion() ?? false;
  const zona = (retraso: number) => (reducido ? varianteSoloOpacidad : varianteZona(retraso));

  return (
    <motion.div
      className={`cielo fixed inset-0 ${abierto ? "" : "pointer-events-none"}`}
      variants={varianteVelo}
      initial="oculto"
      animate={abierto ? "visible" : "saliendo"}
      onAnimationComplete={(variante) => {
        if (variante === "saliendo") onSalidaCompleta();
      }}
      onClick={onCerrar}
    >
      <div className="relative mx-auto grid h-full max-w-[1800px] grid-cols-[minmax(320px,420px)_1fr_minmax(320px,420px)] gap-x-12 p-margen">
        <motion.div className="flex min-h-0 flex-col gap-10" variants={zona(0.05)}>
          <ResumenDelDia ahora={ahora} />
          <Calendario ahora={ahora} />
          <Facu ahora={ahora} />
        </motion.div>

        <div className="flex min-h-0 flex-col items-center justify-center">
          <div className="relative aspect-square w-full max-w-[520px]">
            {/* La luz de la esfera tiñe el cielo alrededor */}
            <motion.div
              aria-hidden
              className="halo pointer-events-none absolute -inset-[70%]"
              variants={reducido ? varianteSoloOpacidad : varianteHalo}
            />
            <motion.div
              variants={reducido ? varianteSoloOpacidad : varianteEsfera}
              className="absolute inset-0"
              onClick={(e) => e.stopPropagation()}
            >
              <Esfera activa={abierto} momento={momento} />
            </motion.div>
          </div>
          <p aria-live="polite" className="mt-2 h-6 font-display text-[17px] italic text-tinta-2">
            {ESTADOS_ESFERA[estadoEsfera]}
          </p>
          <motion.div className="mt-10 flex w-full justify-center" variants={zona(0.09)}>
            <BarraEntrada abierto={abierto} />
          </motion.div>
        </div>

        <motion.div className="flex min-h-0 flex-col gap-10" variants={zona(0.07)}>
          <Futbol ahora={ahora} />
          <Responsabilidades ahora={ahora} />
          <Noticias />
        </motion.div>
      </div>
    </motion.div>
  );
}
