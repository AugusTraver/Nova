import { Canvas, useFrame } from "@react-three/fiber";
import { useEffect, useMemo, useRef } from "react";
import { Color, type Mesh } from "three";
import { fragmentShader, vertexShader } from "./shaders";

/** Lee un color de los tokens CSS (ej. "--color-acento") como THREE.Color. */
function colorDeToken(token: string): Color {
  const valor = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  // Los tokens usan la sintaxis "rgb(122 146 255)", que THREE.Color no entiende:
  // se extraen los tres números a mano.
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

function Orbe({ activa, reducido }: { activa: boolean; reducido: boolean }) {
  const malla = useRef<Mesh>(null);
  const puntero = usePunteroGlobal();

  const uniforms = useMemo(
    () => ({
      uTiempo: { value: 0 },
      uAmplitud: { value: reducido ? 0.05 : 0.09 },
      uRespiracion: { value: reducido ? 0 : 0.025 },
      uImpulso: { value: 0 },
      uLuzCalida: { value: colorDeToken("--color-luz-calida") },
      uLuzFria: { value: colorDeToken("--color-luz-fria") },
    }),
    [reducido],
  );

  // Cada vez que se abre el panel: "salpicón" que decae solo.
  useEffect(() => {
    if (activa && !reducido) uniforms.uImpulso.value = 1;
  }, [activa, reducido, uniforms]);

  useFrame((_, deltaReal) => {
    // Al volver de estar pausada, el primer delta puede ser enorme: se limita.
    const delta = Math.min(deltaReal, 0.05);
    uniforms.uTiempo.value += delta * (reducido ? 0.4 : 1);
    uniforms.uImpulso.value = amortiguar(uniforms.uImpulso.value, 0, 2.2, delta);

    const orbe = malla.current;
    if (!orbe) return;
    // Se inclina apenas hacia el mouse, con inercia (efecto resorte)
    const inclinacion = reducido ? 0 : 0.35;
    orbe.rotation.x = amortiguar(orbe.rotation.x, puntero.current.y * inclinacion, 2.5, delta);
    orbe.rotation.y = amortiguar(orbe.rotation.y, puntero.current.x * inclinacion, 2.5, delta);
  });

  return (
    <mesh ref={malla}>
      {/* Icosaedro muy subdividido: se ve como una esfera y deforma parejo */}
      <icosahedronGeometry args={[1, 96]} />
      <shaderMaterial vertexShader={vertexShader} fragmentShader={fragmentShader} uniforms={uniforms} />
    </mesh>
  );
}

interface EsferaProps {
  /** Si es false, se pausa el render para no gastar CPU/GPU con el panel cerrado. */
  activa: boolean;
}

export function Esfera({ activa }: EsferaProps) {
  const reducido = useMemo(() => window.matchMedia("(prefers-reduced-motion: reduce)").matches, []);
  return (
    <Canvas
      frameloop={activa ? "always" : "never"}
      camera={{ position: [0, 0, 3.4], fov: 42 }}
      gl={{ alpha: true, antialias: true, powerPreference: "high-performance" }}
      dpr={[1, 2]}
      style={{ background: "transparent" }}
    >
      <Orbe activa={activa} reducido={reducido} />
    </Canvas>
  );
}
