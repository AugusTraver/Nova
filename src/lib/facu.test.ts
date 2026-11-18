import { describe, expect, it } from "vitest";
import { progresoPorMateria, proximasEvaluaciones } from "./facu";
import type { Evento, Materia, Ocurrencia, Responsabilidad, TipoEvento } from "./tipos";

const hoy = new Date(2026, 10, 17, 10, 30);

const materias: Materia[] = [
  { id: 1, nombre: "Análisis", color: "celeste" },
  { id: 2, nombre: "Física", color: "coral" },
];

function resp(id: number, materiaId: number | null, estado: "pendiente" | "hecha"): Responsabilidad {
  return {
    id,
    titulo: `r${id}`,
    descripcion: null,
    fechaLimite: null,
    prioridad: "media",
    estado,
    hechaEn: null,
    eventoId: null,
    materiaId,
    origen: "manual",
    creadoEn: "",
  };
}

function oc(titulo: string, tipo: TipoEvento, fecha: string, hora = "09:00"): Ocurrencia {
  const inicio = `${fecha}T${hora}:00-03:00`;
  const evento: Evento = {
    id: Math.random(),
    titulo,
    tipo,
    inicio,
    fin: null,
    todoElDia: false,
    materiaId: null,
    persona: null,
    notas: null,
    recurrencia: "ninguna",
    repetirHasta: null,
    excepciones: [],
  };
  return { evento, fecha, inicio, fin: null };
}

describe("progresoPorMateria", () => {
  it("cuenta hechas sobre el total de cada materia", () => {
    const lista = [resp(1, 1, "hecha"), resp(2, 1, "pendiente"), resp(3, 1, "hecha"), resp(4, null, "hecha"), resp(5, 9, "hecha")];
    const p = progresoPorMateria(materias, lista);
    expect(p.get(1)).toEqual({ hechas: 2, total: 3 });
    expect(p.get(2)).toEqual({ hechas: 0, total: 0 });
    expect(p.has(9)).toBe(false);
  });
});

describe("proximasEvaluaciones", () => {
  it("solo exámenes y entregas desde hoy, en orden y con tope", () => {
    const lista = [
      oc("Final", "examen", "2026-12-01"),
      oc("Clase", "clase", "2026-11-18"),
      oc("Parcial viejo", "examen", "2026-11-10"),
      oc("TP", "entrega", "2026-11-17", "23:59"),
      oc("Parcial", "examen", "2026-11-19"),
    ];
    expect(proximasEvaluaciones(lista, hoy, 2).map((o) => o.evento.titulo)).toEqual(["TP", "Parcial"]);
  });
});
