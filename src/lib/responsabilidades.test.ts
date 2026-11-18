import { describe, expect, it } from "vitest";
import { grupoDe, hechas, ordenar, visibles } from "./responsabilidades";
import type { Prioridad, Responsabilidad } from "./tipos";

const hoy = new Date(2026, 10, 17, 10, 30); // martes 17/11/2026

let siguiente = 1;
function r(titulo: string, fechaLimite: string | null, prioridad: Prioridad = "media", extra: Partial<Responsabilidad> = {}): Responsabilidad {
  return {
    id: siguiente++,
    titulo,
    descripcion: null,
    fechaLimite,
    prioridad,
    estado: "pendiente",
    hechaEn: null,
    eventoId: null,
    materiaId: null,
    origen: "manual",
    creadoEn: "2026-11-01T10:00:00-03:00",
    ...extra,
  };
}

describe("grupoDe", () => {
  it("clasifica por fecha límite", () => {
    expect(grupoDe(r("a", "2026-11-16"), hoy)).toBe("vencidas");
    expect(grupoDe(r("b", "2026-11-17"), hoy)).toBe("hoy");
    expect(grupoDe(r("c", "2026-11-18"), hoy)).toBe("proximas");
    expect(grupoDe(r("d", null), hoy)).toBe("sinFecha");
  });
});

describe("ordenar", () => {
  it("vencidas, hoy, próximas y sin fecha", () => {
    const lista = [
      r("sin fecha", null),
      r("próxima lejana", "2026-12-01"),
      r("hoy", "2026-11-17"),
      r("vencida", "2026-11-10"),
      r("próxima cercana", "2026-11-19"),
    ];
    expect(ordenar(lista, hoy).map((x) => x.titulo)).toEqual([
      "vencida",
      "hoy",
      "próxima cercana",
      "próxima lejana",
      "sin fecha",
    ]);
  });

  it("a igual fecha, primero la de mayor prioridad y después la más vieja", () => {
    const vieja = r("media vieja", "2026-11-17", "media");
    const lista = [r("baja", "2026-11-17", "baja"), r("alta", "2026-11-17", "alta"), vieja, r("media nueva", "2026-11-17", "media")];
    expect(ordenar(lista, hoy).map((x) => x.titulo)).toEqual(["alta", "media vieja", "media nueva", "baja"]);
  });

  it("no modifica la lista original", () => {
    const lista = [r("b", null), r("a", "2026-11-10")];
    ordenar(lista, hoy);
    expect(lista[0].titulo).toBe("b");
  });
});

describe("visibles", () => {
  it("muestra las pendientes y las recién hechas", () => {
    const hecha = r("recién hecha", "2026-11-17", "media", { estado: "hecha" });
    const vieja = r("hecha hace rato", "2026-11-17", "media", { estado: "hecha" });
    const pendiente = r("pendiente", null);
    const lista = visibles([hecha, vieja, pendiente], new Set([hecha.id]), hoy);
    expect(lista.map((x) => x.titulo)).toEqual(["recién hecha", "pendiente"]);
  });
});

describe("hechas", () => {
  it("solo las hechas, la más reciente primero", () => {
    const vieja = r("vieja", null, "media", { estado: "hecha", hechaEn: "2026-11-10T10:00:00-03:00" });
    const nueva = r("nueva", null, "media", { estado: "hecha", hechaEn: "2026-11-17T09:00:00-03:00" });
    const pendiente = r("pendiente", null);
    expect(hechas([vieja, pendiente, nueva]).map((x) => x.titulo)).toEqual(["nueva", "vieja"]);
  });
});
