import { describe, expect, it } from "vitest";
import { competenciaConFase, goleadores, lineaJugador, nombreCorto, textoPosicion, textoResultado } from "./futbol";
import type { EquipoBreve, EstadoJugador, Partido, ResumenEquipo } from "./tipos";

const boca: EquipoBreve = { id: "5", nombre: "Boca Juniors", abreviatura: "CABJ", escudo: null };
const banfield: EquipoBreve = { id: "235", nombre: "Banfield", abreviatura: "BAN", escudo: null };

function partido(extra: Partial<Partido> = {}): Partido {
  return {
    id: "1",
    liga: "arg.copa",
    competencia: "Copa Argentina",
    fase: null,
    fecha: "2026-11-14T22:00:00Z",
    local: banfield,
    visitante: boca,
    estado: "terminado",
    minuto: null,
    golesLocal: 0,
    golesVisitante: 3,
    penalesLocal: null,
    penalesVisitante: null,
    eventos: [],
    ...extra,
  };
}

describe("textos de fútbol", () => {
  it("nombres y competencias", () => {
    expect(nombreCorto(boca)).toBe("Boca");
    expect(nombreCorto(banfield)).toBe("Banfield");
    expect(competenciaConFase(partido())).toBe("Copa Argentina");
    expect(competenciaConFase(partido({ competencia: "Sudamericana", fase: "final" }))).toBe("Final de la Sudamericana");
  });

  it("el resultado desde el lado de Boca", () => {
    expect(textoResultado(partido(), "5")).toBe("Le ganó 3 a 0 a Banfield");
    expect(textoResultado(partido({ golesLocal: 2, golesVisitante: 1 }), "5")).toBe("Perdió 1 a 2 con Banfield");
    expect(textoResultado(partido({ golesLocal: 1, golesVisitante: 1, penalesLocal: 3, penalesVisitante: 4 }), "5")).toBe(
      "Empató 1 a 1 con Banfield y ganó por penales",
    );
  });

  it("la posición en la tabla", () => {
    const r: ResumenEquipo = {
      clave: "boca",
      id: "5",
      nombre: "Boca",
      proximo: null,
      ultimo: null,
      enVivo: null,
      posicion: {
        torneo: "Liga Profesional",
        zona: "Zona A",
        equiposEnZona: 15,
        fila: { puesto: 2, equipo: boca, puntos: 33, jugados: 16, ganados: 9, empatados: 6, perdidos: 1, diferencia: 10 },
      },
    };
    expect(textoPosicion(r)).toBe("2.º en la Zona A · 33 pts");
    expect(textoPosicion({ ...r, posicion: null })).toBeUndefined();
  });

  it("goleadores del partido en vivo", () => {
    const p = partido({
      eventos: [
        { minuto: "45'+1'", tipo: "gol", equipoId: "5", jugador: "Ascacibar" },
        { minuto: "50'", tipo: "expulsion", equipoId: "5", jugador: "X" },
        { minuto: "63'", tipo: "gol_en_contra", equipoId: "5", jugador: "Meriano" },
      ],
    });
    expect(goleadores(p, "5")).toEqual(["Ascacibar 45'+1'", "Meriano (en contra) 63'"]);
    expect(goleadores(p, "235")).toEqual([]);
  });

  it("lo que hizo cada jugador", () => {
    const ahora = new Date("2026-11-17T14:00:00-03:00");
    const j = (ultimo: EstadoJugador["ultimo"]): EstadoJugador => ({
      id: "1", nombre: "Messi", club: "Inter Miami", ultimo, partidos: 30, goles: 20, asistencias: 10,
    });
    const u = { fecha: "2026-11-16T22:00:00Z", rival: "X", resultado: "2-1", desenlace: "G" as const, competencia: null, goles: 1, asistencias: 2 };
    expect(lineaJugador(j(u), ahora)).toBe("1 gol y 2 asistencias, ayer");
    expect(lineaJugador(j({ ...u, goles: 0, asistencias: 0, fecha: "2026-11-14T22:00:00Z" }), ahora)).toBe("Jugó hace 3 días");
    expect(lineaJugador(j({ ...u, fecha: "2026-10-20T22:00:00Z" }), ahora)).toBe("No juega desde el 20 oct");
    expect(lineaJugador(j(null), ahora)).toBe("Sin partidos");
  });
});
