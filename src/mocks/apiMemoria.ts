// Versión en memoria de la API, SOLO para ver la interfaz en el navegador sin
// Tauri. No replica toda la lógica de Rust (las automáticas, por ejemplo, no
// se generan): la fuente de verdad es src-tauri.
import type { Api } from "../lib/api";
import { aFecha, aIsoLocal, desdeFecha, sumarDias } from "../lib/fechas";
import type { Ajustes, Evento, Materia, Ocurrencia, Responsabilidad } from "../lib/tipos";

function esperar<T>(valor: T): Promise<T> {
  return new Promise((ok) => window.setTimeout(() => ok(structuredClone(valor)), 60));
}

export function crearApiMemoria(): Api {
  const hoy = aFecha(new Date());
  const dia = (n: number) => sumarDias(hoy, n);
  const ahora = new Date().toISOString();
  let siguiente = 100;

  let materias: Materia[] = [
    { id: 1, nombre: "Análisis Matemático II", color: "celeste" },
    { id: 2, nombre: "Algoritmos y Estructuras", color: "coral" },
    { id: 3, nombre: "Sistemas Operativos", color: "menta" },
  ];

  const evento = (id: number, e: Partial<Evento> & Pick<Evento, "titulo" | "tipo" | "inicio">): Evento => ({
    id,
    fin: null,
    todoElDia: e.inicio.length === 10,
    materiaId: null,
    persona: null,
    notas: null,
    recurrencia: "ninguna",
    repetirHasta: null,
    excepciones: [],
    ...e,
  });

  let eventos: Evento[] = [
    evento(1, { titulo: "Análisis II", tipo: "clase", inicio: aIsoLocal(dia(0), "15:00"), fin: aIsoLocal(dia(0), "17:00"), materiaId: 1, recurrencia: "semanal" }),
    evento(2, { titulo: "Entrega TP Algoritmos", tipo: "entrega", inicio: aIsoLocal(dia(1), "23:59"), materiaId: 2 }),
    evento(3, { titulo: "Cumple de Juan", tipo: "cumpleanos", inicio: dia(3), persona: "Juan", recurrencia: "anual" }),
    evento(4, { titulo: "Parcial de Análisis II", tipo: "examen", inicio: aIsoLocal(dia(4), "09:00"), materiaId: 1 }),
    evento(5, { titulo: "Sistemas Operativos", tipo: "clase", inicio: aIsoLocal(dia(2), "18:00"), fin: aIsoLocal(dia(2), "21:00"), materiaId: 3, recurrencia: "semanal" }),
  ];

  const resp = (id: number, r: Partial<Responsabilidad> & Pick<Responsabilidad, "titulo">): Responsabilidad => ({
    id,
    descripcion: null,
    fechaLimite: null,
    prioridad: "media",
    estado: "pendiente",
    hechaEn: null,
    eventoId: null,
    materiaId: null,
    origen: "manual",
    creadoEn: ahora,
    ...r,
  });

  let responsabilidades: Responsabilidad[] = [
    resp(1, { titulo: "Pagar la cuota del gimnasio", fechaLimite: dia(-1) }),
    resp(2, { titulo: "Terminar Entrega TP Algoritmos", fechaLimite: dia(0), prioridad: "alta", origen: "auto", eventoId: 2, materiaId: 2 }),
    resp(3, { titulo: "Comprar regalo para Juan", fechaLimite: dia(0), origen: "auto", eventoId: 3 }),
    resp(4, { titulo: "Estudiar para Análisis Matemático II", fechaLimite: dia(3), prioridad: "alta", origen: "auto", eventoId: 4, materiaId: 1 }),
    resp(5, { titulo: "Llamar al dentista", prioridad: "baja" }),
    resp(6, { titulo: "Resumen de la unidad 3", estado: "hecha", materiaId: 3 }),
    resp(7, { titulo: "Guía de ejercicios 4", estado: "hecha", materiaId: 1 }),
  ];

  let ajustes: Ajustes = {
    atajo: "Ctrl+Shift+Space",
    reglas: {
      examen: { activa: true, diasAntes: 1, plantilla: "Estudiar para {materia}" },
      entrega: { activa: true, diasAntes: 1, plantilla: "Terminar {entrega}" },
      cumpleanos: { activa: true, diasAntes: 3, plantilla: "Comprar regalo para {persona}" },
    },
  };

  /** Repeticiones aproximadas (sin eventos de varios días en series). */
  function repeticiones(e: Evento, desde: string, hasta: string): Ocurrencia[] {
    const primera = e.inicio.slice(0, 10);
    const mover = (texto: string | null, dias: number) =>
      texto === null ? null : texto.length === 10 ? sumarDias(texto, dias) : `${sumarDias(texto, dias)}${texto.slice(10)}`;
    const fechas: string[] = [];
    if (e.recurrencia === "ninguna") fechas.push(primera);
    if (e.recurrencia === "semanal") for (let f = primera; f <= hasta; f = sumarDias(f, 7)) fechas.push(f);
    if (e.recurrencia === "anual") {
      for (let anio = desdeFecha(desde).getFullYear(); anio <= desdeFecha(hasta).getFullYear(); anio++) {
        fechas.push(`${anio}${primera.slice(4)}`);
      }
    }
    return fechas
      .filter((f) => f >= primera && f >= desde && f <= hasta && !e.excepciones.includes(f))
      .filter((f) => !e.repetirHasta || f <= e.repetirHasta)
      .map((f) => {
        const dias = Math.round((desdeFecha(f).getTime() - desdeFecha(primera).getTime()) / 86_400_000);
        return { evento: e, fecha: f, inicio: mover(e.inicio, dias) ?? e.inicio, fin: mover(e.fin, dias) };
      });
  }

  return {
    listarMaterias: () => esperar(materias),
    crearMateria: (datos) => {
      const m = { id: siguiente++, ...datos };
      materias = [...materias, m];
      return esperar(m);
    },
    editarMateria: (id, datos) => {
      materias = materias.map((m) => (m.id === id ? { ...m, ...datos } : m));
      return esperar(materias.find((m) => m.id === id)!);
    },
    borrarMateria: (id) => {
      materias = materias.filter((m) => m.id !== id);
      return esperar(undefined);
    },

    listarOcurrencias: (desde, hasta) =>
      esperar(
        eventos
          .flatMap((e) => repeticiones(e, desde, hasta))
          .sort((a, b) => a.fecha.localeCompare(b.fecha) || Number(a.evento.todoElDia ? 0 : 1) - Number(b.evento.todoElDia ? 0 : 1) || a.inicio.localeCompare(b.inicio)),
      ),
    crearEvento: (datos) => {
      const e = { id: siguiente++, excepciones: [], ...datos };
      eventos = [...eventos, e];
      return esperar(e);
    },
    editarEvento: (id, datos) => {
      eventos = eventos.map((e) => (e.id === id ? { ...e, ...datos } : e));
      return esperar(eventos.find((e) => e.id === id)!);
    },
    borrarEvento: (id) => {
      eventos = eventos.filter((e) => e.id !== id);
      responsabilidades = responsabilidades.filter((r) => !(r.eventoId === id && r.origen === "auto" && r.estado === "pendiente"));
      return esperar(undefined);
    },
    saltearFecha: (id, fecha, saltear) => {
      eventos = eventos.map((e) =>
        e.id === id
          ? { ...e, excepciones: saltear ? [...e.excepciones, fecha] : e.excepciones.filter((f) => f !== fecha) }
          : e,
      );
      return esperar(eventos.find((e) => e.id === id)!);
    },

    listarResponsabilidades: () => esperar(responsabilidades),
    crearResponsabilidad: (datos) => {
      const r = resp(siguiente++, datos);
      responsabilidades = [...responsabilidades, r];
      return esperar(r);
    },
    editarResponsabilidad: (id, datos) => {
      responsabilidades = responsabilidades.map((r) => (r.id === id ? { ...r, ...datos } : r));
      return esperar(responsabilidades.find((r) => r.id === id)!);
    },
    marcarResponsabilidad: (id, hecha) => {
      responsabilidades = responsabilidades.map((r) =>
        r.id === id ? { ...r, estado: hecha ? "hecha" : "pendiente", hechaEn: hecha ? new Date().toISOString() : null } : r,
      );
      return esperar(responsabilidades.find((r) => r.id === id)!);
    },
    borrarResponsabilidad: (id) => {
      responsabilidades = responsabilidades.filter((r) => r.id !== id);
      return esperar(undefined);
    },
    sincronizarAutomaticas: () => esperar(undefined),

    obtenerAjustes: () => esperar(ajustes),
    guardarAjustes: (nuevos) => {
      ajustes = nuevos;
      return esperar(ajustes);
    },
  };
}
