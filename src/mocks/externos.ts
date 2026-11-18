// Datos de internet de ejemplo, SOLO para ver la interfaz en el navegador sin
// Tauri. Las fechas salen de "ahora" para que el panel siempre se vea vigente.
//   ?vivo    → Boca está jugando (modo en vivo).
//   ?sinred  → todo llega con el aviso de "sin conexión".
import type { Cacheado, Clima, EquipoBreve, EstadoJugador, Noticia, Partido, VistaFutbol } from "../lib/tipos";

const parametros = new URLSearchParams(window.location.search);
const enVivo = parametros.has("vivo");
const sinRed = parametros.has("sinred");

/** Fecha relativa a hoy: `en(2, 9, 30)` = pasado mañana a las 9:30. */
function en(dias: number, hora = 0, minutos = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + dias);
  d.setHours(hora, minutos, 0, 0);
  return d.toISOString();
}

function cacheado<T>(datos: T, errorSinRed: string): Cacheado<T> {
  const haceUnRato = new Date(Date.now() - (sinRed ? 3 * 3_600_000 : 5 * 60_000)).toISOString();
  return { datos, obtenidoEn: haceUnRato, error: sinRed ? errorSinRed : null };
}

const equipo = (id: string, nombre: string, abreviatura: string): EquipoBreve => ({ id, nombre, abreviatura, escudo: null });
const boca = equipo("5", "Boca Juniors", "BOC");
const racing = equipo("15", "Racing Club", "RAC");
const estudiantes = equipo("12", "Estudiantes de La Plata", "EST");
const argentina = equipo("202", "Argentina", "ARG");
const uruguay = equipo("212", "Uruguay", "URU");
const colombia = equipo("208", "Colombia", "COL");

function partido(p: Partial<Partido> & Pick<Partido, "id" | "fecha" | "local" | "visitante">): Partido {
  return {
    liga: "arg.1",
    competencia: "Liga Profesional",
    fase: null,
    estado: "programado",
    minuto: null,
    golesLocal: null,
    golesVisitante: null,
    penalesLocal: null,
    penalesVisitante: null,
    eventos: [],
    ...p,
  };
}

const jugador = (j: Partial<EstadoJugador> & Pick<EstadoJugador, "id" | "nombre" | "club">): EstadoJugador => ({
  ultimo: null,
  partidos: 20,
  goles: 3,
  asistencias: 2,
  ...j,
});

export function futbol(): Cacheado<VistaFutbol> {
  const vivo = enVivo
    ? partido({
        id: "vivo",
        fecha: en(0, new Date().getHours() - 1),
        local: boca,
        visitante: racing,
        estado: "en_juego",
        minuto: "63'",
        golesLocal: 2,
        golesVisitante: 1,
        eventos: [
          { minuto: "12'", tipo: "gol", equipoId: "5", jugador: "Miguel Merentiel" },
          { minuto: "38'", tipo: "gol", equipoId: "15", jugador: "Maravilla Martínez" },
          { minuto: "51'", tipo: "expulsion", equipoId: "15", jugador: "Juan Nardoni" },
          { minuto: "59'", tipo: "gol", equipoId: "5", jugador: "Exequiel Zeballos" },
        ],
      })
    : null;
  return cacheado({
    equipos: [
      {
        clave: "boca",
        id: "5",
        nombre: "Boca",
        enVivo: vivo,
        proximo: enVivo ? partido({ id: "p2", fecha: en(4, 19), local: estudiantes, visitante: boca }) : partido({ id: "p1", fecha: en(0, 21, 30), local: boca, visitante: racing }),
        ultimo: partido({
          id: "u1",
          fecha: en(-5, 19),
          local: estudiantes,
          visitante: boca,
          estado: "terminado",
          golesLocal: 1,
          golesVisitante: 2,
        }),
        posicion: {
          torneo: "Liga Profesional",
          zona: "Zona A",
          equiposEnZona: 15,
          fila: { puesto: 3, equipo: boca, puntos: 31, jugados: 15, ganados: 9, empatados: 4, perdidos: 2, diferencia: 11 },
        },
      },
      {
        clave: "seleccion",
        id: "202",
        nombre: "la Selección",
        enVivo: null,
        proximo: partido({ id: "s1", fecha: en(9, 20), local: argentina, visitante: uruguay, competencia: "Amistoso", liga: "fifa.friendly" }),
        ultimo: partido({
          id: "s0",
          fecha: en(-30, 21),
          local: colombia,
          visitante: argentina,
          estado: "terminado",
          competencia: "Eliminatorias",
          golesLocal: 0,
          golesVisitante: 1,
        }),
        posicion: null,
      },
    ],
    jugadores: [
      jugador({ id: "45843", nombre: "Lionel Messi", club: "Inter Miami", ultimo: { fecha: en(-1, 21), rival: "Orlando", resultado: "3-1", desenlace: "G", competencia: "MLS", goles: 1, asistencias: 1 } }),
      jugador({ id: "153115", nombre: "Leandro Paredes", club: "Boca Juniors", ultimo: { fecha: en(-5, 19), rival: "Estudiantes", resultado: "2-1", desenlace: "G", competencia: "Liga Profesional", goles: 0, asistencias: 0 } }),
      jugador({ id: "320760", nombre: "Valentín Barco", club: "Strasbourg", ultimo: { fecha: en(-3, 16), rival: "Lyon", resultado: "1-1", desenlace: "E", competencia: "Ligue 1", goles: 0, asistencias: 1 } }),
      jugador({ id: "132948", nombre: "Neymar", club: "Santos", ultimo: { fecha: en(-20, 20), rival: "Palmeiras", resultado: "0-2", desenlace: "P", competencia: "Brasileirão", goles: 0, asistencias: 0 } }),
    ],
  }, "Sin conexión con ESPN.");
}

const noticia = (id: string, titulo: string, resumen: string, fuente: string): Noticia => ({
  id,
  titulo,
  resumen,
  fuente,
  url: "https://www.ole.com.ar",
  publicado: en(0, 8),
});

export function noticias(): Cacheado<Noticia[]> {
  return cacheado([
    noticia("1", "Boca confirmó el equipo para el clásico del fin de semana", "Vuelve Paredes al medio tras la lesión.", "Olé"),
    noticia("2", "Un nuevo modelo de IA abierto supera a los comerciales en código", "Se puede correr local con 16 GB de RAM.", "Xataka"),
    noticia("3", "La Selección ya tiene rival para la próxima fecha FIFA", "Será en La Bombonera en diciembre.", "Infobae"),
    noticia("4", "Chrome suma una API para correr modelos en el navegador", "Llega primero a la versión de prueba.", "web.dev"),
    noticia("5", "Barco, figura en el empate del Strasbourg", "Dio la asistencia del gol y jugó los 90.", "La Nación"),
    noticia("6", "Cómo funciona el nuevo CSS anchor positioning", "", "CSS-Tricks"),
  ], "Sin conexión con las fuentes de noticias.");
}

export function clima(ciudad: string): Cacheado<Clima> {
  return cacheado({
    ciudad,
    temperatura: 22.8,
    sensacion: 22.5,
    codigo: 1,
    descripcion: "casi despejado",
    esDeDia: true,
    maxima: 26,
    minima: 17.8,
    lluvia: 65,
  }, "Sin conexión con el servicio del clima.");
}
