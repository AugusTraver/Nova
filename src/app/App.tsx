import { useEffect } from "react";
import { useAsistente } from "../store/asistente";
import { useDatos } from "../store/datos";
import { usePanel } from "../store/panel";
import { Panel } from "./Panel";
import { api } from "../lib/api";
import { nivelVoz, useVoz } from "../store/voz";
import { enTauri, escucharCambios, escucharGoles, escucharPanel, escucharVoz, ocultarVentana } from "./puenteTauri";

export function App() {
  const abierto = usePanel((s) => s.abierto);
  const abrir = usePanel((s) => s.abrir);
  const cerrar = usePanel((s) => s.cerrar);
  const cargarTodo = useDatos((s) => s.cargarTodo);

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

  // El asistente cambió algo: recargan solo los widgets afectados, apenas
  // pasa (sin esperar a que termine de responder).
  useEffect(() => {
    let quitar: (() => void) | undefined;
    let cancelado = false;
    escucharCambios((fuentes) => void useDatos.getState().recargar(...fuentes)).then((fn) => {
      if (cancelado) fn();
      else quitar = fn;
    });
    return () => {
      cancelado = true;
      quitar?.();
    };
  }, []);

  // Gol en vivo: la esfera festeja los de Boca (SPEC 4.4) y el marcador se
  // actualiza. En el navegador, `?gol` simula uno a los 3 segundos.
  useEffect(() => {
    let quitar: (() => void) | undefined;
    let cancelado = false;
    escucharGoles((aviso) => {
      if (aviso.equipo === "boca") usePanel.getState().festejarGol();
      void useDatos.getState().recargar("futbol");
    }).then((fn) => {
      if (cancelado) fn();
      else quitar = fn;
    });
    const prueba =
      !enTauri && new URLSearchParams(window.location.search).has("gol")
        ? window.setTimeout(() => usePanel.getState().festejarGol(), 3000)
        : undefined;
    return () => {
      cancelado = true;
      quitar?.();
      window.clearTimeout(prueba);
    };
  }, []);

  // Voz (v0.5): los eventos del motor de Rust. Cuando terminás de hablar,
  // el pedido va al asistente como si lo hubieras escrito (pero la respuesta
  // es corta y se lee en voz alta).
  useEffect(() => {
    let quitar: (() => void) | undefined;
    let cancelado = false;
    escucharVoz({
      alEstado: ({ estado, mensaje }) => useVoz.getState().ponerEstado(estado, mensaje),
      alNivel: (nivel) => {
        nivelVoz.valor = nivel;
      },
      alParcial: (texto) => useVoz.getState().ponerParcial(texto),
      alFinal: (texto) => {
        useVoz.getState().ponerParcial(null);
        // Si todavía estaba respondiendo lo anterior, se vuelve a escuchar.
        if (!useAsistente.getState().enviar(texto, true)) void api.reanudarEscucha().catch(() => {});
      },
      alDescarga: (p) => useVoz.getState().ponerDescarga(p),
    }).then((fn) => {
      if (cancelado) fn();
      else quitar = fn;
    });
    return () => {
      cancelado = true;
      quitar?.();
    };
  }, []);

  // Abrir el panel = empezar a escuchar (SPEC 5.1). La primera vez en el día,
  // Nova lee el resumen. Cerrarlo suelta el micrófono.
  useEffect(() => {
    if (abierto) {
      api
        .activarVoz()
        .then((resumen) => {
          if (resumen) useAsistente.getState().mostrar(resumen);
        })
        .catch((e: unknown) => console.error("No se pudo activar la voz", e));
    } else {
      nivelVoz.valor = 0;
      void api.desactivarVoz().catch((e: unknown) => console.error("No se pudo apagar la voz", e));
    }
  }, [abierto]);

  // Al cerrar, la respuesta se va: la próxima vez la esfera arranca limpia.
  useEffect(() => {
    if (!abierto) useAsistente.getState().limpiar();
  }, [abierto]);

  // Cada vez que se abre: automáticas al día y datos frescos. Mientras tanto se
  // ven los datos de la vez anterior, así que abrir sigue siendo instantáneo.
  useEffect(() => {
    if (abierto) void cargarTodo();
  }, [abierto, cargarTodo]);

  // Esc: si hay algo a medio editar, lo cancela; si no, cierra el panel.
  useEffect(() => {
    const alTeclear = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented) return;
      const { edicion, terminarEdicion } = usePanel.getState();
      // Si Nova está hablando, Esc primero la calla.
      if (useVoz.getState().estado === "hablando") void api.callar().catch(() => {});
      else if (edicion) terminarEdicion();
      else cerrar();
    };
    window.addEventListener("keydown", alTeclear);
    return () => window.removeEventListener("keydown", alTeclear);
  }, [cerrar]);

  return <Panel abierto={abierto} onCerrar={cerrar} onSalidaCompleta={() => void ocultarVentana()} />;
}
