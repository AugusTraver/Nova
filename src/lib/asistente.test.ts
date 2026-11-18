import { describe, expect, it } from "vitest";
import { textoDelPaso } from "./asistente";

describe("textoDelPaso", () => {
  it("dice qué está haciendo según la herramienta", () => {
    expect(textoDelPaso({ tipo: "herramienta", nombre: "crear_evento" })).toBe("Anotando el evento…");
    expect(textoDelPaso({ tipo: "herramienta", nombre: "listar_eventos" })).toBe("Mirando tu agenda…");
  });

  it("vuelve a pensar entre vueltas o con herramientas desconocidas", () => {
    expect(textoDelPaso({ tipo: "pensando" })).toBe("Pensando…");
    expect(textoDelPaso({ tipo: "herramienta", nombre: "volar" })).toBe("Pensando…");
  });
});
