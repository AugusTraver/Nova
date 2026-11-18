// Utilidades de fechas para mostrar en la interfaz.
// Trabajan siempre en la zona horaria local del sistema.

const DIAS = [
  "domingo",
  "lunes",
  "martes",
  "miércoles",
  "jueves",
  "viernes",
  "sábado",
] as const;

const MESES_CORTOS = [
  "ene",
  "feb",
  "mar",
  "abr",
  "may",
  "jun",
  "jul",
  "ago",
  "sep",
  "oct",
  "nov",
  "dic",
] as const;

/** Medianoche local del día de `fecha`. */
export function inicioDelDia(fecha: Date): Date {
  return new Date(fecha.getFullYear(), fecha.getMonth(), fecha.getDate());
}

/**
 * Días de calendario entre `desde` y `hasta` (negativo si `hasta` es anterior).
 * Cuenta cambios de fecha, no bloques de 24 h: de las 23:00 a la 01:00 del día
 * siguiente es 1 día. Usa Math.round para absorber los cambios de horario de verano.
 */
export function diasEntre(desde: Date, hasta: Date): number {
  const ms = inicioDelDia(hasta).getTime() - inicioDelDia(desde).getTime();
  return Math.round(ms / 86_400_000);
}

function capitalizar(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}

/** "Hoy", "Mañana", "Ayer", el día de la semana si cae en los próximos 6 días, o "12 dic". */
export function etiquetaDia(fecha: Date, hoy: Date): string {
  const dias = diasEntre(hoy, fecha);
  if (dias === 0) return "Hoy";
  if (dias === 1) return "Mañana";
  if (dias === -1) return "Ayer";
  if (dias > 1 && dias < 7) return capitalizar(DIAS[fecha.getDay()]);
  return `${fecha.getDate()} ${MESES_CORTOS[fecha.getMonth()]}`;
}

/** "09:00" */
export function formatearHora(fecha: Date): string {
  const h = String(fecha.getHours()).padStart(2, "0");
  const m = String(fecha.getMinutes()).padStart(2, "0");
  return `${h}:${m}`;
}

/** "Faltan 3 días", "Es mañana", "Es hoy", "Pasó hace 2 días". */
export function cuentaRegresiva(fecha: Date, hoy: Date): string {
  const dias = diasEntre(hoy, fecha);
  if (dias === 0) return "Es hoy";
  if (dias === 1) return "Es mañana";
  if (dias > 1) return `Faltan ${dias} días`;
  if (dias === -1) return "Fue ayer";
  return `Pasó hace ${-dias} días`;
}

const dos = (n: number) => String(n).padStart(2, "0");

/** Date → "2026-11-20" (día local). */
export function aFecha(fecha: Date): string {
  return `${fecha.getFullYear()}-${dos(fecha.getMonth() + 1)}-${dos(fecha.getDate())}`;
}

/** "2026-11-20" → Date a medianoche local (sin corrimiento de zona horaria). */
export function desdeFecha(texto: string): Date {
  const [a, m, d] = texto.slice(0, 10).split("-").map(Number);
  return new Date(a, m - 1, d);
}

/** "2026-11-20" + n días → "2026-11-2x". */
export function sumarDias(texto: string, dias: number): string {
  const d = desdeFecha(texto);
  d.setDate(d.getDate() + dias);
  return aFecha(d);
}

/**
 * Fecha y hora locales → ISO 8601 con la zona horaria del sistema para ese día:
 * ("2026-11-20", "09:00") → "2026-11-20T09:00:00-03:00".
 */
export function aIsoLocal(fecha: string, hora: string): string {
  const [h, min] = hora.split(":").map(Number);
  const d = desdeFecha(fecha);
  d.setHours(h, min, 0, 0);
  // getTimezoneOffset da minutos "detrás" de UTC: en Argentina, 180.
  const desfase = -d.getTimezoneOffset();
  const signo = desfase >= 0 ? "+" : "-";
  const abs = Math.abs(desfase);
  return `${fecha}T${dos(h)}:${dos(min)}:00${signo}${dos(Math.floor(abs / 60))}:${dos(abs % 60)}`;
}

/** Un momento guardado (fecha sola o ISO con zona) → Date local. */
export function momentoADate(texto: string): Date {
  return texto.length === 10 ? desdeFecha(texto) : new Date(texto);
}

/** ISO con zona → "09:00" en hora local. */
export function horaDe(iso: string): string {
  return formatearHora(new Date(iso));
}

/** "Lunes 17 de noviembre" */
export function fechaLarga(fecha: Date): string {
  const meses = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
  ];
  return `${capitalizar(DIAS[fecha.getDay()])} ${fecha.getDate()} de ${meses[fecha.getMonth()]}`;
}
