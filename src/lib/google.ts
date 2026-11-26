// Textos de Google Calendar (v0.7.2) para Ajustes y el widget de la agenda.
import { antiguedad } from "./externos";
import type { ResumenGoogle, VistaGoogle } from "./tipos";

/** "hace 5 min", "recién", "ayer" (sin el "de" de los datos de internet). */
export function haceCuanto(iso: string, ahora: Date): string {
  return antiguedad(iso, ahora).replace(/^de /, "");
}

/** El aviso chico del widget cuando Google falla. `null` si está todo bien o no está conectado. */
export function avisoCalendar(google: VistaGoogle | null, ahora: Date): string | null {
  if (!google?.conectado || !google.error) return null;
  const error = google.error.replace(/[.:]\s*$/, "");
  return google.sincronizadoEn ? `${error} · última vez ${haceCuanto(google.sincronizadoEn, ahora)}` : error;
}

/**
 * La versión corta, para el renglón del título del widget (así no le suma
 * altura y no se encima con el de abajo). El detalle va en `avisoCalendar`.
 */
export function avisoCortoCalendar(google: VistaGoogle | null): string | null {
  if (!google?.conectado || !google.error) return null;
  return google.error.startsWith("Sin conexión") ? "Sin conexión con Google" : "Revisá Google Calendar en Ajustes";
}

/** El estado en Ajustes: "Al día · hace 2 min" o el error. */
export function estadoCalendar(google: VistaGoogle, ahora: Date): string {
  if (google.conectando) return "Esperando que aceptes en el navegador…";
  if (!google.conectado) return google.tieneCliente ? "Sin conectar" : "Falta el ID y el secreto del cliente";
  if (google.error) return avisoCalendar(google, ahora) ?? google.error;
  return google.sincronizadoEn ? `Al día · ${haceCuanto(google.sincronizadoEn, ahora)}` : "Conectado · sincronizando…";
}

function cantidad(n: number, singular: string, plural: string): string {
  return `${n} ${n === 1 ? singular : plural}`;
}

/** "Subí 5 eventos, traje 2 y junté 1 que estaba en los dos lados." */
export function textoPrimeraVez(r: ResumenGoogle): string {
  const partes: string[] = [];
  if (r.subidos > 0) partes.push(`subí ${cantidad(r.subidos, "evento", "eventos")}`);
  if (r.traidos > 0) partes.push(`traje ${r.traidos}`);
  if (r.juntados > 0) partes.push(`junté ${cantidad(r.juntados, "que estaba", "que estaban")} en los dos lados`);
  if (partes.length === 0) return "La primera vez no había nada para pasar.";
  const lista = partes.length === 1 ? partes[0] : `${partes.slice(0, -1).join(", ")} y ${partes[partes.length - 1]}`;
  return `La primera vez ${lista}.`;
}
