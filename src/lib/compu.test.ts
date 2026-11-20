import { describe, expect, it } from "vitest";
import { nombreDeCarpeta } from "./compu";

describe("nombreDeCarpeta", () => {
  it("pasa al castellano las carpetas de Windows", () => {
    expect(nombreDeCarpeta(String.raw`C:\Users\vos\Downloads`)).toBe("Descargas");
    expect(nombreDeCarpeta("C:\\Users\\vos\\OneDrive\\Documents\\")).toBe("Documentos");
    expect(nombreDeCarpeta(String.raw`C:\Users\vos\Desktop`)).toBe("Escritorio");
  });

  it("deja las demás como están", () => {
    expect(nombreDeCarpeta(String.raw`D:\Facu\TP Final`)).toBe("TP Final");
    expect(nombreDeCarpeta("D:/Fotos")).toBe("Fotos");
  });
});
