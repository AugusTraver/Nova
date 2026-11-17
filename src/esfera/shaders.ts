// Shaders de la esfera de metal líquido.
//
// Vértice: cada punto de la esfera se empuja hacia afuera según un ruido 3D de
// varias capas que fluye con el tiempo (el "líquido"), más una onda que la
// recorre y una respiración lenta. Como la forma cambia, las normales se
// recalculan midiendo el desplazamiento en dos puntos vecinos.
//
// Fragmento: es un metal, así que su color es lo que refleja. No hay una imagen
// de entorno: el "estudio de fotos" que se refleja (caja de luz arriba, línea de
// horizonte, una luz cálida y una fría) se calcula en el shader. Encima, un
// toque iridiscente en los bordes, como una película de aceite.

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

varying vec3 vNormal;
varying vec3 vVista;

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

  float amplitud = uAmplitud * (1.0 + uImpulso * 1.2);
  return liquido * amplitud + onda * (1.0 + uImpulso * 2.0) + sin(t * 1.05) * uRespiracion;
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

  vNormal = normalize(normalMatrix * normal);
  vec4 posVista = modelViewMatrix * vec4(p, 1.0);
  vVista = normalize(-posVista.xyz);
  gl_Position = projectionMatrix * posVista;
}
`;

export const fragmentShader = /* glsl */ `
uniform float uTiempo;
uniform vec3 uLuzCalida;
uniform vec3 uLuzFria;

varying vec3 vNormal;
varying vec3 vVista;

// El "estudio" que se refleja en el metal, en función de la dirección reflejada.
vec3 entorno(vec3 r) {
  // Fondo: gris muy oscuro que aclara apenas hacia arriba
  vec3 color = mix(vec3(0.012), vec3(0.05), smoothstep(-1.0, 1.0, r.y));

  // Caja de luz rectangular arriba
  float caja = smoothstep(0.55, 0.72, r.y) * smoothstep(0.75, 0.35, abs(r.x));
  color += vec3(1.0) * caja * 1.4;

  // Línea de horizonte fina y brillante
  color += vec3(0.85) * exp(-pow((r.y + 0.04) * 16.0, 2.0)) * 0.55;

  // Luz cálida (ámbar) desde la izquierda y luz fría desde la derecha
  color += uLuzCalida * pow(max(dot(r, normalize(vec3(-0.85, 0.15, 0.5))), 0.0), 22.0) * 2.2;
  color += uLuzFria * pow(max(dot(r, normalize(vec3(0.9, -0.25, 0.35))), 0.0), 16.0) * 1.1;
  return color;
}

void main() {
  vec3 n = normalize(vNormal);
  vec3 v = normalize(vVista);
  vec3 r = reflect(-v, n);

  float borde = 1.0 - max(dot(n, v), 0.0);

  // Metal: el color es el reflejo, levemente cálido
  vec3 color = entorno(r) * vec3(0.96, 0.95, 0.93);

  // Iridiscencia sutil en los bordes (película fina)
  vec3 iris = 0.5 + 0.5 * cos(6.2831 * (borde * 1.4 + uTiempo * 0.04 + vec3(0.0, 0.33, 0.67)));
  color *= mix(vec3(1.0), iris * 1.3, 0.3 * pow(borde, 1.5));

  // Brillo de borde (Fresnel)
  color += vec3(pow(borde, 4.0) * 0.22);

  // Compresión de luces para que los brillos no "quemen"
  color = 1.0 - exp(-color * 1.35);

  gl_FragColor = vec4(color, 1.0);
}
`;
