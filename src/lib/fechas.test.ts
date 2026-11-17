import { describe, expect, it } from "vitest";
import {
  cuentaRegresiva,
  diasEntre,
  etiquetaDia,
  fechaLarga,
  formatearHora,
} from "./fechas";

// Martes 17 de noviembre de 2026, 10:30 hora local
const hoy = new Date(2026, 10, 17, 10, 30);

describe("diasEntre", () => {
  it("cuenta días de calendario, no bloques de 24 horas", () => {
    const tarde = new Date(2026, 10, 17, 23, 0);
    const madrugada = new Date(2026, 10, 18, 1, 0);
    expect(diasEntre(tarde, madrugada)).toBe(1);
  });

  it("es 0 para el mismo día y negativo hacia atrás", () => {
    expect(diasEntre(hoy, new Date(2026, 10, 17, 23, 59))).toBe(0);
    expect(diasEntre(hoy, new Date(2026, 10, 14))).toBe(-3);
  });

  it("cruza meses y años", () => {
    expect(diasEntre(new Date(2026, 11, 31), new Date(2027, 0, 1))).toBe(1);
  });
});

describe("etiquetaDia", () => {
  it("usa Hoy, Mañana y Ayer", () => {
    expect(etiquetaDia(new Date(2026, 10, 17, 20), hoy)).toBe("Hoy");
    expect(etiquetaDia(new Date(2026, 10, 18), hoy)).toBe("Mañana");
    expect(etiquetaDia(new Date(2026, 10, 16), hoy)).toBe("Ayer");
  });

  it("usa el día de la semana dentro de la próxima semana", () => {
    expect(etiquetaDia(new Date(2026, 10, 19), hoy)).toBe("Jueves");
    expect(etiquetaDia(new Date(2026, 10, 23), hoy)).toBe("Lunes");
  });

  it("usa día y mes más allá de una semana", () => {
    expect(etiquetaDia(new Date(2026, 10, 24), hoy)).toBe("24 nov");
    expect(etiquetaDia(new Date(2026, 11, 3), hoy)).toBe("3 dic");
  });
});

describe("cuentaRegresiva", () => {
  it("describe el futuro y el pasado", () => {
    expect(cuentaRegresiva(new Date(2026, 10, 17, 18), hoy)).toBe("Es hoy");
    expect(cuentaRegresiva(new Date(2026, 10, 18), hoy)).toBe("Es mañana");
    expect(cuentaRegresiva(new Date(2026, 10, 20), hoy)).toBe("Faltan 3 días");
    expect(cuentaRegresiva(new Date(2026, 10, 16), hoy)).toBe("Fue ayer");
    expect(cuentaRegresiva(new Date(2026, 10, 12), hoy)).toBe("Pasó hace 5 días");
  });
});

describe("formatos", () => {
  it("formatea la hora con dos dígitos", () => {
    expect(formatearHora(new Date(2026, 10, 17, 9, 5))).toBe("09:05");
  });

  it("arma la fecha larga", () => {
    expect(fechaLarga(hoy)).toBe("Martes 17 de noviembre");
  });
});
