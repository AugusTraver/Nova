import { motion } from "framer-motion";
import { Esfera } from "../esfera/Esfera";
import { Calendario } from "../widgets/Calendario";
import { Facu } from "../widgets/Facu";
import { Futbol } from "../widgets/Futbol";
import { Noticias } from "../widgets/Noticias";
import { Responsabilidades } from "../widgets/Responsabilidades";
import { ResumenDelDia } from "../widgets/ResumenDelDia";
import { varianteEsfera, varianteGrilla, varianteVelo } from "./animaciones";
import { BarraEntrada } from "./BarraEntrada";

interface PanelProps {
  abierto: boolean;
  onCerrar: () => void;
  /** Se llama cuando termina la animación de salida (para ocultar la ventana). */
  onSalidaCompleta: () => void;
}

/**
 * Layout de la sección 3.2 de la especificación:
 * tres columnas (widgets | esfera | widgets) y la barra de entrada abajo.
 * Clic en el fondo vacío cierra; los widgets y la barra frenan la propagación.
 *
 * El panel queda siempre montado (no se desmonta al cerrar) para que abrir sea
 * instantáneo: no hay que recrear el contexto WebGL de la esfera cada vez.
 */
export function Panel({ abierto, onCerrar, onSalidaCompleta }: PanelProps) {
  return (
    <motion.div
      className={`fixed inset-0 bg-velo ${abierto ? "" : "pointer-events-none"}`}
      variants={varianteVelo}
      initial="oculto"
      animate={abierto ? "visible" : "saliendo"}
      onAnimationComplete={(variante) => {
        if (variante === "saliendo") onSalidaCompleta();
      }}
      onClick={onCerrar}
    >
      <motion.div
        className="mx-auto grid h-full max-w-[1800px] grid-cols-[minmax(280px,1fr)_minmax(360px,1.3fr)_minmax(280px,1fr)] grid-rows-[1fr_auto] gap-6 p-10"
        variants={varianteGrilla}
      >
        <div className="flex min-h-0 flex-col gap-6">
          <ResumenDelDia />
          <Calendario />
          <Facu />
        </div>

        <div className="flex min-h-0 flex-col items-center justify-center">
          <motion.div
            variants={varianteEsfera}
            className="aspect-square w-full max-w-[520px]"
            onClick={(e) => e.stopPropagation()}
          >
            <Esfera activa={abierto} />
          </motion.div>
          <p className="mt-2 text-center text-sm text-texto-tenue">Estoy acá. Escribime algo.</p>
        </div>

        <div className="flex min-h-0 flex-col gap-6">
          <Futbol />
          <Responsabilidades />
          <Noticias />
        </div>

        <div className="col-span-3 flex justify-center">
          <BarraEntrada />
        </div>
      </motion.div>
    </motion.div>
  );
}
