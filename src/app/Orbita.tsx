import {
  animate,
  type MotionValue,
  motion,
  useDragControls,
  useMotionValue,
  useSpring,
  useTransform,
  type Variants,
} from "framer-motion";
import { type CSSProperties, type PointerEvent as EventoPuntero, type ReactNode, useEffect, useRef, useState } from "react";
import { acomodar, aFraccion, aPixeles, type Caja } from "../lib/disposicion";
import type { Corrimiento, IdWidget } from "../lib/tipos";
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

/** Para arrastrar el widget (v0.7.1). Sin esto, queda fijo. */
export interface Arrastre {
  id: IdWidget;
  /** Dónde lo dejó el usuario; sin nada, en su lugar de siempre. */
  corrimiento: Corrimiento | undefined;
  /** Apagado mientras se edita algo o se confirma un cambio. */
  activo: boolean;
  alSoltar: (id: IdWidget, corrimiento: Corrimiento) => void;
}

/** Desde acá no se empieza a arrastrar: son para hacer clic o escribir. */
const INTERACTIVOS =
  "button, a, input, textarea, select, label, summary, [role=button], [contenteditable], [data-no-arrastrar]";

/** El resorte con el que el widget se acomoda al soltarlo (el "imán"). */
const RESORTE = { type: "spring", stiffness: 260, damping: 30 } as const;

const tamPanel = () => ({ ancho: window.innerWidth, alto: window.innerHeight });

const aCaja = (r: DOMRect): Caja => ({ x: r.left, y: r.top, ancho: r.width, alto: r.height });

/**
 * Lo que un widget no puede tapar al soltarlo: los otros widgets y lo
 * marcado con `data-obstaculo` en el panel (la esfera, la respuesta y la barra).
 */
function obstaculos(propio: HTMLElement): Caja[] {
  return [...document.querySelectorAll<HTMLElement>("[data-widget], [data-obstaculo]")]
    .filter((el) => el !== propio)
    .map((el) => aCaja(el.getBoundingClientRect()))
    .filter((c) => c.ancho > 0 && c.alto > 0);
}

/**
 * Al soltar, el navegador manda un clic. Si el mouse terminó fuera del
 * widget, ese clic cae en el cielo y cerraría el panel; si terminó sobre un
 * botón, lo apretaría. Se lo frena antes de que llegue a nadie (fase de
 * captura en la ventana) y, si no llega, el freno se saca solo.
 */
function tragarElProximoClic() {
  const frenar = (e: MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    sacar();
  };
  const sacar = () => window.removeEventListener("click", frenar, true);
  window.addEventListener("click", frenar, true);
  setTimeout(sacar, 0);
}

/**
 * La capa que se arrastra. Tiene su propio `x`/`y`, aparte de la entrada,
 * el paralaje y el vaivén, así no se pisan. Al soltar se acomoda con un
 * resorte y avisa dónde quedó.
 */
function Arrastrable({
  arrastre,
  reducido,
  alArrastrar,
  children,
}: {
  arrastre: Arrastre;
  reducido: boolean;
  alArrastrar: (arrastrando: boolean) => void;
  children: ReactNode;
}) {
  const { id, corrimiento, activo, alSoltar } = arrastre;
  const x = useMotionValue(0);
  const y = useMotionValue(0);
  const capa = useRef<HTMLDivElement>(null);
  const controles = useDragControls();
  const arrastrando = useRef(false);

  // Lo guardado se pasa a píxeles; si cambia el tamaño de la ventana, se recalcula.
  const cx = corrimiento?.x ?? 0;
  const cy = corrimiento?.y ?? 0;
  useEffect(() => {
    const ubicar = (suave: boolean) => {
      if (arrastrando.current) return;
      const p = aPixeles({ x: cx, y: cy }, tamPanel());
      if (suave && !reducido) {
        animate(x, p.x, RESORTE);
        animate(y, p.y, RESORTE);
      } else {
        x.set(p.x);
        y.set(p.y);
      }
    };
    ubicar(true);
    const alCambiarTamano = () => ubicar(false);
    window.addEventListener("resize", alCambiarTamano);
    return () => window.removeEventListener("resize", alCambiarTamano);
  }, [cx, cy, reducido, x, y]);

  const empezar = (e: EventoPuntero<HTMLDivElement>) => {
    if (!activo || e.button !== 0) return;
    if ((e.target as HTMLElement).closest(INTERACTIVOS)) return;
    controles.start(e);
  };

  const soltar = () => {
    arrastrando.current = false;
    alArrastrar(false);
    document.body.style.cursor = "";
    tragarElProximoClic();
    const el = capa.current;
    if (!el) return;
    const caja = aCaja(el.getBoundingClientRect());
    const final = acomodar(caja, obstaculos(el), { x: 0, y: 0, ...tamPanel() });
    const destino = { x: x.get() + final.x - caja.x, y: y.get() + final.y - caja.y };
    animate(x, destino.x, reducido ? { duration: 0 } : RESORTE);
    animate(y, destino.y, reducido ? { duration: 0 } : RESORTE);
    alSoltar(id, aFraccion(destino, tamPanel()));
  };

  return (
    <motion.div
      ref={capa}
      data-widget={id}
      style={{ x, y }}
      drag={activo}
      dragControls={controles}
      dragListener={false}
      dragMomentum={false}
      whileDrag={reducido ? undefined : { scale: 1.02 }}
      onPointerDown={empezar}
      onDragStart={() => {
        arrastrando.current = true;
        alArrastrar(true);
        document.body.style.cursor = "grabbing";
      }}
      onDragEnd={soltar}
    >
      {children}
    </motion.div>
  );
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
  /** Para poder arrastrarlo; sin esto, queda fijo. */
  arrastre?: Arrastre;
  children: ReactNode;
}

/**
 * Un widget que orbita la esfera: entra saliendo desde ella, se desplaza
 * apenas al contrario del mouse (paralaje, da profundidad) y flota con un
 * vaivén muy lento. Con "reducir movimiento" solo aparece. Con `arrastre`,
 * se agarra de cualquier parte que no sea un botón o un campo.
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
  arrastre,
  children,
}: OrbitaProps) {
  const p = reducido ? 0 : profundidad;
  const x = useTransform(puntero.x, (v) => v * -p);
  const y = useTransform(puntero.y, (v) => v * -p * 0.6);
  // El que se arrastra pasa por encima de los demás.
  const [arrastrando, setArrastrando] = useState(false);

  const contenido = (
    <motion.div style={{ x, y }}>
      <div className="vaiven" style={{ animationDuration: `${vaiven}s`, animationDelay: `${-retraso * 20}s` }}>
        {children}
      </div>
    </motion.div>
  );

  return (
    <motion.div
      className={`absolute ${arrastrando ? "z-20" : ""} ${className}`}
      style={posicion}
      variants={varianteOrbita(lado, retraso, reducido)}
    >
      {arrastre ? (
        <Arrastrable arrastre={arrastre} reducido={reducido} alArrastrar={setArrastrando}>
          {contenido}
        </Arrastrable>
      ) : (
        contenido
      )}
    </motion.div>
  );
}
