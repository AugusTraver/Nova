import { describe, expect, it } from "vitest";
import {
  cambiarTipo,
  datosDesdeFormEvento,
  datosDesdeFormResponsabilidad,
  formDesdeEvento,
  formEventoNuevo,
  formResponsabilidadNueva,
} from "./formularios";
import { aIsoLocal } from "./fechas";
import type { Evento, Responsabilidad } from "./tipos";

const hoy = new Date(2026, 10, 17, 10, 30);

describe("aIsoLocal", () => {
  it("arma un ISO con la zona del sistema que representa esa hora local", () => {
    const iso = aIsoLocal("2026-11-20", "09:05");
    expect(iso).toMatch(/^2026-11-20T09:05:00[+-]\d{2}:\d{2}$/);
    const d = new Date(iso);
    expect([d.getFullYear(), d.getMonth(), d.getDate(), d.getHours(), d.getMinutes()]).toEqual([2026, 10, 20, 9, 5]);
  });
});

describe("formulario de eventos", () => {
  it("un evento nuevo arranca hoy, como clase semanal", () => {
    const f = formEventoNuevo(hoy);
    expect(f.fecha).toBe("2026-11-17");
    expect(f.tipo).toBe("clase");
    expect(f.recurrencia).toBe("semanal");
  });

  it("al elegir cumpleaños en uno nuevo, propone todo el día y anual", () => {
    const f = cambiarTipo(formEventoNuevo(hoy), "cumpleanos", true);
    expect(f.todoElDia).toBe(true);
    expect(f.recurrencia).toBe("anual");
    const examen = cambiarTipo(f, "examen", true);
    expect(examen.recurrencia).toBe("ninguna");
  });

  it("en uno existente, cambiar el tipo no toca lo demás", () => {
    const f = { ...formEventoNuevo(hoy), recurrencia: "ninguna" as const };
    expect(cambiarTipo(f, "cumpleanos", false)).toEqual({ ...f, tipo: "cumpleanos" });
  });

  it("convierte fecha y horas a ISO con zona", () => {
    const f = { ...formEventoNuevo(hoy), titulo: "  Análisis  ", horaInicio: "15:00", horaFin: "17:00" };
    const r = datosDesdeFormEvento(f);
    if (!r.ok) throw new Error(r.error);
    expect(r.datos.titulo).toBe("Análisis");
    expect(r.datos.inicio).toBe(aIsoLocal("2026-11-17", "15:00"));
    expect(r.datos.fin).toBe(aIsoLocal("2026-11-17", "17:00"));
  });

  it("todo el día va con fecha sola y sin fin", () => {
    const f = { ...formEventoNuevo(hoy), titulo: "Feriado", todoElDia: true, horaFin: "18:00", tipo: "otro" as const };
    const r = datosDesdeFormEvento(f);
    if (!r.ok) throw new Error(r.error);
    expect(r.datos.inicio).toBe("2026-11-17");
    expect(r.datos.fin).toBeNull();
  });

  it("todo el día puede durar varios días, con el último incluido", () => {
    const f = { ...formEventoNuevo(hoy), titulo: "Semana de parciales", tipo: "otro" as const, todoElDia: true, fechaFin: "2026-11-21" };
    const r = datosDesdeFormEvento(f);
    if (!r.ok) throw new Error(r.error);
    expect([r.datos.inicio, r.datos.fin]).toEqual(["2026-11-17", "2026-11-21"]);
    expect(datosDesdeFormEvento({ ...f, fechaFin: "2026-11-16" }).ok).toBe(false);
    // Mismo día que el inicio = un solo día, sin fin
    const mismo = datosDesdeFormEvento({ ...f, fechaFin: "2026-11-17" });
    expect(mismo.ok && mismo.datos.fin).toBeNull();
  });

  it("editar un evento de varios días conserva su fin", () => {
    const evento: Evento = {
      id: 9, titulo: "Viaje", tipo: "otro", inicio: "2026-12-01", fin: "2026-12-05", todoElDia: true,
      materiaId: null, persona: null, notas: null, recurrencia: "ninguna", repetirHasta: null, excepciones: [],
      deGoogle: false, soloLectura: false,
    };
    const r = datosDesdeFormEvento(formDesdeEvento(evento));
    expect(r.ok && r.datos.fin).toBe("2026-12-05");
  });

  it("limpia lo que no corresponde al tipo", () => {
    const f = { ...formEventoNuevo(hoy), titulo: "Cumple", tipo: "cumpleanos" as const, materiaId: 3, persona: " Juan " };
    const r = datosDesdeFormEvento(f);
    if (!r.ok) throw new Error(r.error);
    expect(r.datos.materiaId).toBeNull();
    expect(r.datos.persona).toBe("Juan");

    const clase = datosDesdeFormEvento({ ...formEventoNuevo(hoy), titulo: "Clase", persona: "Juan", materiaId: 3 });
    if (!clase.ok) throw new Error(clase.error);
    expect(clase.datos.persona).toBeNull();
    expect(clase.datos.materiaId).toBe(3);
  });

  it("avisa los errores típicos", () => {
    const base = formEventoNuevo(hoy);
    expect(datosDesdeFormEvento(base)).toEqual({ ok: false, error: "Ponele un título." });
    expect(datosDesdeFormEvento({ ...base, titulo: "x", horaInicio: "10:00", horaFin: "09:00" }).ok).toBe(false);
    expect(datosDesdeFormEvento({ ...base, titulo: "x", repetirHasta: "2026-11-01" }).ok).toBe(false);
  });

  it("ida y vuelta desde un evento guardado", () => {
    const inicio = aIsoLocal("2026-11-18", "15:00");
    const evento: Evento = {
      id: 1,
      titulo: "Física",
      tipo: "clase",
      inicio,
      fin: aIsoLocal("2026-11-18", "17:00"),
      todoElDia: false,
      materiaId: 2,
      persona: null,
      notas: null,
      recurrencia: "semanal",
      repetirHasta: "2026-12-05",
      excepciones: [],
      deGoogle: false,
      soloLectura: false,
    };
    const f = formDesdeEvento(evento);
    expect([f.fecha, f.horaInicio, f.horaFin, f.repetirHasta]).toEqual(["2026-11-18", "15:00", "17:00", "2026-12-05"]);
    const r = datosDesdeFormEvento(f);
    if (!r.ok) throw new Error(r.error);
    expect(r.datos.inicio).toBe(inicio);
  });
});

describe("formulario de responsabilidades", () => {
  it("valida el título y conserva el vínculo al editar", () => {
    expect(datosDesdeFormResponsabilidad(formResponsabilidadNueva()).ok).toBe(false);
    const anterior = { eventoId: 7, descripcion: "detalle" } as Responsabilidad;
    const r = datosDesdeFormResponsabilidad({ ...formResponsabilidadNueva(), titulo: " Estudiar " }, anterior);
    if (!r.ok) throw new Error(r.error);
    expect(r.datos).toEqual({
      titulo: "Estudiar",
      descripcion: "detalle",
      fechaLimite: null,
      prioridad: "media",
      materiaId: null,
      eventoId: 7,
    });
  });
});
