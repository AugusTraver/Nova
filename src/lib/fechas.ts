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
