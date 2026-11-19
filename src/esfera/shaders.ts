// Shaders de la esfera: la fuente de luz del cielo (un sol de día, una luna de noche).
//
// Vértice: cada punto de la esfera se empuja hacia afuera según un ruido 3D
// que fluye con el tiempo, más una onda que la recorre y una respiración lenta.
// Como la forma cambia, las normales se recalculan midiendo el desplazamiento
// en dos puntos vecinos.
//
// Fragmento: la esfera emite luz propia. El centro es el color más claro y
// hacia el borde va tomando el color del "borde"; adentro se mueven nubes de
// luz suaves (ruido). El contorno se desvanece para fundirse con el halo.

const ruidoSimplex = /* glsl */ `
// Simplex noise 3D — Ian McEwan, Ashima Arts (licencia MIT)
vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 permute(vec4 x) { return mod289(((x * 34.0) + 1.0) * x); }
vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }

float snoise(vec3 v) {
  const vec2 C = vec2(1.0 / 6.0, 1.0 / 3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);
  vec3 i = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);
  vec3 g = step(x0.yzx, x0.xyz);
  vec3 l = 1.0 - g;
  vec3 i1 = min(g.xyz, l.zxy);
  vec3 i2 = max(g.xyz, l.zxy);
  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;
  i = mod289(i);
  vec4 p = permute(permute(permute(
            i.z + vec4(0.0, i1.z, i2.z, 1.0))
          + i.y + vec4(0.0, i1.y, i2.y, 1.0))
          + i.x + vec4(0.0, i1.x, i2.x, 1.0));
  float n_ = 0.142857142857;
  vec3 ns = n_ * D.wyz - D.xzx;
  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);
  vec4 x_ = floor(j * ns.z);
  vec4 y_ = floor(j - 7.0 * x_);
  vec4 x = x_ * ns.x + ns.yyyy;
  vec4 y = y_ * ns.x + ns.yyyy;
  vec4 h = 1.0 - abs(x) - abs(y);
  vec4 b0 = vec4(x.xy, y.xy);
  vec4 b1 = vec4(x.zw, y.zw);
  vec4 s0 = floor(b0) * 2.0 + 1.0;
  vec4 s1 = floor(b1) * 2.0 + 1.0;
  vec4 sh = -step(h, vec4(0.0));
  vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;
  vec3 p0 = vec3(a0.xy, h.x);
  vec3 p1 = vec3(a0.zw, h.y);
  vec3 p2 = vec3(a1.xy, h.z);
  vec3 p3 = vec3(a1.zw, h.w);
  vec4 norm = taylorInvSqrt(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
  p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
  vec4 m = max(0.6 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
  m = m * m;
  return 42.0 * dot(m * m, vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}
`;

export const vertexShader = /* glsl */ `
uniform float uTiempo;
uniform float uAmplitud;    // cuánto se deforma en reposo (después: el audio)
uniform float uRespiracion; // intensidad del "respirar"
uniform float uImpulso;     // 1 justo al abrir el panel, decae a 0 (salpicón)
uniform float uPensando;    // 0 a 1 mientras el asistente piensa
uniform float uGol;         // 1 en un gol de Boca, decae a 0
uniform float uNivel;       // volumen de la voz (micrófono o Nova), 0 a 1
uniform float uEscucha;     // 0 a 1 mientras escucha
uniform float uHabla;       // 0 a 1 mientras habla

varying vec3 vNormal;
varying vec3 vVista;
varying vec3 vDir;

${ruidoSimplex}

// Desplazamiento radial de la superficie en la dirección "dir" (vector unitario).
float desplazamiento(vec3 dir) {
  float t = uTiempo;
  float liquido =
      snoise(dir * 0.85 + vec3(0.0, t * 0.18, t * 0.12)) * 0.72
    + snoise(dir * 1.7 - vec3(t * 0.24, 0.0, t * 0.16)) * 0.28;

  // Onda que recorre la superficie desde un polo que se mueve despacio
  vec3 polo = normalize(vec3(sin(t * 0.3), cos(t * 0.21), 0.5));
  float onda = sin(dot(dir, polo) * 6.0 - t * 2.0) * 0.006;

  float amplitud = uAmplitud * (1.0 + uImpulso * 1.2 + uPensando * 0.5 + uGol * 2.2);
  // Gol: además, late rápido como un corazón
  float festejo = uGol * 0.035 * max(sin(t * 9.0), 0.0);

  // Voz: la superficie ondula con el volumen. Escuchando, ondas finas que
  // recorren la esfera (tu voz la "toca"); hablando, pulsos más grandes y
  // se infla un poco con cada sílaba.
  float voz = uNivel * (0.075 * uEscucha + 0.11 * uHabla);
  float ondasVoz = snoise(dir * (2.2 + uEscucha * 1.4) + vec3(t * 1.3, -t * 0.9, t * 0.7)) * voz;
  float inflar = uNivel * uHabla * 0.035;

  return liquido * amplitud + onda * (1.0 + uImpulso * 2.0) + sin(t * 1.05) * uRespiracion + festejo + ondasVoz + inflar;
}

vec3 superficie(vec3 dir) {
  return dir * (1.0 + desplazamiento(dir));
}

void main() {
  vec3 dir = normalize(position);

  // Dos direcciones tangentes para medir la pendiente y rehacer la normal
  vec3 eje = abs(dir.y) < 0.99 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
  vec3 tangente = normalize(cross(eje, dir));
  vec3 bitangente = cross(dir, tangente);

  float e = 0.012;
  vec3 p = superficie(dir);
  vec3 pt = superficie(normalize(dir + tangente * e));
  vec3 pb = superficie(normalize(dir + bitangente * e));
  vec3 normal = normalize(cross(pt - p, pb - p));

  vDir = dir;
  vNormal = normalize(normalMatrix * normal);
  vec4 posVista = modelViewMatrix * vec4(p, 1.0);
  vVista = normalize(-posVista.xyz);
  gl_Position = projectionMatrix * posVista;
}
`;

