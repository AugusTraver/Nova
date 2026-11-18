// Puente tipado con el backend de Rust.
// Si la app corre en un navegador común (npm run dev sin Tauri), las llamadas
// no hacen nada: así se puede iterar la interfaz sin compilar Rust.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AvisoEstadoVoz, AvisoGol, FuenteDatos, ProgresoDescarga } from "../lib/tipos";

/** Eventos que emite Rust hacia el frontend. */
export const EVENTOS = {
  abrir: "panel:abrir",
  cerrar: "panel:cerrar",
  /** El asistente cambió datos (ver comandos.rs, EVENTO_DATOS_CAMBIADOS). */
  datosCambiados: "datos-cambiados",
  /** Gol de Boca o la Selección en vivo (fondo.rs, EVENTO_GOL). */
  gol: "futbol:gol",
  /** Motor de voz (voz/motor.rs). */
  vozEstado: "voz:estado",
  vozNivel: "voz:nivel",
  vozParcial: "voz:parcial",
  vozFinal: "voz:final",
  vozDescarga: "voz:descarga",
} as const;

export const enTauri = isTauri();

/** Oculta la ventana del panel (se llama al terminar la animación de salida). */
export async function ocultarVentana(): Promise<void> {
  if (!enTauri) return;
  await invoke("ocultar_panel");
}

/** Escucha los pedidos de abrir/cerrar que llegan desde el atajo o la bandeja. */
export async function escucharPanel(handlers: {
  onAbrir: () => void;
  onCerrar: () => void;
}): Promise<UnlistenFn> {
  if (!enTauri) return () => {};
  const quitarAbrir = await listen(EVENTOS.abrir, handlers.onAbrir);
  const quitarCerrar = await listen(EVENTOS.cerrar, handlers.onCerrar);
  return () => {
    quitarAbrir();
    quitarCerrar();
  };
}

type OyenteCambios = (fuentes: FuenteDatos[]) => void;
const oyentesLocales = new Set<OyenteCambios>();

/** Para la API en memoria del navegador: avisa cambios como lo haría Rust. */
export function avisarCambiosLocal(fuentes: FuenteDatos[]): void {
  for (const oyente of oyentesLocales) oyente(fuentes);
}

/** Escucha cuando el asistente cambia datos (en Tauri o en el navegador). */
export async function escucharCambios(oyente: OyenteCambios): Promise<UnlistenFn> {
  oyentesLocales.add(oyente);
  const quitarLocal = () => oyentesLocales.delete(oyente);
  if (!enTauri) return quitarLocal;
  const quitarTauri = await listen<FuenteDatos[]>(EVENTOS.datosCambiados, (e) => oyente(e.payload));
  return () => {
    quitarLocal();
    quitarTauri();
  };
}

/** Escucha los goles en vivo que detecta la tarea de fondo. */
export async function escucharGoles(oyente: (aviso: AvisoGol) => void): Promise<UnlistenFn> {
  if (!enTauri) return () => {};
  return listen<AvisoGol>(EVENTOS.gol, (e) => oyente(e.payload));
}

export interface OyentesVoz {
  alEstado: (aviso: AvisoEstadoVoz) => void;
  /** 0 a 1, unas 30 veces por segundo. */
  alNivel: (nivel: number) => void;
  /** Lo que vas diciendo (transcripción en vivo). */
  alParcial: (texto: string) => void;
  /** Terminaste de hablar: el pedido completo. */
  alFinal: (texto: string) => void;
  alDescarga: (progreso: ProgresoDescarga) => void;
}

/** Escucha todos los eventos del motor de voz. */
export async function escucharVoz(o: OyentesVoz): Promise<UnlistenFn> {
  if (!enTauri) return () => {};
  const quitar = await Promise.all([
    listen<AvisoEstadoVoz>(EVENTOS.vozEstado, (e) => o.alEstado(e.payload)),
    listen<number>(EVENTOS.vozNivel, (e) => o.alNivel(e.payload)),
    listen<string>(EVENTOS.vozParcial, (e) => o.alParcial(e.payload)),
    listen<string>(EVENTOS.vozFinal, (e) => o.alFinal(e.payload)),
    listen<ProgresoDescarga>(EVENTOS.vozDescarga, (e) => o.alDescarga(e.payload)),
  ]);
  return () => quitar.forEach((q) => q());
}
