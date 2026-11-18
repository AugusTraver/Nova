import { describe, expect, it } from "vitest";
import { climaCorto, futbolDeHoy } from "./resumen";
import type { EquipoBreve, Partido, ResumenEquipo, VistaFutbol } from "./tipos";

const boca: EquipoBreve = { id: "5", nombre: "Boca Juniors", abreviatura: "BOC", escudo: null };
const racing: EquipoBreve = { id: "15", nombre: "Racing Club", abreviatura: "RAC", escudo: null };
const ahora = new Date("2026-11-17T14:00:00-03:00");

function partido(extra: Partial<Partido>): Partido {
  return {
    id: "1", liga: "arg.1", competencia: "Liga Profesional", fase: null, fecha: "2026-11-18T00:30:00Z",
    local: boca, visitante: racing, estado: "programado", minuto: null, golesLocal: null, golesVisitante: null,
    penalesLocal: null, penalesVisitante: null, eventos: [], ...extra,
  };
}

function vista(equipo: Partial<ResumenEquipo>): VistaFutbol {
  return {
    equipos: [{ clave: "boca", id: "5", nombre: "Boca", proximo: null, ultimo: null, enVivo: null, posicion: null, ...equipo }],
    jugadores: [],
  };
}

describe("resumen del día", () => {
  it("dice si juega Boca hoy", () => {
    expect(futbolDeHoy(vista({ proximo: partido({}) }), ahora)).toBe("A las 21:30 juega Boca con Racing Club.");
    expect(futbolDeHoy(vista({ proximo: partido({ fecha: "2026-11-19T00:30:00Z" }) }), ahora)).toBeNull();
    expect(futbolDeHoy(null, ahora)).toBeNull();
  });

  it("cuenta el partido en vivo", () => {
    const enVivo = partido({ estado: "en_juego", golesLocal: 2, golesVisitante: 1 });
    expect(futbolDeHoy(vista({ enVivo }), ahora)).toBe("Boca está jugando: va 2 a 1 con Racing Club.");
  });

  it("clima corto", () => {
    expect(
      climaCorto({ ciudad: "BA", temperatura: 22.6, sensacion: null, codigo: 1, descripcion: "casi despejado", esDeDia: true, maxima: null, minima: null, lluvia: null }),
    ).toBe("23°, casi despejado");
  });
});