export const fragmentShader = /* glsl */ `
uniform float uTiempo;
uniform float uImpulso;
uniform vec3 uLuz;      // centro (token --color-luz)
uniform vec3 uBorde;    // hacia el contorno (token --color-luz-borde)
uniform vec3 uAviso;    // destello de error (token --color-aviso)
uniform float uPensando;
uniform float uError;
uniform float uGol;     // gol de Boca: se pinta de azul con la franja de oro
uniform vec3 uGolAzul;  // token --color-gol-azul
uniform vec3 uGolOro;   // token --color-gol-oro
uniform float uNivel;
uniform float uEscucha;
uniform float uHabla;
uniform float uDespertar; // 1 al oír "Oye Nova", decae rápido

varying vec3 vNormal;
varying vec3 vVista;
varying vec3 vDir;

${ruidoSimplex}

void main() {
  vec3 n = normalize(vNormal);
  vec3 v = normalize(vVista);

  // 1 mirando de frente, 0 en el contorno
  float frente = max(dot(n, v), 0.0);

  // Nubes de luz que fluyen despacio por adentro
  float t = uTiempo;
  float nubes = snoise(vDir * 1.6 + vec3(t * 0.08, -t * 0.11, t * 0.05)) * 0.6
              + snoise(vDir * 3.2 - vec3(0.0, t * 0.16, t * 0.07)) * 0.4;

  // Del borde al centro; las nubes corren el límite un poco
  float mezcla = smoothstep(0.05, 0.95, pow(frente, 0.75) + nubes * 0.12);
  vec3 color = mix(uBorde, uLuz, mezcla);

  // Más brillo justo al abrir el panel
  color *= 1.0 + uImpulso * 0.12 + nubes * 0.04;

  // Pensando: late despacio, más fuerte donde hay nubes
  color *= 1.0 + uPensando * (0.05 + 0.05 * sin(t * 3.0) + nubes * 0.06);

  // Escuchando: el borde se aclara (está atenta) y brilla con tu voz.
  color = mix(color, uLuz, (1.0 - frente) * uEscucha * (0.22 + uNivel * 0.3));
  // Hablando: la luz de adentro sube y baja con su voz.
  color *= 1.0 + uNivel * (0.08 * uEscucha + 0.2 * uHabla);

  // "Oye Nova": un pulso de luz que recorre el contorno.
  color = mix(color, uLuz * 1.3, pow(1.0 - frente, 1.5) * uDespertar * 0.7);

  // Error: el borde se tiñe un instante con el color de aviso
  color = mix(color, uAviso, uError * (1.0 - mezcla * 0.6) * 0.75);

  // Gol: azul, con una franja horizontal de oro que ondula como una bandera
  float altura = vDir.y + sin(vDir.x * 4.0 + t * 3.0) * 0.06;
  float franja = 1.0 - smoothstep(0.17, 0.24, abs(altura));
  vec3 camiseta = mix(uGolAzul, uGolOro, franja) * (0.85 + mezcla * 0.35);
  color = mix(color, camiseta, smoothstep(0.0, 0.35, uGol) * 0.92);

  // El contorno se desvanece: la esfera se funde con su halo
  float alfa = smoothstep(0.0, 0.32, frente);

  gl_FragColor = vec4(color, alfa);
}
`;
