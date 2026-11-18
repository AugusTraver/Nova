import { describe, expect, it } from "vitest";
import { antiguedad, estadoExterno } from "./externos";

const ahora = new Date("2026-11-17T14:00:00-03:00");

describe("antiguedad", () => {
  it("dice cuán viejo es un dato", () => {
    expect(antiguedad("2026-11-17T13:59:40-03:00", ahora)).toBe("de recién");
    expect(antiguedad("2026-11-17T13:55:00-03:00", ahora)).toBe("de hace 5 min");
    expect(antiguedad("2026-11-17T11:00:00-03:00", ahora)).toBe("de hace 3 h");
    expect(antiguedad("2026-11-16T12:00:00-03:00", ahora)).toBe("de ayer");
    expect(antiguedad("2026-11-13T14:00:00-03:00", ahora)).toBe("de hace 4 días");
  });
});

describe("estadoExterno", () => {
  it("distingue cargando, sin datos y datos viejos", () => {
    expect(estadoExterno(null, ahora)).toEqual({ tipo: "cargando" });
    expect(estadoExterno({ datos: null, obtenidoEn: null, error: null }, ahora)).toEqual({ tipo: "cargando" });
    expect(estadoExterno({ datos: null, obtenidoEn: null, error: "Sin conexión con ESPN." }, ahora)).toEqual({
      tipo: "sin_datos",
      mensaje: "Sin conexión con ESPN.",
    });
    expect(estadoExterno({ datos: 1, obtenidoEn: "2026-11-17T13:00:00-03:00", error: null }, ahora)).toEqual({
      tipo: "listo",
      aviso: null,
    });
    expect(
      estadoExterno({ datos: 1, obtenidoEn: "2026-11-17T11:00:00-03:00", error: "Sin conexión con ESPN." }, ahora),
    ).toEqual({ tipo: "listo", aviso: "Sin conexión con ESPN · datos de hace 3 h" });
  });
});
