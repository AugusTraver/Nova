// Datos de ejemplo para v0.1. Las fechas se calculan a partir de "ahora"
// para que el panel siempre se vea vigente. En v0.2 esto se reemplaza por SQLite.
import type {
  Evento,
  Materia,
  Noticia,
  Partido,
  Responsabilidad,
  ResultadoPartido,
} from "../lib/tipos";

/** Fecha relativa a hoy: `en(2, 9, 30)` = pasado mañana a las 9:30. */
function en(dias: number, hora = 0, minutos = 0): Date {
  const d = new Date();
  d.setDate(d.getDate() + dias);
  d.setHours(hora, minutos, 0, 0);
  return d;
}

const iso = (d: Date) => d.toISOString();
const soloFecha = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

export const materias: Materia[] = [
  { id: 1, nombre: "Análisis Matemático II", color: "var(--color-tinta-2)" },
  { id: 2, nombre: "Algoritmos y Estructuras", color: "var(--color-tinta-2)" },
  { id: 3, nombre: "Sistemas Operativos", color: "var(--color-tinta-2)" },
];

export const eventos: Evento[] = [
  { id: 1, titulo: "Clase de Análisis II", tipo: "clase", inicio: iso(en(0, 15)), fin: iso(en(0, 17)), todoElDia: false, materiaId: 1 },
  { id: 2, titulo: "Boca vs. Racing", tipo: "partido", inicio: iso(en(0, 21, 30)), todoElDia: false },
  { id: 3, titulo: "Entrega TP Algoritmos", tipo: "entrega", inicio: iso(en(1, 23, 59)), todoElDia: false, materiaId: 2 },
  { id: 4, titulo: "Cumple de Juan", tipo: "cumpleanos", inicio: iso(en(3)), todoElDia: true, persona: "Juan" },
  { id: 5, titulo: "Parcial de Análisis II", tipo: "examen", inicio: iso(en(4, 9)), todoElDia: false, materiaId: 1 },
  { id: 6, titulo: "Clase de Sistemas Operativos", tipo: "clase", inicio: iso(en(2, 18)), fin: iso(en(2, 21)), todoElDia: false, materiaId: 3 },
  { id: 7, titulo: "Final de Sistemas Operativos", tipo: "examen", inicio: iso(en(12, 14)), todoElDia: false, materiaId: 3 },
];

export const responsabilidades: Responsabilidad[] = [
  { id: 1, titulo: "Pagar la cuota del gimnasio", fechaLimite: soloFecha(en(-1)), prioridad: "media", estado: "pendiente", origen: "manual" },
  { id: 2, titulo: "Terminar TP Algoritmos", fechaLimite: soloFecha(en(0)), prioridad: "alta", estado: "pendiente", origen: "auto", materiaId: 2 },
  { id: 3, titulo: "Comprar regalo para Juan", fechaLimite: soloFecha(en(0)), prioridad: "media", estado: "pendiente", origen: "auto" },
  { id: 4, titulo: "Estudiar para Análisis II", fechaLimite: soloFecha(en(3)), prioridad: "alta", estado: "pendiente", origen: "auto", materiaId: 1 },
  { id: 5, titulo: "Llamar al dentista", prioridad: "baja", estado: "pendiente", origen: "asistente" },
  { id: 6, titulo: "Resumen de la unidad 3", prioridad: "baja", estado: "hecha", origen: "manual", materiaId: 3 },
];

/** Progreso de estudio por materia: [hechas, total]. */
export const progresoMaterias: Record<number, [number, number]> = {
  1: [3, 7],
  2: [5, 6],
  3: [2, 5],
};

export const proximoPartidoBoca: Partido = {
  rival: "Racing",
  competencia: "Liga Profesional",
  fecha: iso(en(0, 21, 30)),
  local: true,
};

export const ultimoResultadoBoca: ResultadoPartido = {
  rival: "Estudiantes",
  competencia: "Liga Profesional",
  fecha: iso(en(-5, 19)),
  local: false,
  golesBoca: 2,
  golesRival: 1,
};

export const tablaBoca = { posicion: 3, puntos: 41, partidos: 21 };

export const proximoPartidoSeleccion: Partido = {
  rival: "Uruguay",
  competencia: "Amistoso",
  fecha: iso(en(9, 20)),
  local: true,
};

export const jugadoresSeguidos = [
  { nombre: "Julián Álvarez", equipo: "Atlético de Madrid", ultimaFecha: "1 gol" },
  { nombre: "Enzo Fernández", equipo: "Chelsea", ultimaFecha: "1 asistencia" },
  { nombre: "Lautaro Martínez", equipo: "Inter", ultimaFecha: "Jugó 90'" },
];

export const noticias: Noticia[] = [
  { id: 1, titulo: "Boca confirmó el equipo para el clásico del fin de semana", resumen: "Vuelve el 5 titular tras la lesión.", fuente: "Olé", url: "https://www.ole.com.ar" },
  { id: 2, titulo: "Un nuevo modelo de IA abierto supera a los comerciales en código", resumen: "Se puede correr local con 16 GB de RAM.", fuente: "Xataka", url: "https://www.xataka.com" },
  { id: 3, titulo: "La Selección ya tiene rival para la próxima fecha FIFA", resumen: "Será en La Bombonera en diciembre.", fuente: "TyC Sports", url: "https://www.tycsports.com" },
  { id: 4, titulo: "React 20 trae compilación automática por defecto", resumen: "Menos useMemo, mismo rendimiento.", fuente: "Dev.to", url: "https://dev.to" },
  { id: 5, titulo: "Julián Álvarez, el argentino con más goles en Europa", resumen: "Lleva 11 en lo que va de la temporada.", fuente: "Infobae", url: "https://www.infobae.com" },
];
