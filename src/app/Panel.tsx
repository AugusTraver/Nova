import { motion, useReducedMotion } from "framer-motion";
import { Esfera } from "../esfera/Esfera";
import { useAhora } from "../lib/useAhora";
import { usePanel } from "../store/panel";
import { Calendario } from "../widgets/Calendario";
import { Facu } from "../widgets/Facu";
import { Futbol } from "../widgets/Futbol";
import { Noticias } from "../widgets/Noticias";
import { Responsabilidades } from "../widgets/Responsabilidades";
import { ResumenDelDia } from "../widgets/ResumenDelDia";
import { varianteEsfera, varianteSoloOpacidad, varianteVelo, varianteZona } from "./animaciones";
import { BarraEntrada } from "./BarraEntrada";
import { BarraEstado } from "./BarraEstado";

interface PanelProps {
  abierto: boolean;
  onCerrar: () => void;
  /** Se llama cuando termina la animación de salida (para ocultar la ventana). */
  onSalidaCompleta: () => void;
}

const ESTADOS_ESFERA = {
  reposo: "EN REPOSO",
  escuchando: "ESCUCHANDO",
  pensando: "PENSANDO",
  hablando: "HABLANDO",
  error: "ERROR",
} as const;

/**
 * Layout de la sección 3.2 de la especificación, en estilo "instrumento":
 * barra de estado arriba, tres columnas (widgets | esfera | widgets) y la
 * línea de comandos abajo. Clic en el fondo vacío cierra; los bloques frenan el clic.
 *
 * El panel queda siempre montado (no se desmonta al cerrar) para que abrir sea
 * instantáneo: no hay que recrear el contexto WebGL de la esfera cada vez.
 */
export function Panel({ abierto, onCerrar, onSalidaCompleta }: PanelProps) {
  const ahora = useAhora();
  const estadoEsfera = usePanel((s) => s.estadoEsfera);
  const reducido = useReducedMotion() ?? false;
  const zona = (retraso: number) => (reducido ? varianteSoloOpacidad : varianteZona(retraso));

  return (
    <motion.div
      className={`fixed inset-0 bg-fondo ${abierto ? "" : "pointer-events-none"}`}
      variants={varianteVelo}
      initial="oculto"
      animate={abierto ? "visible" : "saliendo"}
      onAnimationComplete={(variante) => {
        if (variante === "saliendo") onSalidaCompleta();
      }}
      onClick={onCerrar}
    >
      <div className="mx-auto grid h-full max-w-[1760px] grid-cols-[minmax(300px,1fr)_minmax(380px,1.25fr)_minmax(300px,1fr)] grid-rows-[auto_1fr_auto] gap-x-16 gap-y-8 p-margen">
        <motion.div className="col-span-3" variants={zona(0.06)}>
          <BarraEstado ahora={ahora} />
        </motion.div>

        <motion.div className="flex min-h-0 flex-col gap-9" variants={zona(0.04)}>
          <ResumenDelDia ahora={ahora} />
          <Calendario ahora={ahora} />
          <Facu ahora={ahora} />
        </motion.div>

        <div className="flex min-h-0 flex-col items-center justify-center">
          <motion.div
            variants={reducido ? varianteSoloOpacidad : varianteEsfera}
            className="aspect-square w-full max-w-[560px]"
            onClick={(e) => e.stopPropagation()}
          >
            <Esfera activa={abierto} />
          </motion.div>
          <p className="mt-1 font-mono text-[11px] tracking-[0.16em] text-texto-3">
            <span className="mr-2 inline-block h-1.5 w-1.5 translate-y-[-1px] rounded-full bg-acento" />
            {ESTADOS_ESFERA[estadoEsfera]}
          </p>
        </div>

        <motion.div className="flex min-h-0 flex-col gap-9" variants={zona(0.07)}>
          <Futbol ahora={ahora} />
          <Responsabilidades ahora={ahora} />
          <Noticias />
        </motion.div>

        <motion.div className="col-span-3 flex justify-center" variants={zona(0.1)}>
          <BarraEntrada abierto={abierto} />
        </motion.div>
      </div>
    </motion.div>
  );
}
