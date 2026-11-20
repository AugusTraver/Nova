// Tipos del dominio. Los de datos locales son espejo de los structs de Rust
// (src-tauri/src/repos/*.rs, con serde en camelCase): si cambia uno, cambia el otro.

export type TipoEvento = "clase" | "examen" | "entrega" | "cumpleanos" | "partido" | "otro";
export type Recurrencia = "ninguna" | "semanal" | "anual";
export type Prioridad = "baja" | "media" | "alta";
export type EstadoResponsabilidad = "pendiente" | "hecha";
export type OrigenResponsabilidad = "manual" | "auto" | "asistente";

/** Paleta de materias (tokens `--color-materia-*`). Igual a `PALETA` en materias.rs. */
export const PALETA_MATERIAS = ["celeste", "coral", "menta", "lavanda", "ambar", "rosa"] as const;
export type ColorMateria = (typeof PALETA_MATERIAS)[number];

export interface Materia {
  id: number;
  nombre: string;
  color: ColorMateria;
}

export interface DatosMateria {
  nombre: string;
  color: ColorMateria;
}

export interface Evento {
  id: number;
  titulo: string;
  tipo: TipoEvento;
  /** ISO 8601 con zona ("2026-11-20T09:00:00-03:00") o fecha sola si es de todo el día. */
  inicio: string;
  fin: string | null;
  todoElDia: boolean;
  materiaId: number | null;
  persona: string | null;
  notas: string | null;
  recurrencia: Recurrencia;
  /** Fecha sola, incluida. */
  repetirHasta: string | null;
  /** Fechas salteadas de la serie. */
  excepciones: string[];
}

export type DatosEvento = Omit<Evento, "id" | "excepciones">;

/** Una repetición concreta de un evento. */
export interface Ocurrencia {
  evento: Evento;
  /** Día en que empieza ("2026-11-20"). */
  fecha: string;
  inicio: string;
  fin: string | null;
}

export interface Responsabilidad {
  id: number;
  titulo: string;
  descripcion: string | null;
  /** Fecha sola ("2026-11-20"). */
  fechaLimite: string | null;
  prioridad: Prioridad;
  estado: EstadoResponsabilidad;
  hechaEn: string | null;
  eventoId: number | null;
  materiaId: number | null;
  origen: OrigenResponsabilidad;
  creadoEn: string;
}

export interface DatosResponsabilidad {
  titulo: string;
  descripcion: string | null;
  fechaLimite: string | null;
  prioridad: Prioridad;
  materiaId: number | null;
  eventoId: number | null;
}

export interface Regla {
  activa: boolean;
  diasAntes: number;
  /** Con marcadores {materia}, {entrega}, {persona}, {titulo}. */
  plantilla: string;
}

export interface Reglas {
  examen: Regla;
  entrega: Regla;
  cumpleanos: Regla;
}

export interface Ajustes {
  /** "Ctrl+Shift+Space": modificadores + `KeyboardEvent.code`. */
  atajo: string;
  reglas: Reglas;
}

/** Error que devuelven los comandos de Rust (ver src-tauri/src/error.rs). */
export interface ErrorNova {
  tipo: "base" | "validacion" | "no_encontrado" | "sistema";
  mensaje: string;
}

// ---- Asistente (ver src-tauri/src/asistente) ----

/** Qué datos cambió el asistente; los widgets recargan solo eso. */
export type FuenteDatos = "agenda" | "responsabilidades" | "materias" | "futbol" | "noticias" | "clima";

/** Cómo terminó un turno con el asistente. */
export interface Turno {
  respuesta: string;
  /** Si quedó un borrado esperando el "sí": la pregunta para los botones. */
  confirmacion: string | null;
  /** Si lo que espera el "sí" es un cambio de archivo: su antes y después. */
  cambio: VistaCambio | null;
}

// ---- Compu (v0.7.1, ver src-tauri/src/compu) ----

export type TipoLinea = "igual" | "sale" | "entra" | "corte";

/** Una línea del antes y después: lo igual de antes, lo que cambia y lo igual de después. */
export interface LineaCambio {
  tipo: TipoLinea;
  inicio: string;
  marca: string;
  fin: string;
}

export type AccionArchivo = "cambiar" | "crear" | "borrar" | "deshacer";

/** Un cambio de archivo esperando el "sí". */
export interface VistaCambio {
  archivo: string;
  /** Dónde está ("Documentos\Facu"). */
  carpeta: string;
  accion: AccionArchivo;
  lineas: LineaCambio[];
  /** Líneas cambiadas que no entraron. */
  mas: number;
  nota: string | null;
}

