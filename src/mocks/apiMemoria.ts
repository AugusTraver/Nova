// Versión en memoria de la API, SOLO para ver la interfaz en el navegador sin
// Tauri. No replica toda la lógica de Rust (las automáticas, por ejemplo, no
// se generan): la fuente de verdad es src-tauri.
import { avisarCambiosLocal } from "../app/puenteTauri";
import type { Api } from "../lib/api";
import { aFecha, aIsoLocal, desdeFecha, sumarDias } from "../lib/fechas";
import type {
  Ajustes,
  AjustesClima,
  AjustesIa,
  AjustesNoticias,
  Evento,
  JugadorSeguido,
  Materia,
  Ocurrencia,
  Progreso,
  Responsabilidad,
  Turno,
} from "../lib/tipos";
import * as externos from "./externos";
import { vozDePrueba } from "./vozDePrueba";

function pausa(ms: number): Promise<void> {
  return new Promise((ok) => window.setTimeout(ok, ms));
}

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

  let ajustesIa: AjustesIa = {
    proveedor: "groq",
    modeloGroq: "openai/gpt-oss-120b",
    modeloOllama: "qwen3:8b",
    urlOllama: "http://localhost:11434",
  };
  let tieneKey = false;
  let jugadoresSeguidos: JugadorSeguido[] = [
    { id: "45843", nombre: "Lionel Messi" },
    { id: "153115", nombre: "Leandro Paredes" },
    { id: "320760", nombre: "Valentín Barco" },
    { id: "132948", nombre: "Neymar" },
  ];
  let ajustesNoticias: AjustesNoticias = {
    intereses: "Boca, Selección Argentina, fútbol argentino, IA, desarrollo web",
    feeds: [
      { nombre: "Olé", url: "https://www.ole.com.ar/rss/boca-juniors/" },
      { nombre: "Xataka", url: "https://www.xataka.com/feedburner.xml" },
    ],
  };
  let ajustesClima: AjustesClima = { mostrar: true, ciudad: "Buenos Aires", latitud: -34.61, longitud: -58.38 };
  /** Evento esperando el "sí" para borrarse. */
  let pendiente: Evento | null = null;

  function confirmar(aceptar: boolean): Turno {
    const evento = pendiente;
    pendiente = null;
    if (!evento) throw { tipo: "validacion", mensaje: "No hay nada esperando confirmación." };
    if (!aceptar) return { respuesta: "Dale, no toco nada.", confirmacion: null };
    eventos = eventos.filter((e) => e.id !== evento.id);
    responsabilidades = responsabilidades.filter((r) => r.eventoId !== evento.id);
    avisarCambiosLocal(["agenda", "responsabilidades"]);
    return { respuesta: `Listo, borré «${evento.titulo}».`, confirmacion: null };
  }

  /**
   * Un "asistente" de mentira para ver la interfaz: entiende un parcial y un
   * borrado, y para lo demás avisa que en el navegador no hay modelo.
   */
  async function turnoDePrueba(texto: string, alProgreso: (p: Progreso) => void): Promise<Turno> {
    alProgreso({ tipo: "pensando" });
    await pausa(900);
    const t = texto.toLowerCase().trim();
    if (pendiente && /^(s[ií]|dale|ok)/.test(t)) return confirmar(true);
    if (pendiente && /^no/.test(t)) return confirmar(false);
    pendiente = null;

    if (t.includes("borr") && eventos.length > 0) {
      pendiente = eventos[eventos.length - 1];
      return { respuesta: "¿Seguro?", confirmacion: `¿Borro «${pendiente.titulo}»?` };
    }
    if (t.includes("parcial") || t.includes("examen")) {
      alProgreso({ tipo: "herramienta", nombre: "crear_evento" });
      await pausa(600);
      const id = siguiente++;
      eventos = [...eventos, evento(id, { titulo: "Parcial de Análisis", tipo: "examen", inicio: aIsoLocal(dia(2), "09:00"), materiaId: 1 })];
      responsabilidades = [
        ...responsabilidades,
        resp(siguiente++, { titulo: "Estudiar para Análisis Matemático II", fechaLimite: dia(1), prioridad: "alta", origen: "auto", eventoId: id, materiaId: 1 }),
      ];
      avisarCambiosLocal(["agenda", "responsabilidades"]);
      alProgreso({ tipo: "pensando" });
      await pausa(500);
      return {
        respuesta: "Listo, parcial de Análisis pasado mañana a las 9. Te anoté «Estudiar para Análisis Matemático II» para mañana.",
        confirmacion: null,
      };
    }
    return {
      respuesta: "Acá en el navegador no tengo modelo: soy una respuesta de prueba. Abrime desde Nova para hablar en serio.",
      confirmacion: null,
    };
  }

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

    enviarMensaje: turnoDePrueba,
    confirmarAccion: async (aceptar) => {
      await pausa(300);
      return confirmar(aceptar);
    },
    borrarHistorial: () => esperar(undefined),
    obtenerAjustesIa: () => esperar({ ajustes: ajustesIa, tieneKey }),
    guardarAjustesIa: (nuevos) => {
      ajustesIa = nuevos;
      return esperar({ ajustes: ajustesIa, tieneKey });
    },
    guardarApiKey: (key) => {
      tieneKey = key.trim() !== "";
      return esperar(tieneKey);
    },
    probarConexion: async () => {
      await pausa(700);
      if (ajustesIa.proveedor === "groq" && !tieneKey) {
        throw { tipo: "validacion", mensaje: "Falta la API key de Groq. Cargala en Ajustes." };
      }
    },

    // Internet de prueba (ver mocks/externos.ts). Con ?vivo en la dirección,
    // Boca está jugando; con ?sinred, todo viene viejo y con aviso.
    obtenerFutbol: () => esperar(externos.futbol()),
    refrescarExternos: async () => {},
    buscarJugadores: (texto) =>
      esperar(texto.length < 3 ? [] : [{ id: "1", nombre: `${texto} (de prueba)`, club: "Club de prueba" }]),
    obtenerJugadores: () => esperar(jugadoresSeguidos),
    guardarJugadores: (jugadores) => {
      jugadoresSeguidos = jugadores;
      return esperar(jugadores);
    },

    obtenerNoticias: () => esperar(externos.noticias()),
    obtenerAjustesNoticias: () => esperar(ajustesNoticias),
    guardarAjustesNoticias: (nuevos) => {
      ajustesNoticias = nuevos;
      return esperar(nuevos);
    },
    abrirEnlace: async (url) => {
      window.open(url, "_blank", "noopener");
    },

    obtenerClima: () => esperar({ mostrar: ajustesClima.mostrar, clima: externos.clima(ajustesClima.ciudad) }),
    obtenerAjustesClima: () => esperar(ajustesClima),
    guardarAjustesClima: (nuevos) => {
      ajustesClima = nuevos;
      return esperar(nuevos);
    },
    buscarCiudades: (texto) =>
      esperar(texto.length < 2 ? [] : [{ nombre: texto, region: "Buenos Aires, Argentina", latitud: -34.6, longitud: -58.4 }]),

    estadoVoz: vozDePrueba.estado,
    guardarAjustesVoz: vozDePrueba.guardar,
    guardarKeyAzure: vozDePrueba.guardarKeyAzure,
    probarVozAzure: vozDePrueba.probarAzure,
    bajarModelosVoz: vozDePrueba.bajar,
    activarVoz: vozDePrueba.activar,
    desactivarVoz: vozDePrueba.desactivar,
    pausarEscucha: vozDePrueba.pausar,
    reanudarEscucha: vozDePrueba.reanudar,
    esperarRespuesta: vozDePrueba.esperar,
    hablar: vozDePrueba.hablar,
    callar: vozDePrueba.callar,
  };
}
