// Voz de mentira para el navegador (sin Tauri no hay micrófono ni Piper):
// mueve los mismos estados que el motor de Rust, para poder ver la esfera y
// los textos. Con `?voz=no` arranca sin modelos (para ver la descarga).
import type { AjustesVoz, ProveedorVoz, PruebaVoz, VistaVoz, VozDeCuenta } from "../lib/tipos";
import { nivelVoz, useVoz } from "../store/voz";

const sinModelos = new URLSearchParams(window.location.search).get("voz") === "no";

let ajustes: AjustesVoz = {
  activada: true,
  velocidad: 1,
  leerResumen: true,
  usarGpu: true,
  regionAzure: "",
  vozAzure: "es-MX-DaliaNeural",
  proveedor: "azure",
  vozEleven: "",
  modeloEleven: "eleven_flash_v2_5",
  vozOpenai: "coral",
};
let conKey: ProveedorVoz[] = [];
const nombres: Record<ProveedorVoz, string> = { azure: "Azure", elevenlabs: "ElevenLabs", openai: "OpenAI", local: "la voz local" };
let modelosListos = !sinModelos;
let activa = false;
let animacion: number | undefined;
let finDeHabla: number | undefined;

function animar(fuente: "mic" | "voz") {
  window.cancelAnimationFrame(animacion ?? 0);
  const inicio = performance.now();
  const paso = (t: number) => {
    const s = (t - inicio) / 1000;
    // La voz "habla" en sílabas; el micrófono apenas respira.
    nivelVoz.valor = fuente === "voz" ? Math.max(0, Math.sin(s * 11) * 0.5 + Math.sin(s * 3.1) * 0.4) : 0.08 + Math.sin(s * 2) * 0.04;
    animacion = window.requestAnimationFrame(paso);
  };
  animacion = window.requestAnimationFrame(paso);
}

function escuchar() {
  if (!activa || !ajustes.activada) return;
  if (!modelosListos) return useVoz.getState().ponerEstado("sin_modelos", null);
  useVoz.getState().ponerEstado("escuchando", null);
  animar("mic");
}

export const vozDePrueba = {
  estado: async (): Promise<VistaVoz> => ({ ajustes, modelosListos, megasFaltantes: modelosListos ? 0 : 1340, hayGpu: true, conKey }),
  guardarKey: async (proveedor: ProveedorVoz, key: string) => {
    conKey = conKey.filter((p) => p !== proveedor);
    if (key.trim() !== "") conKey.push(proveedor);
    return key.trim() !== "";
  },
  probar: async (proveedor: ProveedorVoz): Promise<PruebaVoz> => {
    if (proveedor !== "local" && !conKey.includes(proveedor)) throw { tipo: "validacion", mensaje: `Falta la key de ${nombres[proveedor]}.` };
    if (proveedor === "azure" && !ajustes.regionAzure) throw { tipo: "validacion", mensaje: "Falta la región de Azure." };
    if (proveedor === "elevenlabs" && !ajustes.vozEleven) throw { tipo: "validacion", mensaje: "Elegí una voz de ElevenLabs." };
    await new Promise((ok) => window.setTimeout(ok, 400));
    void vozDePrueba.hablar("Hola, soy Nova. Mañana a las nueve tenés el parcial de Análisis.");
    return { milisegundos: { azure: 380, elevenlabs: 290, openai: 820, local: 140 }[proveedor] };
  },
  vocesElevenlabs: async (): Promise<VozDeCuenta[]> => {
    if (!conKey.includes("elevenlabs")) throw { tipo: "validacion", mensaje: "Falta la key de ElevenLabs." };
    return [
      { id: "voz0argentina0001", nombre: "Valentina", detalle: "argentine, female" },
      { id: "voz0argentina0002", nombre: "Tomás", detalle: "argentine, male" },
      { id: "voz0neutra0000003", nombre: "Sarah", detalle: "american, female" },
    ];
  },
  guardar: async (nuevos: AjustesVoz) => {
    ajustes = nuevos;
    if (!ajustes.activada) useVoz.getState().ponerEstado("apagada", null);
    else escuchar();
    return ajustes;
  },
  bajar: async () => {
    const total = 1340;
    for (let bajados = 0; bajados <= total; bajados += 67) {
      useVoz.getState().ponerDescarga({ nombre: "Whisper (modelo)", megasBajados: bajados, megasTotales: total, terminado: false, error: null });
      await new Promise((ok) => window.setTimeout(ok, 120));
    }
    useVoz.getState().ponerDescarga({ nombre: "Listo", megasBajados: total, megasTotales: total, terminado: true, error: null });
    modelosListos = true;
    escuchar();
  },
  activar: async () => {
    activa = true;
    escuchar();
    return null;
  },
  desactivar: async () => {
    activa = false;
    window.cancelAnimationFrame(animacion ?? 0);
    nivelVoz.valor = 0;
    useVoz.getState().ponerEstado("apagada", null);
  },
  pausar: async () => {
    if (activa && ajustes.activada && modelosListos) useVoz.getState().ponerEstado("pausada", null);
  },
  reanudar: async () => escuchar(),
  esperar: async () => {
    if (activa) useVoz.getState().ponerEstado("pensando", null);
  },
  hablar: async (texto: string) => {
    if (!activa || !modelosListos) return;
    useVoz.getState().ponerEstado("hablando", null);
    animar("voz");
    window.clearTimeout(finDeHabla);
    finDeHabla = window.setTimeout(escuchar, Math.min(6000, 600 + texto.length * 55));
  },
  callar: async () => {
    window.clearTimeout(finDeHabla);
    escuchar();
  },
};
