// Versión en memoria de la API, SOLO para ver la interfaz en el navegador sin
// Tauri. No replica toda la lógica de Rust (las automáticas, por ejemplo, no
// se generan): la fuente de verdad es src-tauri.
import { avisarCambiosLocal } from "../app/puenteTauri";
import type { Api } from "../lib/api";
import { aFecha, aIsoLocal, desdeFecha, sumarDias } from "../lib/fechas";
import type {
  Ajustes,
  AjustesClima,
  AjustesCompu,
  Disposicion,
  LineaCambio,
  VistaCambio,
  AjustesIa,
  AjustesNoticias,
  Evento,
  JugadorSeguido,
  Materia,
  Ocurrencia,
  Perfil,
  AjustesAvisos,
  Progreso,
  Responsabilidad,
  Turno,
  VistaGoogle,
} from "../lib/tipos";
import { LARGO_SOBRE_VOS } from "../lib/tipos";
import * as externos from "./externos";
import { vozDePrueba } from "./vozDePrueba";

// Google Calendar de prueba (v0.7.2): `?calendar` arranca conectado, con una
// serie de solo lectura, y a los 3 segundos "llega" un examen cargado desde
// el celular; `?calendarsinred` muestra el aviso de sin conexión.
const parametrosCalendar = new URLSearchParams(window.location.search);
const calendarConectado = parametrosCalendar.has("calendar") || parametrosCalendar.has("calendarsinred");
const calendarSinRed = parametrosCalendar.has("calendarsinred");

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
    deGoogle: false,
    soloLectura: false,
    ...e,
  });

  let eventos: Evento[] = [
    evento(1, { titulo: "Análisis II", tipo: "clase", inicio: aIsoLocal(dia(0), "15:00"), fin: aIsoLocal(dia(0), "17:00"), materiaId: 1, recurrencia: "semanal" }),
    evento(2, { titulo: "Entrega TP Algoritmos", tipo: "entrega", inicio: aIsoLocal(dia(1), "23:59"), materiaId: 2 }),
    evento(3, { titulo: "Cumple de Juan", tipo: "cumpleanos", inicio: dia(3), persona: "Juan", recurrencia: "anual" }),
    evento(4, { titulo: "Parcial de Análisis II", tipo: "examen", inicio: aIsoLocal(dia(4), "09:00"), materiaId: 1 }),
    evento(5, { titulo: "Sistemas Operativos", tipo: "clase", inicio: aIsoLocal(dia(2), "18:00"), fin: aIsoLocal(dia(2), "21:00"), materiaId: 3, recurrencia: "semanal" }),
  ];
  if (calendarConectado) {
    eventos = [
      ...eventos,
      evento(6, { titulo: "Turno con el dentista", tipo: "otro", inicio: aIsoLocal(dia(1), "10:30"), deGoogle: true }),
      // Lunes, miércoles y viernes: Nova no la sabe repetir, se edita en Calendar.
      evento(7, { titulo: "Gimnasio", tipo: "otro", inicio: aIsoLocal(dia(2), "08:00"), deGoogle: true, soloLectura: true }),
    ];
  }

  let google: VistaGoogle = {
    tieneCliente: calendarConectado,
    conectado: calendarConectado,
    conectando: false,
    error: calendarSinRed ? "Sin conexión con Google Calendar" : null,
    sincronizadoEn: calendarConectado ? new Date(Date.now() - (calendarSinRed ? 3 : 0.1) * 3600_000).toISOString() : null,
    primeraVez: calendarConectado ? { traidos: 2, subidos: 5, juntados: 0, borrados: 0 } : null,
  };
  let cancelarConexion = false;

  if (parametrosCalendar.has("calendar")) {
    window.setTimeout(() => {
      eventos = [
        ...eventos,
        evento(8, { titulo: "Parcial de Sistemas Operativos", tipo: "examen", inicio: aIsoLocal(dia(5), "18:00"), materiaId: 3, deGoogle: true }),
      ];
      responsabilidades = [
        ...responsabilidades,
        resp(8, { titulo: "Estudiar para Sistemas Operativos", fechaLimite: dia(4), prioridad: "alta", origen: "auto", eventoId: 8, materiaId: 3 }),
      ];
      google = { ...google, sincronizadoEn: new Date().toISOString() };
      avisarCambiosLocal(["agenda", "responsabilidades"]);
    }, 3000);
  }

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
  let perfil: Perfil = { comoLlamarte: "", tono: "con_onda", chistes: false, sobreVos: "" };
  let avisos: AjustesAvisos = {
    eventos: true,
    minutosAntes: 30,
    vencimientos: true,
    horaVencimientos: "09:00",
    partidos: true,
    salir: true,
    minutosViaje: 60,
    noMolestar: true,
    desde: "23:00",
    hasta: "08:00",
    noMolestarPausaPalabra: true,
    enVoz: false,
  };
  /** Evento esperando el "sí" para borrarse. */
  let pendiente: Evento | null = null;
  /** Un cambio de archivo de prueba esperando el "sí" (v0.7.1). */
  let archivoPendiente: { vista: VistaCambio; hecho: string; despues: string } | null = null;
  /** El "notas.txt" de prueba y cómo estaba antes del último cambio. */
  let notas = "Reunión de grupo el martes\nLlevar los apuntes de Física\nComprar yerba\n";
  let notasAntes: string | null = null;
  let disposicion: Disposicion = {};
  let ajustesCompu: AjustesCompu = {
    activa: true,
    carpetas: [String.raw`C:\Users\vos\Documents`, String.raw`C:\Users\vos\Desktop`, String.raw`C:\Users\vos\Downloads`],
    paginas: [{ nombre: "Facultad", direccion: "https://www.frba.utn.edu.ar/" }],
  };
  let tieneKeyYoutube = false;

  function confirmar(aceptar: boolean): Turno {
    if (archivoPendiente) {
      const cambio = archivoPendiente;
      archivoPendiente = null;
      if (!aceptar) return { respuesta: "Dale, no toco nada.", confirmacion: null, cambio: null };
      notasAntes = cambio.vista.accion === "deshacer" ? null : notas;
      notas = cambio.despues;
      return { respuesta: cambio.hecho, confirmacion: null, cambio: null };
    }
    const evento = pendiente;
    pendiente = null;
    if (!evento) throw { tipo: "validacion", mensaje: "No hay nada esperando confirmación." };
    if (!aceptar) return { respuesta: "Dale, no toco nada.", confirmacion: null, cambio: null };
    eventos = eventos.filter((e) => e.id !== evento.id);
    responsabilidades = responsabilidades.filter((r) => r.eventoId !== evento.id);
    avisarCambiosLocal(["agenda", "responsabilidades"]);
    return { respuesta: `Listo, borré «${evento.titulo}».`, confirmacion: null, cambio: null };
  }

  /** Un antes y después simple: las líneas que cambian, con una de contexto. */
  function vistaDePrueba(antes: string, despues: string, accion: VistaCambio["accion"]): VistaCambio {
    const a = antes.split("\n").filter(Boolean);
    const d = despues.split("\n").filter(Boolean);
    const lineas: LineaCambio[] = [];
    for (let i = 0; i < Math.max(a.length, d.length); i++) {
      if (a[i] === d[i]) {
        if (a[i - 1] !== d[i - 1] || a[i + 1] !== d[i + 1]) lineas.push({ tipo: "igual", inicio: a[i], marca: "", fin: "" });
        continue;
      }
      if (a[i] !== undefined && d[i] !== undefined) {
        let pre = 0;
        while (pre < a[i].length && a[i][pre] === d[i][pre]) pre++;
        while (pre > 0 && /\p{L}/u.test(a[i][pre - 1])) pre--;
        let suf = 0;
        while (suf < a[i].length - pre && suf < d[i].length - pre && a[i][a[i].length - 1 - suf] === d[i][d[i].length - 1 - suf]) suf++;
        while (suf > 0 && /\p{L}/u.test(a[i][a[i].length - suf])) suf--;
        const partir = (l: string) => ({ inicio: l.slice(0, pre), marca: l.slice(pre, l.length - suf), fin: l.slice(l.length - suf) });
        lineas.push({ tipo: "sale", ...partir(a[i]) }, { tipo: "entra", ...partir(d[i]) });
      } else if (a[i] !== undefined) {
        lineas.push({ tipo: "sale", inicio: "", marca: a[i], fin: "" });
      } else {
        lineas.push({ tipo: "entra", inicio: "", marca: d[i], fin: "" });
      }
    }
    return { archivo: "notas.txt", carpeta: "Documentos", accion, lineas: lineas.slice(0, 8), mas: Math.max(0, lineas.length - 8), nota: null };
  }

  /** Lo de la compu en el navegador: no abre nada, solo dice qué haría. */
  async function compuDePrueba(t: string, alProgreso: (p: Progreso) => void): Promise<Turno | null> {
    const hecho = async (herramienta: string, respuesta: string): Promise<Turno> => {
      alProgreso({ tipo: "herramienta", nombre: herramienta });
      await pausa(500);
      return { respuesta, confirmacion: null, cambio: null };
    };
    const preguntar = async (herramienta: string, despues: string, accion: VistaCambio["accion"], pregunta: string, dicho: string) => {
      alProgreso({ tipo: "herramienta", nombre: herramienta });
      await pausa(500);
      const vista = vistaDePrueba(notas, despues, accion);
      archivoPendiente = { vista, hecho: dicho, despues };
      return { respuesta: pregunta, confirmacion: pregunta, cambio: vista };
    };
    if (!ajustesCompu.activa && /(abr[ií]|poneme|google|youtube|archivo|\.txt)/.test(t)) {
      return { respuesta: "El acceso a la compu está apagado en Ajustes.", confirmacion: null, cambio: null };
    }
    const cambio = /cambi[aá] (.+?) por (.+?)$/.exec(t);
    if (cambio && (t.includes(".txt") || t.includes("archivo") || t.includes("notas"))) {
      const [, buscar, poner] = cambio;
      if (!notas.includes(buscar)) return { respuesta: `No encontré «${buscar}» en notas.txt.`, confirmacion: null, cambio: null };
      return preguntar(
        "cambiar_archivo",
        notas.split(buscar).join(poner),
        "cambiar",
        "¿Cambio notas.txt así?",
        `Listo, en notas.txt cambié «${buscar}» por «${poner}». Si querés, lo deshago.`,
      );
    }
    if (/agreg[aá]le/.test(t) && t.includes("notas")) {
      const linea = t.split(/l[ií]nea:?\s*/)[1]?.trim() || "Llamar a la abuela";
      return preguntar("cambiar_archivo", `${notas}${linea}\n`, "cambiar", "¿Cambio notas.txt así?", "Listo, le agregué la línea a notas.txt.");
    }
    if (/deshac[eé]/.test(t)) {
      if (notasAntes === null) return { respuesta: "No tengo nada para deshacer.", confirmacion: null, cambio: null };
      return preguntar("deshacer_cambio", notasAntes, "deshacer", "¿Dejo notas.txt como estaba?", "Listo, notas.txt quedó como estaba.");
    }
    if (/qu[eé] dice|le[eé]/.test(t) && t.includes("notas")) {
      return hecho("leer_archivo", `Dice: ${notas.trim().split("\n").join(", ")}.`);
    }
    if (t.includes("windows") || t.includes("system32")) {
      return hecho("leer_archivo", "Eso está fuera de las carpetas que me dejaste usar, así que no lo toco.");
    }
    if (t.includes("youtube") || t.includes("video")) {
      const busqueda = t.split(/busc[aá]s |video de |youtube /).pop()?.trim() || "lo-fi";
      return hecho(
        "poner_video",
        tieneKeyYoutube
          ? `Te puse el primer video de «${busqueda}» (de prueba).`
          : `Te abrí la lista de YouTube de «${busqueda}»: no tengo la key de YouTube (se carga en Ajustes).`,
      );
    }
    if (t.includes("google")) {
      return hecho("buscar_en_google", `Te abrí la búsqueda de Google de «${t.split("google ").pop()?.trim() ?? ""}» (de prueba).`);
    }
    if (/^(abr[ií]|abrime)\s/.test(t)) {
      const que = t.replace(/^(abr[ií]|abrime)\s+(la |el )?/, "");
      if (que.includes("carpeta") || que.includes("descargas")) return hecho("abrir_archivo", "Te abrí Descargas (de prueba).");
      if (/p[aá]gina|facultad/.test(que)) return hecho("abrir_pagina", "Te abrí la página de la facultad (de prueba).");
      return hecho("abrir_programa", `Abrí ${que} (de prueba).`);
    }
    if (/paus[aá]|siguiente tema|sub[ií] el volumen|baj[aá] el volumen/.test(t)) return hecho("controlar_musica", "Listo (de prueba).");
    const temporizador = /avisame en (\d+) minutos? (?:que )?(.+)$/.exec(t);
    if (temporizador) {
      const [, minutos, que] = temporizador;
      return hecho("poner_temporizador", `Dale, en ${minutos} minutos te aviso que ${que} (de prueba: en el navegador no avisa).`);
    }
    return null;
  }

  /**
   * Un "asistente" de mentira para ver la interfaz: entiende un parcial y un
   * borrado, y para lo demás avisa que en el navegador no hay modelo.
   */
  async function turnoDePrueba(texto: string, alProgreso: (p: Progreso) => void): Promise<Turno> {
    alProgreso({ tipo: "pensando" });
    await pausa(900);
    const t = texto.toLowerCase().trim();
    const hayPendiente = pendiente !== null || archivoPendiente !== null;
    if (hayPendiente && /^(s[ií]|dale|ok)\b/.test(t)) return confirmar(true);
    if (hayPendiente && /^no\b/.test(t)) return confirmar(false);
    pendiente = null;
    archivoPendiente = null;
    const deLaCompu = await compuDePrueba(t, alProgreso);
    if (deLaCompu) return deLaCompu;

    if (t.includes("borr") && eventos.length > 0) {
      pendiente = eventos[eventos.length - 1];
      return { respuesta: "¿Seguro?", confirmacion: `¿Borro «${pendiente.titulo}»?`, cambio: null };
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
        cambio: null,
      };
    }
    return {
      respuesta: "Acá en el navegador no tengo modelo: soy una respuesta de prueba. Abrime desde Nova para hablar en serio.",
      confirmacion: null,
      cambio: null,
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
      const e: Evento = { id: siguiente++, excepciones: [], deGoogle: false, soloLectura: false, ...datos };
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
    obtenerAjustesCompu: () =>
      esperar({ ajustes: ajustesCompu, tieneKeyYoutube, carpetasPorDefecto: ajustesCompu.carpetas.slice(0, 3) }),
    guardarAjustesCompu: async (nuevos) => {
      await pausa(60);
      // Lo mismo que rechaza Rust (carpeta_prohibida), a grandes rasgos.
      const mala = nuevos.carpetas.find((c) => /^[a-z]:\\?$|\\windows|program files|appdata/i.test(c.trim()));
      if (mala) throw { tipo: "validacion", mensaje: `${mala}: Esa es una carpeta del sistema: no la puedo usar.` };
      ajustesCompu = { ...nuevos, carpetas: nuevos.carpetas.map((c) => c.trim()).filter(Boolean) };
      return { ajustes: ajustesCompu, tieneKeyYoutube, carpetasPorDefecto: [] };
    },
    guardarKeyYoutube: (key) => {
      tieneKeyYoutube = key.trim() !== "";
      return esperar(tieneKeyYoutube);
    },
    obtenerGoogle: () => esperar(google),
    guardarClienteGoogle: (id, secreto) => {
      if (!id.trim() && !secreto.trim()) {
        google = { ...google, tieneCliente: false };
        return esperar(google);
      }
      if (!id.trim().endsWith(".apps.googleusercontent.com")) {
        return Promise.reject({ tipo: "validacion", mensaje: "Ese ID no parece de Google: termina en «.apps.googleusercontent.com»." });
      }
      google = { ...google, tieneCliente: true };
      return esperar(google);
    },
    conectarGoogle: async () => {
      cancelarConexion = false;
      google = { ...google, conectando: true };
      await pausa(2500);
      google = { ...google, conectando: false };
      if (cancelarConexion) throw { tipo: "validacion", mensaje: "Se canceló la conexión con Google." };
      google = {
        ...google,
        conectado: true,
        error: null,
        sincronizadoEn: new Date().toISOString(),
        primeraVez: { traidos: 0, subidos: eventos.length, juntados: 0, borrados: 0 },
      };
      return esperar(google);
    },
    cancelarConexionGoogle: async () => {
      cancelarConexion = true;
    },
    desconectarGoogle: () => {
      google = { ...google, conectado: false, error: null, sincronizadoEn: null, primeraVez: null };
      avisarCambiosLocal(["agenda"]);
      return esperar(google);
    },
    sincronizarGoogle: async () => {
      if (google.conectado && !calendarSinRed) google = { ...google, error: null, sincronizadoEn: new Date().toISOString() };
      avisarCambiosLocal(["agenda"]);
    },
    obtenerDisposicion: () => esperar(disposicion),
    guardarDisposicion: (nueva) => {
      disposicion = nueva;
      return esperar(disposicion);
    },
    elegirCarpeta: async () => window.prompt("Carpeta (en Nova se abre el selector de Windows):", String.raw`C:\Users\vos\Facu`),
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

    obtenerAvisos: () => esperar(avisos),
    guardarAvisos: (nuevos) => {
      if (![10, 15, 30, 60].includes(nuevos.minutosAntes))
        return Promise.reject({ tipo: "validacion", mensaje: "El aviso antes de un evento puede ser de 10, 15, 30 o 60 minutos." });
      if (nuevos.minutosViaje < 5 || nuevos.minutosViaje > 240)
        return Promise.reject({ tipo: "validacion", mensaje: "El viaje tiene que durar entre 5 minutos y 4 horas." });
      if (nuevos.noMolestar && nuevos.desde === nuevos.hasta)
        return Promise.reject({ tipo: "validacion", mensaje: "\"No molestar\" tiene que empezar y terminar a horas distintas." });
      avisos = nuevos;
      return esperar(avisos);
    },
    probarAviso: () => {
      console.info("Aviso de prueba: «Así te aviso»");
      return esperar(undefined);
    },

    obtenerPerfil: () => esperar(perfil),
    guardarPerfil: (nuevo) => {
      const sobreVos = nuevo.sobreVos.trim();
      if (sobreVos.length > LARGO_SOBRE_VOS) return Promise.reject({ tipo: "validacion", mensaje: `"Sobre vos" puede tener hasta ${LARGO_SOBRE_VOS} letras.` });
      perfil = { ...nuevo, comoLlamarte: nuevo.comoLlamarte.trim().replace(/\s+/g, " "), sobreVos };
      return esperar(perfil);
    },

    estadoVoz: vozDePrueba.estado,
    listarMicrofonos: vozDePrueba.microfonos,
    guardarAjustesVoz: vozDePrueba.guardar,
    guardarKeyVoz: vozDePrueba.guardarKey,
    probarVoz: vozDePrueba.probar,
    vocesElevenlabs: vozDePrueba.vocesElevenlabs,
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
