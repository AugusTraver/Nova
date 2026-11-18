// Comandos de Rust, tipados (ver src-tauri/src/comandos.rs).
// En el navegador (npm run dev sin Tauri) se usa una versión en memoria con
// datos de ejemplo, para poder iterar la interfaz sin compilar Rust.
import { Channel, invoke } from "@tauri-apps/api/core";
import { enTauri } from "../app/puenteTauri";
import { crearApiMemoria } from "../mocks/apiMemoria";
import type {
  Ajustes,
  AjustesIa,
  DatosEvento,
  DatosMateria,
  DatosResponsabilidad,
  ErrorNova,
  Evento,
  Materia,
  Ocurrencia,
  Progreso,
  Responsabilidad,
  Turno,
  VistaAjustesIa,
} from "./tipos";

export interface Api {
  listarMaterias(): Promise<Materia[]>;
  crearMateria(datos: DatosMateria): Promise<Materia>;
  editarMateria(id: number, datos: DatosMateria): Promise<Materia>;
  borrarMateria(id: number): Promise<void>;

  /** Repeticiones entre dos fechas ("2026-11-17"), ambas incluidas. */
  listarOcurrencias(desde: string, hasta: string): Promise<Ocurrencia[]>;
  crearEvento(datos: DatosEvento): Promise<Evento>;
  editarEvento(id: number, datos: DatosEvento): Promise<Evento>;
  borrarEvento(id: number): Promise<void>;
  saltearFecha(id: number, fecha: string, saltear: boolean): Promise<Evento>;

  listarResponsabilidades(): Promise<Responsabilidad[]>;
  crearResponsabilidad(datos: DatosResponsabilidad): Promise<Responsabilidad>;
  editarResponsabilidad(id: number, datos: DatosResponsabilidad): Promise<Responsabilidad>;
  marcarResponsabilidad(id: number, hecha: boolean): Promise<Responsabilidad>;
  borrarResponsabilidad(id: number): Promise<void>;
  sincronizarAutomaticas(): Promise<void>;

  obtenerAjustes(): Promise<Ajustes>;
  guardarAjustes(ajustes: Ajustes): Promise<Ajustes>;

  /** Un turno con el asistente. `alProgreso` recibe los pasos mientras trabaja. */
  enviarMensaje(texto: string, alProgreso: (p: Progreso) => void): Promise<Turno>;
  /** Respuesta a "¿Borro…?" desde los botones. */
  confirmarAccion(aceptar: boolean, alProgreso: (p: Progreso) => void): Promise<Turno>;
  borrarHistorial(): Promise<void>;
  obtenerAjustesIa(): Promise<VistaAjustesIa>;
  guardarAjustesIa(ajustes: AjustesIa): Promise<VistaAjustesIa>;
  /** Guarda la API key en el llavero ("" la borra). Devuelve si quedó una. */
  guardarApiKey(key: string): Promise<boolean>;
  probarConexion(): Promise<void>;
}

/** Un `Channel` de Tauri: Rust manda mensajes mientras el comando corre. */
function canal(alProgreso: (p: Progreso) => void): Channel<Progreso> {
  const c = new Channel<Progreso>();
  c.onmessage = alProgreso;
  return c;
}

// Los nombres de los argumentos van en camelCase: Tauri los traduce a los
// nombres de Rust (snake_case) solo.
const apiTauri: Api = {
  listarMaterias: () => invoke("listar_materias"),
  crearMateria: (datos) => invoke("crear_materia", { datos }),
  editarMateria: (id, datos) => invoke("editar_materia", { id, datos }),
  borrarMateria: (id) => invoke("borrar_materia", { id }),

  listarOcurrencias: (desde, hasta) => invoke("listar_ocurrencias", { desde, hasta }),
  crearEvento: (datos) => invoke("crear_evento", { datos }),
  editarEvento: (id, datos) => invoke("editar_evento", { id, datos }),
  borrarEvento: (id) => invoke("borrar_evento", { id }),
  saltearFecha: (id, fecha, saltear) => invoke("saltear_fecha", { id, fecha, saltear }),

  listarResponsabilidades: () => invoke("listar_responsabilidades"),
  crearResponsabilidad: (datos) => invoke("crear_responsabilidad", { datos }),
  editarResponsabilidad: (id, datos) => invoke("editar_responsabilidad", { id, datos }),
  marcarResponsabilidad: (id, hecha) => invoke("marcar_responsabilidad", { id, hecha }),
  borrarResponsabilidad: (id) => invoke("borrar_responsabilidad", { id }),
  sincronizarAutomaticas: () => invoke("sincronizar_automaticas"),

  obtenerAjustes: () => invoke("obtener_ajustes"),
  guardarAjustes: (ajustes) => invoke("guardar_ajustes", { ajustes }),

  enviarMensaje: (texto, alProgreso) => invoke("enviar_mensaje", { texto, progreso: canal(alProgreso) }),
  confirmarAccion: (aceptar, alProgreso) => invoke("confirmar_accion", { aceptar, progreso: canal(alProgreso) }),
  borrarHistorial: () => invoke("borrar_historial"),
  obtenerAjustesIa: () => invoke("obtener_ajustes_ia"),
  guardarAjustesIa: (ajustes) => invoke("guardar_ajustes_ia", { ajustes }),
  guardarApiKey: (key) => invoke("guardar_api_key", { key }),
  probarConexion: () => invoke("probar_conexion"),
};

export const api: Api = enTauri ? apiTauri : crearApiMemoria();

function esErrorNova(e: unknown): e is ErrorNova {
  return typeof e === "object" && e !== null && "mensaje" in e && typeof e.mensaje === "string";
}

/** Texto para mostrar al usuario. Los de validación vienen listos de Rust. */
export function mensajeDeError(e: unknown): string {
  if (esErrorNova(e)) {
    return e.tipo === "validacion" ? e.mensaje : "Algo falló al guardar. Probá de nuevo.";
  }
  return "Algo falló. Probá de nuevo.";
}

/**
 * Para el asistente y la conexión con la IA: los errores de sistema también
 * traen un texto pensado para mostrar ("No encuentro Ollama…", "Llegué al
 * límite…").
 */
export function mensajeDelAsistente(e: unknown): string {
  if (esErrorNova(e) && (e.tipo === "validacion" || e.tipo === "sistema" || e.tipo === "no_encontrado")) {
    return e.mensaje;
  }
  return "Algo falló. Probá de nuevo.";
}
