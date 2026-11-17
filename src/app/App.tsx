import { useEffect } from "react";
import { usePanel } from "../store/panel";
import { Panel } from "./Panel";
import { enTauri, escucharPanel, ocultarVentana } from "./puenteTauri";

export function App() {
  const abierto = usePanel((s) => s.abierto);
  const abrir = usePanel((s) => s.abrir);
  const cerrar = usePanel((s) => s.cerrar);

  // Atajo global y bandeja → eventos de Rust
  useEffect(() => {
    let quitar: (() => void) | undefined;
    let cancelado = false;
    escucharPanel({ onAbrir: abrir, onCerrar: cerrar }).then((fn) => {
      if (cancelado) fn();
      else quitar = fn;
    });
    // En el navegador (sin Tauri) el panel arranca abierto para poder verlo.
    if (!enTauri) abrir();
    return () => {
      cancelado = true;
      quitar?.();
    };
  }, [abrir, cerrar]);

  // Esc cierra
  useEffect(() => {
    const alTeclear = (e: KeyboardEvent) => {
      if (e.key === "Escape") cerrar();
    };
    window.addEventListener("keydown", alTeclear);
    return () => window.removeEventListener("keydown", alTeclear);
  }, [cerrar]);

  return <Panel abierto={abierto} onCerrar={cerrar} onSalidaCompleta={() => void ocultarVentana()} />;
}
