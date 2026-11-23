import { describe, expect, it } from "vitest";
import { acomodar, aFraccion, aPixeles, type Caja } from "./disposicion";

const panel: Caja = { x: 0, y: 0, ancho: 1600, alto: 900 };
const esfera: Caja = { x: 600, y: 250, ancho: 400, alto: 400 };
const widget = (x: number, y: number): Caja => ({ x, y, ancho: 300, alto: 200 });

describe("acomodar", () => {
  it("si no toca nada, queda donde se soltó", () => {
    expect(acomodar(widget(100, 100), [esfera], panel)).toEqual(widget(100, 100));
  });

  it("no se sale del panel", () => {
    expect(acomodar(widget(-80, 850), [], panel)).toEqual(widget(16, 900 - 16 - 200));
    expect(acomodar(widget(1500, -40), [], panel)).toEqual(widget(1600 - 16 - 300, 16));
  });

  it("si tapa la esfera, sale por el lado más cercano", () => {
    // Apenas metido por la izquierda: vuelve a la izquierda.
    expect(acomodar(widget(400, 300), [esfera], panel)).toEqual(widget(600 - 16 - 300, 300));
    // Apenas metido por abajo: baja.
    expect(acomodar(widget(650, 600), [esfera], panel)).toEqual(widget(650, 650 + 16));
  });

  it("no pisa a otro widget", () => {
    const otro = widget(100, 100);
    const final = acomodar(widget(150, 150), [otro, esfera], panel);
    expect(final.y).toBe(100 + 200 + 16);
    expect(final.x).toBe(150);
  });

  it("si al salir de uno cae sobre otro, busca un lado libre", () => {
    const arriba = widget(100, 16);
    const abajo = widget(100, 330);
    // Entre los dos no entra: sale hacia la derecha.
    const final = acomodar(widget(100, 200), [arriba, abajo], panel);
    expect(final.x).toBe(100 + 300 + 16);
  });
});

describe("fracciones", () => {
  it("ida y vuelta", () => {
    const tam = { ancho: 1600, alto: 900 };
    expect(aFraccion({ x: 160, y: -90 }, tam)).toEqual({ x: 0.1, y: -0.1 });
    expect(aPixeles({ x: 0.1, y: -0.1 }, tam)).toEqual({ x: 160, y: -90 });
    expect(aPixeles(undefined, tam)).toEqual({ x: 0, y: 0 });
  });
});
