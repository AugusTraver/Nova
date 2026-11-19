import { describe, expect, it } from "vitest";
import { esferaSegun, pistaDeVoz, textoDescarga, vozQueHabla } from "./voz";

describe("voz", () => {
  it("la esfera sigue a la voz salvo que el asistente esté ocupado", () => {
    expect(esferaSegun("reposo", "escuchando")).toBe("escuchando");
    expect(esferaSegun("reposo", "oyendo")).toBe("escuchando");
    expect(esferaSegun("reposo", "hablando")).toBe("hablando");
    expect(esferaSegun("reposo", "transcribiendo")).toBe("pensando");
    expect(esferaSegun("reposo", "apagada")).toBe("reposo");
    expect(esferaSegun("pensando", "escuchando")).toBe("pensando");
    expect(esferaSegun("error", "hablando")).toBe("error");
  });

  it("pistas", () => {
    expect(pistaDeVoz("escuchando")).toBe("Te escucho");
    expect(pistaDeVoz("hablando")).toBeNull();
    expect(pistaDeVoz("sin_modelos")).toContain("Ajustes");
  });

  it("progreso de la descarga", () => {
    expect(textoDescarga(312, 12000)).toBe("312 de 12.000 MB");
  });

  it("habla con Azure solo si hay región y key", () => {
    expect(vozQueHabla("eastus", true)).toBe("azure");
    expect(vozQueHabla("", true)).toBe("local");
    expect(vozQueHabla("  ", true)).toBe("local");
    expect(vozQueHabla("eastus", false)).toBe("local");
  });
});
