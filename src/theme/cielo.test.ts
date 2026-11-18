import { describe, expect, it } from "vitest";
import { esMomento, momentoDelDia, saludo } from "./cielo";

const a = (hora: number, minuto = 0) => new Date(2026, 10, 17, hora, minuto);

describe("momentoDelDia", () => {
  it("cubre los límites de cada momento", () => {
    expect(momentoDelDia(a(0))).toBe("madrugada");
    expect(momentoDelDia(a(5, 59))).toBe("madrugada");
    expect(momentoDelDia(a(6))).toBe("manana");
    expect(momentoDelDia(a(10, 59))).toBe("manana");
    expect(momentoDelDia(a(11))).toBe("mediodia");
    expect(momentoDelDia(a(16))).toBe("tarde");
    expect(momentoDelDia(a(19))).toBe("anochecer");
    expect(momentoDelDia(a(21))).toBe("noche");
    expect(momentoDelDia(a(23, 59))).toBe("noche");
  });
});

describe("esMomento", () => {
  it("acepta solo nombres válidos", () => {
    expect(esMomento("tarde")).toBe(true);
    expect(esMomento("siesta")).toBe(false);
    expect(esMomento(null)).toBe(false);
  });
});

describe("saludo", () => {
  it("cambia a la 1 de la tarde y a las 8 de la noche", () => {
    expect(saludo(a(12, 59))).toBe("Buen día");
    expect(saludo(a(13))).toBe("Buenas tardes");
    expect(saludo(a(20))).toBe("Buenas noches");
    expect(saludo(a(3))).toBe("Buenas noches");
  });
});
