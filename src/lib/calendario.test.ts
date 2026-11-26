import { describe, expect, it } from "vitest";
import { agruparPorDia } from "./calendario";
import type { Ocurrencia } from "./tipos";

const hoy = new Date(2026, 10, 17, 10, 30);

function oc(titulo: string, fecha: string, fin: string | null = null): Ocurrencia {
  return {
    evento: {
      id: 1,
      titulo,
      tipo: "otro",
      inicio: fecha,
      fin,
      todoElDia: true,
      materiaId: null,
      persona: null,
      notas: null,
      recurrencia: "ninguna",
      repetirHasta: null,
      excepciones: [],
      deGoogle: false,
      soloLectura: false,
    },
    fecha,
    inicio: fecha,
    fin,
  };
}

describe("agruparPorDia", () => {
  it("agrupa los días con eventos dentro del rango", () => {
    const grupos = agruparPorDia([oc("a", "2026-11-17"), oc("b", "2026-11-17"), oc("c", "2026-11-19"), oc("lejos", "2026-11-30")], hoy, 4);
    expect(grupos.map((g) => [g.fecha, g.ocurrencias.map((o) => o.evento.titulo)])).toEqual([
      ["2026-11-17", ["a", "b"]],
      ["2026-11-19", ["c"]],
    ]);
  });

  it("un evento de varios días aparece en cada día, recortado al rango", () => {
    const grupos = agruparPorDia([oc("semana de parciales", "2026-11-15", "2026-11-18")], hoy, 4);
    expect(grupos.map((g) => g.fecha)).toEqual(["2026-11-17", "2026-11-18"]);
  });
});
