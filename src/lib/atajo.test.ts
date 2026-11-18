import { describe, expect, it } from "vitest";
import { atajoDesdeTecla, nombreAtajo } from "./atajo";

const tecla = (code: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  code,
  ...mods,
});

describe("atajoDesdeTecla", () => {
  it("arma modificadores + código", () => {
    expect(atajoDesdeTecla(tecla("Space", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+Space");
    expect(atajoDesdeTecla(tecla("KeyN", { altKey: true }))).toBe("Alt+KeyN");
  });

  it("espera mientras solo hay modificadores", () => {
    expect(atajoDesdeTecla(tecla("ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(atajoDesdeTecla(tecla("ShiftRight", { ctrlKey: true, shiftKey: true }))).toBeNull();
  });

  it("no acepta una tecla sola", () => {
    expect(atajoDesdeTecla(tecla("KeyN"))).toBeNull();
  });
});

describe("nombreAtajo", () => {
  it("lo muestra legible", () => {
    expect(nombreAtajo("Ctrl+Shift+Space")).toBe("Ctrl + Shift + Espacio");
    expect(nombreAtajo("Alt+KeyN")).toBe("Alt + N");
    expect(nombreAtajo("Ctrl+Alt+Digit1")).toBe("Ctrl + Alt + 1");
    expect(nombreAtajo("Ctrl+F5")).toBe("Ctrl + F5");
  });
});
