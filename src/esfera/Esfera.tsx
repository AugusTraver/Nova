import { Canvas, useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import { Color, type Mesh, type ShaderMaterial } from "three";
import { fragmentShader, vertexShader } from "./shaders";

/** Lee un color de los tokens CSS (ej. "--color-acento") como THREE.Color. */
function colorDeToken(token: string): Color {
  const valor = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  // Los tokens usan la sintaxis "rgb(122 146 255)", que THREE.Color no entiende:
  // se extraen los tres números a mano.
  const [r, g, b] = (valor.match(/\d+(\.\d+)?/g) ?? ["255", "255", "255"]).map(Number);
  return new Color(r / 255, g / 255, b / 255);
}

function Orbe() {
  const malla = useRef<Mesh>(null);
  const material = useRef<ShaderMaterial>(null);

  const uniforms = useMemo(
    () => ({
      uTiempo: { value: 0 },
      uAmplitud: { value: 0.07 },
      uRespiracion: { value: 0.035 },
      uColorA: { value: colorDeToken("--color-acento") },
      uColorB: { value: colorDeToken("--color-acento-2") },
    }),
    [],
  );

  useFrame((_, delta) => {
    uniforms.uTiempo.value += delta;
    if (malla.current) malla.current.rotation.y += delta * 0.08;
  });

  return (
    <mesh ref={malla}>
      {/* Icosaedro muy subdividido: se ve como una esfera y deforma parejo */}
      <icosahedronGeometry args={[1, 64]} />
      <shaderMaterial
        ref={material}
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
}

export function Esfera({ activa }: EsferaProps) {
  return (
    <Canvas
      frameloop={activa ? "always" : "never"}
      camera={{ position: [0, 0, 3.2], fov: 45 }}
      gl={{ alpha: true, antialias: true, powerPreference: "high-performance" }}
      dpr={[1, 2]}
      style={{ background: "transparent" }}
    >
      <Orbe />
    </Canvas>
  );
}
