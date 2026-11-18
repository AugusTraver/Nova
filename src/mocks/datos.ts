// Datos de ejemplo de fútbol y noticias, hasta que lleguen las integraciones
// reales en v0.4. Las fechas se calculan a partir de "ahora" para que el panel
// siempre se vea vigente. (Agenda, pendientes y materias ya son reales: SQLite.)
import type { Noticia, Partido, ResultadoPartido } from "../lib/tipos";

/** Fecha relativa a hoy: `en(2, 9, 30)` = pasado mañana a las 9:30. */
function en(dias: number, hora = 0, minutos = 0): Date {
  const d = new Date();
  d.setDate(d.getDate() + dias);
  d.setHours(hora, minutos, 0, 0);
  return d;
}

const iso = (d: Date) => d.toISOString();

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
