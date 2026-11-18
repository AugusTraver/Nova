import { type MotionValue, motion, useSpring, useTransform, type Variants } from "framer-motion";
import { type CSSProperties, type ReactNode, useEffect } from "react";
import { EASE_SALIDA } from "./animaciones";

/**
 * Posición del mouse (-1 a 1) con resorte, compartida por todos los widgets.
 * Se actualiza sin renders de React (son MotionValues).
 */
export function usePunteroSuave(activo: boolean): { x: MotionValue<number>; y: MotionValue<number> } {
  const x = useSpring(0, { stiffness: 40, damping: 18, mass: 1 });
  const y = useSpring(0, { stiffness: 40, damping: 18, mass: 1 });

  useEffect(() => {
    if (!activo) return;
    const alMover = (e: PointerEvent) => {
      x.set((e.clientX / window.innerWidth) * 2 - 1);
      y.set((e.clientY / window.innerHeight) * 2 - 1);
    };
    window.addEventListener("pointermove", alMover);
    return () => window.removeEventListener("pointermove", alMover);
  }, [activo, x, y]);

  return { x, y };
}

type Lado = "izquierda" | "derecha" | "arriba" | "abajo";

/** Desde dónde "sale" cada widget: desde la esfera hacia afuera. */
const DESDE: Record<Lado, { x: number; y: number }> = {
  izquierda: { x: 22, y: 0 },
  derecha: { x: -22, y: 0 },
  arriba: { x: 0, y: 14 },
  abajo: { x: 0, y: -14 },
};

function varianteOrbita(lado: Lado, retraso: number, reducido: boolean): Variants {
  const desde = reducido ? { x: 0, y: 0 } : DESDE[lado];
  return {
    oculto: { opacity: 0, ...desde, scale: reducido ? 1 : 0.98 },
    visible: {
      opacity: 1,
      x: 0,
      y: 0,
      scale: 1,
      transition: { duration: 0.32, ease: EASE_SALIDA, delay: retraso },
    },
    saliendo: { opacity: 0, transition: { duration: 0.12, ease: "easeOut" } },
  };
}

interface OrbitaProps {
  /** De qué lado de la esfera está: define hacia dónde entra. */
  lado: Lado;
  /** Dónde se ubica en la pantalla (left/right/top/bottom). */
  posicion: CSSProperties;
  /** Cuánto se mueve con el mouse, en píxeles. Más = se siente más cerca. */
  profundidad: number;
  /** Segundos de retraso en la entrada. */
  retraso: number;
  /** Duración del vaivén lento, en segundos (distinta en cada widget). */
  vaiven: number;
  puntero: { x: MotionValue<number>; y: MotionValue<number> };
  reducido: boolean;
  className?: string;
  children: ReactNode;
}

/**
 * Un widget que orbita la esfera: entra saliendo desde ella, se desplaza
 * apenas al contrario del mouse (paralaje, da profundidad) y flota con un
 * vaivén muy lento. Con "reducir movimiento" solo aparece.
 */
export function Orbita({
  lado,
  posicion,
  profundidad,
  retraso,
  vaiven,
  puntero,
  reducido,
  className = "",
  children,
}: OrbitaProps) {
  const p = reducido ? 0 : profundidad;
  const x = useTransform(puntero.x, (v) => v * -p);
  const y = useTransform(puntero.y, (v) => v * -p * 0.6);

  return (
    <motion.div
      className={`absolute ${className}`}
      style={posicion}
      variants={varianteOrbita(lado, retraso, reducido)}
    >
      <motion.div style={{ x, y }}>
        <div className="vaiven" style={{ animationDuration: `${vaiven}s`, animationDelay: `${-retraso * 20}s` }}>
          {children}
        </div>
      </motion.div>
    </motion.div>
  );
}
