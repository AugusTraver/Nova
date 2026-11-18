import { Canvas, useFrame } from "@react-three/fiber";
import { useEffect, useMemo, useRef } from "react";
import { Color, type Mesh } from "three";
import type { EstadoEsfera } from "../store/panel";
import type { Momento } from "../theme/cielo";
import { fragmentShader, vertexShader } from "./shaders";

/** Lee un color de los tokens CSS (ej. "--color-luz") como THREE.Color. */
function colorDeToken(token: string): Color {
  const valor = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  if (valor.startsWith("#")) return new Color(valor);
  // "rgb(122 146 255)": THREE.Color no entiende esa sintaxis, se extraen los números.
  const [r, g, b] = (valor.match(/\d+(\.\d+)?/g) ?? ["255", "255", "255"]).map(Number);
  return new Color(r / 255, g / 255, b / 255);
}

/**
 * Posición del mouse en toda la ventana, normalizada de -1 a 1.
 * Se lee en cada cuadro sin provocar renders de React.
 */
function usePunteroGlobal() {
  const puntero = useRef({ x: 0, y: 0 });
  useEffect(() => {
    const alMover = (e: PointerEvent) => {
      puntero.current.x = (e.clientX / window.innerWidth) * 2 - 1;
      puntero.current.y = (e.clientY / window.innerHeight) * 2 - 1;
    };
    window.addEventListener("pointermove", alMover);
    return () => window.removeEventListener("pointermove", alMover);
  }, []);
  return puntero;
}

/** Acerca `actual` a `objetivo` de forma suave e independiente de los FPS. */
function amortiguar(actual: number, objetivo: number, velocidad: number, delta: number): number {
  return actual + (objetivo - actual) * (1 - Math.exp(-velocidad * delta));
}

interface OrbeProps {
  activa: boolean;
  reducido: boolean;
  momento: Momento;
  estado: EstadoEsfera;
}

function Orbe({ activa, reducido, momento, estado }: OrbeProps) {
  const malla = useRef<Mesh>(null);
  const puntero = usePunteroGlobal();

  const uniforms = useMemo(
    () => ({
      uTiempo: { value: 0 },
      uAmplitud: { value: reducido ? 0.015 : 0.028 },
      uRespiracion: { value: reducido ? 0 : 0.018 },
      uImpulso: { value: 0 },
      // 0 → 1 mientras el asistente piensa (se acerca suave, no salta)
      uPensando: { value: 0 },
      // 1 al fallar algo, decae solo: un destello en el color de aviso
      uError: { value: 0 },
      uLuz: { value: new Color() },
      uBorde: { value: new Color() },
      uAviso: { value: new Color() },
    }),
    [reducido],
  );

  // Los colores dependen del cielo: se vuelven a leer cuando cambia el momento
  // (el atributo data-cielo ya está puesto cuando corre este efecto).
  useEffect(() => {
    uniforms.uLuz.value.copy(colorDeToken("--color-luz"));
    uniforms.uBorde.value.copy(colorDeToken("--color-luz-borde"));
    uniforms.uAviso.value.copy(colorDeToken("--color-aviso"));
  }, [momento, uniforms]);

  useEffect(() => {
    if (estado === "error") uniforms.uError.value = 1;
  }, [estado, uniforms]);

  // Se lee en cada cuadro sin volver a renderizar.
  const pensando = useRef(false);
  pensando.current = estado === "pensando";

  // Cada vez que se abre el panel: un pulso de luz que decae solo.
  useEffect(() => {
    if (activa && !reducido) uniforms.uImpulso.value = 1;
  }, [activa, reducido, uniforms]);

  useFrame((_, deltaReal) => {
    // Al volver de estar pausada, el primer delta puede ser enorme: se limita.
    const delta = Math.min(deltaReal, 0.05);
    uniforms.uPensando.value = amortiguar(uniforms.uPensando.value, pensando.current ? 1 : 0, 3, delta);
    // Pensando, el interior fluye más rápido (con movimiento reducido, no).
    const ritmo = reducido ? 0.4 : 1 + uniforms.uPensando.value * 1.4;
    uniforms.uTiempo.value += delta * ritmo;
    uniforms.uImpulso.value = amortiguar(uniforms.uImpulso.value, 0, 2.2, delta);
    uniforms.uError.value = amortiguar(uniforms.uError.value, 0, 1.6, delta);

    const orbe = malla.current;
    if (!orbe) return;
    // Se inclina apenas hacia el mouse, con inercia
    const inclinacion = reducido ? 0 : 0.3;
    orbe.rotation.x = amortiguar(orbe.rotation.x, puntero.current.y * inclinacion, 2.5, delta);
    orbe.rotation.y = amortiguar(orbe.rotation.y, puntero.current.x * inclinacion, 2.5, delta);
  });

  return (
    <mesh ref={malla}>
      {/* Icosaedro muy subdividido: se ve como una esfera y deforma parejo */}
      <icosahedronGeometry args={[1, 96]} />
      <shaderMaterial
        vertexShader={vertexShader}
        fragmentShader={fragmentShader}
        uniforms={uniforms}
        transparent
        depthWrite={false}
      />
    </mesh>
  );
}

interface EsferaProps {
  /** Si es false, se pausa el render para no gastar CPU/GPU con el panel cerrado. */
  activa: boolean;
  momento: Momento;
  estado: EstadoEsfera;
}

export function Esfera({ activa, momento, estado }: EsferaProps) {
  const reducido = useMemo(() => window.matchMedia("(prefers-reduced-motion: reduce)").matches, []);
  return (
    <Canvas
      frameloop={activa ? "always" : "never"}
      camera={{ position: [0, 0, 3.4], fov: 42 }}
      gl={{ alpha: true, antialias: true, powerPreference: "high-performance" }}
      dpr={[1, 2]}
      style={{ background: "transparent" }}
    >
      <Orbe activa={activa} reducido={reducido} momento={momento} estado={estado} />
    </Canvas>
  );
}
