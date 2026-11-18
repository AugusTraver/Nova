// Puente tipado con el backend de Rust.
// Si la app corre en un navegador común (npm run dev sin Tauri), las llamadas
// no hacen nada: así se puede iterar la interfaz sin compilar Rust.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AvisoGol, FuenteDatos } from "../lib/tipos";

/** Eventos que emite Rust hacia el frontend. */
export const EVENTOS = {
  abrir: "panel:abrir",
  cerrar: "panel:cerrar",
  /** El asistente cambió datos (ver comandos.rs, EVENTO_DATOS_CAMBIADOS). */
  datosCambiados: "datos-cambiados",
  /** Gol de Boca o la Selección en vivo (fondo.rs, EVENTO_GOL). */
  gol: "futbol:gol",
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
