// Comandos de Rust, tipados (ver src-tauri/src/comandos.rs).
// En el navegador (npm run dev sin Tauri) se usa una versión en memoria con
// datos de ejemplo, para poder iterar la interfaz sin compilar Rust.
import { Channel, invoke } from "@tauri-apps/api/core";
import { enTauri } from "../app/puenteTauri";
import { crearApiMemoria } from "../mocks/apiMemoria";
import type {
  Ajustes,
  AjustesClima,
  AjustesIa,
  AjustesNoticias,
  AjustesVoz,
  Cacheado,
  CiudadEncontrada,
  DatosEvento,
  DatosMateria,
  DatosResponsabilidad,
  ErrorNova,
  Evento,
  JugadorEncontrado,
  JugadorSeguido,
  Materia,
  Noticia,
  Ocurrencia,
  Perfil,
  AjustesAvisos,
  Progreso,
  Responsabilidad,
  Turno,
  VistaAjustesIa,
  AjustesCompu,
  Disposicion,
  VistaAjustesCompu,
  VistaGoogle,
  VistaClima,
  VistaFutbol,
  VistaVoz,
  ProveedorVoz,
  PruebaVoz,
  VozDeCuenta,
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

  /**
   * Un turno con el asistente. `alProgreso` recibe los pasos mientras trabaja.
   * `porVoz`: el pedido fue hablado (la respuesta tiene que ser corta).
   */
  enviarMensaje(texto: string, alProgreso: (p: Progreso) => void, porVoz?: boolean): Promise<Turno>;
  /** Respuesta a "¿Borro…?" desde los botones. */
  confirmarAccion(aceptar: boolean, alProgreso: (p: Progreso) => void): Promise<Turno>;
  borrarHistorial(): Promise<void>;
  obtenerAjustesIa(): Promise<VistaAjustesIa>;
  /** Acceso a la compu (v0.7.1). Rust valida las carpetas. */
  obtenerAjustesCompu(): Promise<VistaAjustesCompu>;
  guardarAjustesCompu(ajustes: AjustesCompu): Promise<VistaAjustesCompu>;
  /** Vacía = borrarla. Devuelve si quedó una guardada. */
  guardarKeyYoutube(key: string): Promise<boolean>;
  /** Google Calendar (v0.7.2). El ID, el secreto y el permiso van al llavero. */
  obtenerGoogle(): Promise<VistaGoogle>;
  /** Los dos vacíos = borrarlos. */
  guardarClienteGoogle(id: string, secreto: string): Promise<VistaGoogle>;
  /** Abre el navegador y espera (hasta 5 minutos) a que aceptes. */
  conectarGoogle(): Promise<VistaGoogle>;
  cancelarConexionGoogle(): Promise<void>;
  /** Le saca el permiso a Nova. No borra eventos. */
  desconectarGoogle(): Promise<VistaGoogle>;
  sincronizarGoogle(): Promise<void>;
  /** Dónde dejó cada widget (v0.7.1). Rust valida y devuelve lo guardado. */
  obtenerDisposicion(): Promise<Disposicion>;
  guardarDisposicion(disposicion: Disposicion): Promise<Disposicion>;
  /** El selector de carpetas de Windows; null si se canceló. */
  elegirCarpeta(): Promise<string | null>;
  guardarAjustesIa(ajustes: AjustesIa): Promise<VistaAjustesIa>;
  /** Guarda la API key en el llavero ("" la borra). Devuelve si quedó una. */
  guardarApiKey(key: string): Promise<boolean>;
  probarConexion(): Promise<void>;

  /** Fútbol desde la caché (la tarea de fondo de Rust es la que descarga). */
  obtenerFutbol(): Promise<Cacheado<VistaFutbol>>;
  /** Se abrió el panel: que Rust mire si algo de internet venció. */
  refrescarExternos(): Promise<void>;
  buscarJugadores(texto: string): Promise<JugadorEncontrado[]>;
  obtenerJugadores(): Promise<JugadorSeguido[]>;
  guardarJugadores(jugadores: JugadorSeguido[]): Promise<JugadorSeguido[]>;

  obtenerNoticias(): Promise<Cacheado<Noticia[]>>;
  obtenerAjustesNoticias(): Promise<AjustesNoticias>;
  guardarAjustesNoticias(ajustes: AjustesNoticias): Promise<AjustesNoticias>;
  /** Abre una dirección web en el navegador del sistema. */
  abrirEnlace(url: string): Promise<void>;

  obtenerClima(): Promise<VistaClima>;
  obtenerAjustesClima(): Promise<AjustesClima>;
  guardarAjustesClima(ajustes: AjustesClima): Promise<AjustesClima>;
  buscarCiudades(texto: string): Promise<CiudadEncontrada[]>;

  obtenerAvisos(): Promise<AjustesAvisos>;
  guardarAvisos(avisos: AjustesAvisos): Promise<AjustesAvisos>;
  /** Muestra una notificación de muestra. */
  probarAviso(): Promise<void>;

  obtenerPerfil(): Promise<Perfil>;
  guardarPerfil(perfil: Perfil): Promise<Perfil>;

  estadoVoz(): Promise<VistaVoz>;
  /** Los micrófonos conectados (por nombre). */
  listarMicrofonos(): Promise<string[]>;
  guardarAjustesVoz(ajustes: AjustesVoz): Promise<AjustesVoz>;
  /** Guarda la key de un proveedor en el llavero ("" la borra). Devuelve si quedó una. */
  guardarKeyVoz(proveedor: ProveedorVoz, key: string): Promise<boolean>;
  /** El comparador: con lo guardado, Nova dice la frase de prueba con esa voz. */
  probarVoz(proveedor: ProveedorVoz): Promise<PruebaVoz>;
  /** Las voces de la cuenta de ElevenLabs (con la key guardada). */
  vocesElevenlabs(): Promise<VozDeCuenta[]>;
  /** Baja Whisper, Piper y los modelos (avisa el progreso con `voz:descarga`). */
  bajarModelosVoz(): Promise<void>;
  /** Se abrió el panel: escuchar. Devuelve el resumen si lo va a leer. */
  activarVoz(): Promise<string | null>;
  desactivarVoz(): Promise<void>;
  pausarEscucha(): Promise<void>;
  reanudarEscucha(): Promise<void>;
  esperarRespuesta(): Promise<void>;
  hablar(texto: string): Promise<void>;
  callar(): Promise<void>;
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

  enviarMensaje: (texto, alProgreso, porVoz = false) =>
    invoke("enviar_mensaje", { texto, porVoz, progreso: canal(alProgreso) }),
  confirmarAccion: (aceptar, alProgreso) => invoke("confirmar_accion", { aceptar, progreso: canal(alProgreso) }),
  borrarHistorial: () => invoke("borrar_historial"),
  obtenerAjustesIa: () => invoke("obtener_ajustes_ia"),
  obtenerAjustesCompu: () => invoke("obtener_ajustes_compu"),
  guardarAjustesCompu: (ajustes) => invoke("guardar_ajustes_compu", { ajustes }),
  guardarKeyYoutube: (key) => invoke("guardar_key_youtube", { key }),
  obtenerGoogle: () => invoke("obtener_google"),
  guardarClienteGoogle: (id, secreto) => invoke("guardar_cliente_google", { id, secreto }),
  conectarGoogle: () => invoke("conectar_google"),
  cancelarConexionGoogle: () => invoke("cancelar_conexion_google"),
  desconectarGoogle: () => invoke("desconectar_google"),
  sincronizarGoogle: () => invoke("sincronizar_google"),
  obtenerDisposicion: () => invoke("obtener_disposicion"),
  guardarDisposicion: (disposicion) => invoke("guardar_disposicion", { disposicion }),
  elegirCarpeta: async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const elegida = await open({ directory: true, multiple: false, title: "Elegí una carpeta para Nova" });
    return typeof elegida === "string" ? elegida : null;
  },
  guardarAjustesIa: (ajustes) => invoke("guardar_ajustes_ia", { ajustes }),
  guardarApiKey: (key) => invoke("guardar_api_key", { key }),
  probarConexion: () => invoke("probar_conexion"),

  obtenerFutbol: () => invoke("obtener_futbol"),
  refrescarExternos: () => invoke("refrescar_externos"),
  buscarJugadores: (texto) => invoke("buscar_jugadores", { texto }),
  obtenerJugadores: () => invoke("obtener_jugadores"),
  guardarJugadores: (jugadores) => invoke("guardar_jugadores", { jugadores }),

  obtenerNoticias: () => invoke("obtener_noticias"),
  obtenerAjustesNoticias: () => invoke("obtener_ajustes_noticias"),
  guardarAjustesNoticias: (ajustes) => invoke("guardar_ajustes_noticias", { ajustes }),
  abrirEnlace: (url) => invoke("abrir_enlace", { url }),

  obtenerClima: () => invoke("obtener_clima"),
  obtenerAjustesClima: () => invoke("obtener_ajustes_clima"),
  guardarAjustesClima: (ajustes) => invoke("guardar_ajustes_clima", { ajustes }),
  buscarCiudades: (texto) => invoke("buscar_ciudades", { texto }),

  obtenerAvisos: () => invoke("obtener_avisos"),
  guardarAvisos: (avisos) => invoke("guardar_avisos", { avisos }),
  probarAviso: () => invoke("probar_aviso"),

  obtenerPerfil: () => invoke("obtener_perfil"),
  guardarPerfil: (perfil) => invoke("guardar_perfil", { perfil }),

  estadoVoz: () => invoke("estado_voz"),
  listarMicrofonos: () => invoke("listar_microfonos"),
  guardarAjustesVoz: (ajustes) => invoke("guardar_ajustes_voz", { ajustes }),
  guardarKeyVoz: (proveedor, key) => invoke("guardar_key_voz", { proveedor, key }),
  probarVoz: (proveedor) => invoke("probar_voz", { proveedor }),
  vocesElevenlabs: () => invoke("voces_elevenlabs"),
  bajarModelosVoz: () => invoke("bajar_modelos_voz"),
  activarVoz: () => invoke("activar_voz"),
  desactivarVoz: () => invoke("desactivar_voz"),
  pausarEscucha: () => invoke("pausar_escucha"),
  reanudarEscucha: () => invoke("reanudar_escucha"),
  esperarRespuesta: () => invoke("esperar_respuesta"),
  hablar: (texto) => invoke("hablar", { texto }),
  callar: () => invoke("callar"),
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
