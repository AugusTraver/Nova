import { describe, expect, it } from "vitest";
import { avisoCalendar, avisoCortoCalendar, estadoCalendar, textoPrimeraVez } from "./google";
import type { VistaGoogle } from "./tipos";

const ahora = new Date("2026-11-17T14:00:00-03:00");

const conectado: VistaGoogle = {
  tieneCliente: true,
  conectado: true,
  conectando: false,
  error: null,
  sincronizadoEn: "2026-11-17T13:58:00-03:00",
  primeraVez: null,
};

describe("Google Calendar", () => {
  it("el aviso del widget solo aparece si falla", () => {
    expect(avisoCalendar(conectado, ahora)).toBeNull();
    expect(avisoCalendar(null, ahora)).toBeNull();
    expect(avisoCalendar({ ...conectado, conectado: false, error: "x" }, ahora)).toBeNull();
    expect(
      avisoCalendar({ ...conectado, error: "Sin conexión con Google Calendar", sincronizadoEn: "2026-11-17T11:00:00-03:00" }, ahora),
    ).toBe("Sin conexión con Google Calendar · última vez hace 3 h");
    expect(avisoCalendar({ ...conectado, error: "Volvé a conectar.", sincronizadoEn: null }, ahora)).toBe("Volvé a conectar");
  });

  it("la versión corta para el título", () => {
    expect(avisoCortoCalendar(conectado)).toBeNull();
    expect(avisoCortoCalendar({ ...conectado, error: "Sin conexión con Google Calendar" })).toBe("Sin conexión con Google");
    expect(avisoCortoCalendar({ ...conectado, error: "Google ya no acepta el permiso de Nova" })).toBe("Revisá Google Calendar en Ajustes");
  });

  it("el estado en Ajustes", () => {
    expect(estadoCalendar(conectado, ahora)).toBe("Al día · hace 2 min");
    expect(estadoCalendar({ ...conectado, conectando: true }, ahora)).toBe("Esperando que aceptes en el navegador…");
    expect(estadoCalendar({ ...conectado, conectado: false }, ahora)).toBe("Sin conectar");
    expect(estadoCalendar({ ...conectado, conectado: false, tieneCliente: false }, ahora)).toBe("Falta el ID y el secreto del cliente");
  });

  it("cuenta lo que hizo la primera vez", () => {
    expect(textoPrimeraVez({ subidos: 5, traidos: 2, juntados: 1, borrados: 0 })).toBe(
      "La primera vez subí 5 eventos, traje 2 y junté 1 que estaba en los dos lados.",
    );
    expect(textoPrimeraVez({ subidos: 1, traidos: 0, juntados: 0, borrados: 0 })).toBe("La primera vez subí 1 evento.");
    expect(textoPrimeraVez({ subidos: 0, traidos: 0, juntados: 0, borrados: 0 })).toBe("La primera vez no había nada para pasar.");
  });
});