export interface PaginaGuardada {
  nombre: string;
  direccion: string;
}

export interface AjustesCompu {
  /** El interruptor general del acceso a la compu. */
  activa: boolean;
  carpetas: string[];
  paginas: PaginaGuardada[];
}

/** Lo que ve Ajustes (la key de YouTube nunca sale de Rust). */
export interface VistaAjustesCompu {
  ajustes: AjustesCompu;
  tieneKeyYoutube: boolean;
  carpetasPorDefecto: string[];
}

/** Avisos mientras el asistente trabaja. */
export type Progreso = { tipo: "pensando" } | { tipo: "herramienta"; nombre: string };

export type TipoProveedor = "groq" | "ollama";

export interface AjustesIa {
  proveedor: TipoProveedor;
  modeloGroq: string;
  modeloOllama: string;
  urlOllama: string;
}

/** Ajustes de IA y si hay una API key guardada (la key nunca sale de Rust). */
export interface VistaAjustesIa {
  ajustes: AjustesIa;
  tieneKey: boolean;
}

// ---- Datos de internet (ver src-tauri/src/futbol, noticias, clima) ----

/** Un dato de internet guardado en la caché de Rust. */
export interface Cacheado<T> {
  /** `null` si todavía no se pudo descargar nunca. */
  datos: T | null;
  /** Cuándo se obtuvo lo que se muestra (ISO 8601). */
  obtenidoEn: string | null;
  /** El error del último intento; con `datos`, se muestra lo viejo con un aviso. */
  error: string | null;
}

export interface EquipoBreve {
  id: string;
  nombre: string;
  abreviatura: string;
  escudo: string | null;
}

export type EstadoPartido = "programado" | "en_juego" | "entretiempo" | "terminado" | "suspendido";

export interface EventoPartido {
  /** "45'+1'" */
  minuto: string;
  tipo: "gol" | "gol_en_contra" | "expulsion";
  /** El equipo al que le suma el gol, o el del expulsado. */
  equipoId: string;
  jugador: string | null;
}

export interface Partido {
  id: string;
  liga: string;
  /** "Liga Profesional", "Sudamericana"… */
  competencia: string;
  /** "final", "octavos de final"… */
  fase: string | null;
  /** ISO 8601 (UTC) */
  fecha: string;
  local: EquipoBreve;
  visitante: EquipoBreve;
  estado: EstadoPartido;
  /** "34'" mientras se juega. */
  minuto: string | null;
  golesLocal: number | null;
  golesVisitante: number | null;
  penalesLocal: number | null;
  penalesVisitante: number | null;
  eventos: EventoPartido[];
}

export interface FilaTabla {
  puesto: number;
  equipo: EquipoBreve;
  puntos: number;
  jugados: number;
  ganados: number;
  empatados: number;
  perdidos: number;
  diferencia: number;
}

export interface Posicion {
  torneo: string;
  /** "Zona A" */
  zona: string;
  fila: FilaTabla;
  equiposEnZona: number;
}

export interface ResumenEquipo {
  clave: "boca" | "seleccion";
  id: string;
  nombre: string;
  proximo: Partido | null;
  ultimo: Partido | null;
  enVivo: Partido | null;
  posicion: Posicion | null;
}

export interface PartidoJugador {
  fecha: string;
  rival: string;
  /** "2-1", desde el lado del jugador. */
  resultado: string;
  /** Ganó, empató o perdió. */
  desenlace: "G" | "E" | "P" | null;
  competencia: string | null;
  goles: number;
  asistencias: number;
}

export interface EstadoJugador {
  id: string;
  nombre: string;
  club: string | null;
  ultimo: PartidoJugador | null;
  partidos: number;
  goles: number;
  asistencias: number;
}

export interface VistaFutbol {
  /** Boca primero, después la Selección. */
  equipos: ResumenEquipo[];
  jugadores: EstadoJugador[];
}

/** Un jugador que sigo (Ajustes). */
export interface JugadorSeguido {
  id: string;
  nombre: string;
}

/** Un resultado del buscador de jugadores. */
export interface JugadorEncontrado {
  id: string;
  nombre: string;
  club: string | null;
}

/** Evento `futbol:gol` de Rust. */
export interface AvisoGol {
  equipo: "boca" | "seleccion";
  goles: number;
}

export interface Noticia {
  id: string;
  titulo: string;
  /** Una línea (la escribe la IA o es la bajada del feed); puede venir vacío. */
  resumen: string;
  fuente: string;
  url: string;
  publicado: string | null;
}

