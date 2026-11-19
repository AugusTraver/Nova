import { describe, expect, it } from "vitest";
import type { AjustesVoz } from "./tipos";
import { esferaSegun, pistaDeVoz, proveedorListo, textoDemora, textoDescarga, vozQueHabla } from "./voz";

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

  it("habla con la voz elegida solo si está todo cargado", () => {
    const base: AjustesVoz = {
      activada: true,
      velocidad: 1,
      leerResumen: true,
      usarGpu: true,
      palabraClave: true,
      palabraPausaBateria: true,
      regionAzure: "eastus",
      vozAzure: "es-AR-ElenaNeural",
      proveedor: "azure",
      vozEleven: "",
      modeloEleven: "eleven_flash_v2_5",
      vozOpenai: "coral",
    };
    expect(vozQueHabla(base, ["azure"])).toBe("azure");
    expect(vozQueHabla({ ...base, regionAzure: "  " }, ["azure"])).toBe("local");
    expect(vozQueHabla(base, [])).toBe("local");
    expect(vozQueHabla({ ...base, proveedor: "openai" }, ["openai"])).toBe("openai");
    // ElevenLabs necesita además una voz elegida.
    expect(vozQueHabla({ ...base, proveedor: "elevenlabs" }, ["elevenlabs"])).toBe("local");
    expect(vozQueHabla({ ...base, proveedor: "elevenlabs", vozEleven: "abc" }, ["elevenlabs"])).toBe("elevenlabs");
    expect(proveedorListo("local", base, [])).toBe(true);
  });

  it("demora", () => {
    expect(textoDemora(412.4)).toBe("Tardó 412 ms en empezar");
    expect(textoDemora(1530)).toBe("Tardó 1,5 s en empezar");
  });
});
