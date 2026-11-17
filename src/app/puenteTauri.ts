// Puente tipado con el backend de Rust.
// Si la app corre en un navegador común (npm run dev sin Tauri), las llamadas
// no hacen nada: así se puede iterar la interfaz sin compilar Rust.
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Eventos que emite Rust hacia el frontend. */
export const EVENTOS = {
  abrir: "panel:abrir",
  cerrar: "panel:cerrar",
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