export interface Feed {
  nombre: string;
  url: string;
}

export interface AjustesNoticias {
  /** Texto libre: "Boca, Selección Argentina, IA". */
  intereses: string;
  feeds: Feed[];
}

export interface Clima {
  ciudad: string;
  temperatura: number;
  sensacion: number | null;
  codigo: number;
  /** "despejado", "lluvia leve"… */
  descripcion: string;
  esDeDia: boolean;
  maxima: number | null;
  minima: number | null;
  /** Probabilidad de lluvia del día (%). */
  lluvia: number | null;
}

export interface VistaClima {
  mostrar: boolean;
  clima: Cacheado<Clima>;
}

export interface AjustesClima {
  mostrar: boolean;
  ciudad: string;
  latitud: number;
  longitud: number;
}

// ---- Perfil (ver src-tauri/src/dominio/perfil.rs) ----

export type Tono = "con_onda" | "informal" | "sobrio";

export interface Perfil {
  /** Cómo te llama Nova. Vacío: sin nombre. */
  comoLlamarte: string;
  tono: Tono;
  chistes: boolean;
  /** Texto libre, hasta LARGO_SOBRE_VOS letras. */
  sobreVos: string;
}

export const LARGO_SOBRE_VOS = 400;

// ---- Voz (ver src-tauri/src/voz) ----

export type EstadoVoz =
  | "apagada"
  | "preparando"
  | "escuchando"
  | "oyendo"
  | "transcribiendo"
  | "pensando"
  | "hablando"
  | "pausada"
  | "sin_modelos"
  | "error";

/** Evento `voz:estado`. */
export interface AvisoEstadoVoz {
  estado: EstadoVoz;
  mensaje: string | null;
}

/** Con quién habla Nova (ver `voz/nube.rs`). */
export type ProveedorVoz = "azure" | "elevenlabs" | "openai" | "local";

export interface AjustesVoz {
  activada: boolean;
  /** 1 = normal, de 0,7 a 1,4. */
  velocidad: number;
  leerResumen: boolean;
  usarGpu: boolean;
  /** Con el panel cerrado, "Oye Nova" lo abre y empieza a escuchar. */
  palabraClave: boolean;
  /** Desenchufada, "Oye Nova" descansa. */
  palabraPausaBateria: boolean;
  /** El micrófono elegido por nombre ("" = el de Windows). */
  microfono: string;
  /** Región del recurso de Azure ("eastus"); vacía = voz local. */
  regionAzure: string;
  /** La voz de Microsoft ("es-MX-DaliaNeural"). */
  vozAzure: string;
  proveedor: ProveedorVoz;
  /** El ID de una voz de la cuenta de ElevenLabs ("" = sin elegir). */
  vozEleven: string;
  modeloEleven: "eleven_flash_v2_5" | "eleven_multilingual_v2";
  /** La voz de OpenAI ("coral"). */
  vozOpenai: string;
}

/** Una voz de la cuenta de ElevenLabs. */
export interface VozDeCuenta {
  id: string;
  nombre: string;
  /** "argentine, female" o vacío. */
  detalle: string;
}

/** Lo que devuelve el comparador de voces. */
export interface PruebaVoz {
  /** Cuánto tardó en tener el audio. */
  milisegundos: number;
}

export interface VistaVoz {
  ajustes: AjustesVoz;
  modelosListos: boolean;
  megasFaltantes: number;
  hayGpu: boolean;
  /** Los proveedores con key en el llavero (las keys nunca llegan acá). */
  conKey: ProveedorVoz[];
}

/** Evento `voz:descarga`. */
export interface ProgresoDescarga {
  nombre: string;
  megasBajados: number;
  megasTotales: number;
  terminado: boolean;
  error: string | null;
}

export interface CiudadEncontrada {
  nombre: string;
  /** "Buenos Aires, Argentina" */
  region: string;
  latitud: number;
  longitud: number;
}

/** Avisos del sistema (preferencia `avisos`). Las horas van como "09:00". */
export interface AjustesAvisos {
  eventos: boolean;
  /** 10, 15, 30 o 60. */
  minutosAntes: number;
  vencimientos: boolean;
  horaVencimientos: string;
  partidos: boolean;
  /** "Hora de salir" antes de la primera clase o examen del día. */
  salir: boolean;
  minutosViaje: number;
  noMolestar: boolean;
  desde: string;
  hasta: string;
  noMolestarPausaPalabra: boolean;
  enVoz: boolean;
}
