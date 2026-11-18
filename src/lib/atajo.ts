// Atajos de teclado: de la tecla apretada al texto que guarda Rust, y de ese
// texto a algo legible ("Ctrl + Shift + Espacio").

interface Tecla {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  code: string;
}

const SOLO_MODIFICADORES = /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/;

/**
 * La combinación apretada → "Ctrl+Shift+KeyN". `null` mientras solo haya
 * modificadores, o si no hay ninguno (un atajo global de una sola tecla
 * taparía esa tecla en todo Windows).
 */
export function atajoDesdeTecla(e: Tecla): string | null {
  if (SOLO_MODIFICADORES.test(e.code) || !e.code) return null;
  const partes = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift"].filter(Boolean) as string[];
  if (partes.length === 0) return null;
  return [...partes, e.code].join("+");
}

const NOMBRES: Record<string, string> = {
  Space: "Espacio",
  Enter: "Enter",
  Backquote: "º",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Minus: "-",
  Equal: "=",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

/** "Ctrl+Shift+KeyN" → "Ctrl + Shift + N". */
export function nombreAtajo(atajo: string): string {
  return atajo
    .split("+")
    .map((parte) => {
      if (NOMBRES[parte]) return NOMBRES[parte];
      if (/^Key[A-Z]$/.test(parte)) return parte.slice(3);
      if (/^Digit\d$/.test(parte)) return parte.slice(5);
      if (/^Numpad\d$/.test(parte)) return `${parte.slice(6)} (numérico)`;
      return parte;
    })
    .join(" + ");
}
