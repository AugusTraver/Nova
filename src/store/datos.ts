import { create } from "zustand";
import { api, mensajeDeError } from "../lib/api";
import { aFecha, sumarDias } from "../lib/fechas";
import type {
  DatosEvento,
  DatosMateria,
  DatosResponsabilidad,
  Materia,
  Ocurrencia,
  Responsabilidad,
} from "../lib/tipos";

/** Cuántos días hacia adelante se cargan (la agenda muestra 4; Facu cuenta hasta acá). */
export const DIAS_CARGADOS = 90;

export type Fuente = "agenda" | "responsabilidades" | "materias";

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
  async function cargar<K extends "ocurrencias" | "responsabilidades" | "materias">(
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
      return cargar("agenda", "ocurrencias", () => api.listarOcurrencias(hoy, sumarDias(hoy, DIAS_CARGADOS)));
    },
    responsabilidades: () => cargar("responsabilidades", "responsabilidades", () => api.listarResponsabilidades()),
    materias: () => cargar("materias", "materias", () => api.listarMaterias()),
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
    fuentes: { agenda: sinCargar, responsabilidades: sinCargar, materias: sinCargar },

    async cargarTodo() {
      try {
        await api.sincronizarAutomaticas();
      } catch (e) {
        console.error("No se pudieron sincronizar las automáticas", e);
      }
      await get().recargar("agenda", "responsabilidades", "materias");
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
