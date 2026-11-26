import { create } from "zustand";
import { api, mensajeDeError } from "../lib/api";
import { aFecha, sumarDias } from "../lib/fechas";
import type {
  Cacheado,
  VistaClima,
  DatosEvento,
  DatosMateria,
  DatosResponsabilidad,
  FuenteDatos,
  Materia,
  Noticia,
  Ocurrencia,
  Responsabilidad,
  VistaFutbol,
  VistaGoogle,
} from "../lib/tipos";

/** Cuántos días hacia adelante se cargan (la agenda muestra 4; Facu cuenta hasta acá). */
export const DIAS_CARGADOS = 90;

export type Fuente = FuenteDatos;

export interface EstadoFuente {
  /** Ya se cargó alguna vez (hay datos para mostrar aunque falle la próxima). */
  cargada: boolean;
  error: string | null;
}

/** `null` = salió bien; si no, el texto del error para mostrar en el formulario. */
export type ResultadoAccion = Promise<string | null>;

interface EstadoDatos {
  materias: Materia[];
  ocurrencias: Ocurrencia[];
  responsabilidades: Responsabilidad[];
  /** Datos de internet: vienen de la caché de Rust (`null` = todavía no se leyó). */
  futbol: Cacheado<VistaFutbol> | null;
  noticias: Cacheado<Noticia[]> | null;
  clima: VistaClima | null;
  /** Google Calendar (v0.7.2): si está conectado y su último error. Se lee con la agenda. */
  google: VistaGoogle | null;
  fuentes: Record<Fuente, EstadoFuente>;

  /** Pone al día las automáticas y recarga todo. Se llama al abrir el panel. */
  cargarTodo(): Promise<void>;
  recargar(...fuentes: Fuente[]): Promise<void>;

  guardarEvento(id: number | null, datos: DatosEvento): ResultadoAccion;
  borrarEvento(id: number): ResultadoAccion;
  saltearFecha(id: number, fecha: string): ResultadoAccion;

  guardarResponsabilidad(id: number | null, datos: DatosResponsabilidad): ResultadoAccion;
  marcarResponsabilidad(id: number, hecha: boolean): ResultadoAccion;
  borrarResponsabilidad(id: number): ResultadoAccion;

  guardarMateria(id: number | null, datos: DatosMateria): ResultadoAccion;
  borrarMateria(id: number): ResultadoAccion;
}

const sinCargar: EstadoFuente = { cargada: false, error: null };

export const useDatos = create<EstadoDatos>()((set, get) => {
  /** Carga una fuente sin tocar las demás: si una falla, las otras siguen. */
  async function cargar<K extends "ocurrencias" | "responsabilidades" | "materias" | "futbol" | "noticias" | "clima">(
    fuente: Fuente,
    clave: K,
    leer: () => Promise<EstadoDatos[K]>,
  ) {
    try {
      const datos = await leer();
      set((s) => ({ [clave]: datos, fuentes: { ...s.fuentes, [fuente]: { cargada: true, error: null } } }) as Partial<EstadoDatos>);
    } catch (e) {
      console.error(`No se pudo cargar ${fuente}`, e);
      set((s) => ({ fuentes: { ...s.fuentes, [fuente]: { ...s.fuentes[fuente], error: mensajeDeError(e) } } }));
    }
  }

  const lectores: Record<Fuente, () => Promise<void>> = {
    agenda: () => {
      const hoy = aFecha(new Date());
      // El estado de Google va aparte: si falla, la agenda se muestra igual.
      void api.obtenerGoogle().then(
        (google) => set({ google }),
        (e: unknown) => console.error("No se pudo leer Google Calendar", e),
      );
      return cargar("agenda", "ocurrencias", () => api.listarOcurrencias(hoy, sumarDias(hoy, DIAS_CARGADOS)));
    },
    responsabilidades: () => cargar("responsabilidades", "responsabilidades", () => api.listarResponsabilidades()),
    materias: () => cargar("materias", "materias", () => api.listarMaterias()),
    // Leen la caché de Rust (no la red): la tarea de fondo la mantiene al día.
    futbol: () => cargar("futbol", "futbol", () => api.obtenerFutbol()),
    noticias: () => cargar("noticias", "noticias", () => api.obtenerNoticias()),
    clima: () => cargar("clima", "clima", () => api.obtenerClima()),
  };

  /** Ejecuta una escritura y recarga lo afectado. */
  async function accion(escribir: () => Promise<unknown>, ...afectadas: Fuente[]): ResultadoAccion {
    try {
      await escribir();
    } catch (e) {
      console.error(e);
      return mensajeDeError(e);
    }
    await get().recargar(...afectadas);
    return null;
  }

  return {
    materias: [],
    ocurrencias: [],
    responsabilidades: [],
    futbol: null,
    noticias: null,
    clima: null,
    google: null,
    fuentes: {
      agenda: sinCargar,
      responsabilidades: sinCargar,
      materias: sinCargar,
      futbol: sinCargar,
      noticias: sinCargar,
      clima: sinCargar,
    },

    async cargarTodo() {
      try {
        await api.sincronizarAutomaticas();
      } catch (e) {
        console.error("No se pudieron sincronizar las automáticas", e);
      }
      // Despierta la tarea de fondo: si algo de internet venció, lo baja y
      // avisa con `datos-cambiados`. Mientras, se muestra lo de la caché.
      void api.refrescarExternos().catch((e) => console.error("No se pudo refrescar", e));
      await get().recargar("agenda", "responsabilidades", "materias", "futbol", "noticias", "clima");
    },

    async recargar(...fuentes) {
      await Promise.all(fuentes.map((f) => lectores[f]()));
    },

    // Un evento puede crear, cambiar o borrar automáticas: se recargan las dos.
    guardarEvento: (id, datos) =>
      accion(() => (id === null ? api.crearEvento(datos) : api.editarEvento(id, datos)), "agenda", "responsabilidades"),
    borrarEvento: (id) => accion(() => api.borrarEvento(id), "agenda", "responsabilidades"),
    saltearFecha: (id, fecha) => accion(() => api.saltearFecha(id, fecha, true), "agenda", "responsabilidades"),

    guardarResponsabilidad: (id, datos) =>
      accion(() => (id === null ? api.crearResponsabilidad(datos) : api.editarResponsabilidad(id, datos)), "responsabilidades"),
    borrarResponsabilidad: (id) => accion(() => api.borrarResponsabilidad(id), "responsabilidades"),

    async marcarResponsabilidad(id, hecha) {
      // Optimista: la interfaz cambia al instante y se corrige si falla.
      const antes = get().responsabilidades;
      set({
        responsabilidades: antes.map((r) => (r.id === id ? { ...r, estado: hecha ? "hecha" : "pendiente" } : r)),
      });
      try {
        const actualizada = await api.marcarResponsabilidad(id, hecha);
        set((s) => ({ responsabilidades: s.responsabilidades.map((r) => (r.id === id ? actualizada : r)) }));
        return null;
      } catch (e) {
        set({ responsabilidades: antes });
        return mensajeDeError(e);
      }
    },

    // El nombre de la materia aparece en títulos de eventos y automáticas.
    guardarMateria: (id, datos) =>
      accion(
        () => (id === null ? api.crearMateria(datos) : api.editarMateria(id, datos)),
        "materias",
        "responsabilidades",
        "agenda",
      ),
    borrarMateria: (id) => accion(() => api.borrarMateria(id), "materias", "responsabilidades", "agenda"),
  };
});
