// Textos del widget de fútbol (la lógica de qué partido mostrar está en Rust,
// en dominio/futbol.rs; acá solo se arma cómo se dice).
import { diasEntre, etiquetaDia, formatearHora } from "./fechas";
import type { EquipoBreve, EstadoJugador, Partido, ResumenEquipo } from "./tipos";

/** "Boca Juniors" → "Boca". El resto, como viene. */
export function nombreCorto(equipo: EquipoBreve): string {
  if (equipo.id === "5") return "Boca";
  return equipo.nombre;
}

/** "Hoy, 19:30" o "Sábado, 19:30". */
export function cuandoJuega(iso: string, ahora: Date): string {
  const fecha = new Date(iso);
  return `${etiquetaDia(fecha, ahora)}, ${formatearHora(fecha)}`;
}

export function esHoy(iso: string, ahora: Date): boolean {
  return diasEntre(ahora, new Date(iso)) === 0;
}

/** "final de la Sudamericana", "Liga Profesional". */
export function competenciaConFase(p: Partido): string {
  if (!p.fase) return p.competencia;
  const articulo = ["Libertadores", "Sudamericana", "Copa Argentina", "Copa América", "Liga Profesional"].includes(p.competencia)
    ? "la "
    : "";
  return `${p.fase.charAt(0).toUpperCase()}${p.fase.slice(1)} de ${articulo}${p.competencia}`;
}

export function golesDe(p: Partido, equipoId: string): number {
  return (p.local.id === equipoId ? p.golesLocal : p.golesVisitante) ?? 0;
}

export function rivalDe(p: Partido, equipoId: string): EquipoBreve {
  return p.local.id === equipoId ? p.visitante : p.local;
}

/** "Le ganó 3 a 0 a Banfield", "Empató 1 a 1 con Racing", "Perdió 0 a 2 con River". */
export function textoResultado(p: Partido, equipoId: string): string {
  const propios = golesDe(p, equipoId);
  const rival = rivalDe(p, equipoId);
  const contra = golesDe(p, rival.id);
  const nombre = nombreCorto(rival);
  let texto: string;
  if (propios > contra) texto = `Le ganó ${propios} a ${contra} a ${nombre}`;
  else if (propios < contra) texto = `Perdió ${propios} a ${contra} con ${nombre}`;
  else texto = `Empató ${propios} a ${contra} con ${nombre}`;
  if (p.penalesLocal !== null && p.penalesVisitante !== null) {
    const [pp, pr] = p.local.id === equipoId ? [p.penalesLocal, p.penalesVisitante] : [p.penalesVisitante, p.penalesLocal];
    texto += pp > pr ? ` y ganó por penales` : ` y perdió por penales`;
  }
  return texto;
}

/** El detalle a la derecha del título: "2.º en la Zona A · 33 pts". */
export function textoPosicion(r: ResumenEquipo): string | undefined {
  if (!r.posicion) return undefined;
  const { fila, zona } = r.posicion;
  const enZona = zona ? ` en la ${zona}` : "";
  return `${fila.puesto}.º${enZona} · ${fila.puntos} pts`;
}

/** Lo que hizo un jugador en su último partido, en pocas palabras. */
export function lineaJugador(j: EstadoJugador, ahora: Date): string {
  const u = j.ultimo;
  if (!u) return "Sin partidos";
  const dias = diasEntre(new Date(u.fecha), ahora);
  const cuando = dias <= 0 ? "hoy" : dias === 1 ? "ayer" : dias < 7 ? `hace ${dias} días` : etiquetaDia(new Date(u.fecha), ahora);
  const aportes = [
    u.goles > 0 ? `${u.goles} ${u.goles === 1 ? "gol" : "goles"}` : null,
    u.asistencias > 0 ? `${u.asistencias} ${u.asistencias === 1 ? "asistencia" : "asistencias"}` : null,
  ].filter(Boolean);
  if (dias > 10) return `No juega desde el ${etiquetaDia(new Date(u.fecha), ahora).toLowerCase()}`;
  if (aportes.length > 0) return `${aportes.join(" y ")}, ${cuando}`;
  return `Jugó ${cuando}`;
}

/** Los goles de un equipo en un partido, para la lista del modo en vivo. */
export function goleadores(p: Partido, equipoId: string): string[] {
  return p.eventos
    .filter((e) => e.equipoId === equipoId && e.tipo !== "expulsion")
    .map((e) => `${e.jugador ?? "Gol"}${e.tipo === "gol_en_contra" ? " (en contra)" : ""} ${e.minuto}`);
}

export function expulsados(p: Partido): { equipo: string; texto: string }[] {
  return p.eventos
    .filter((e) => e.tipo === "expulsion")
    .map((e) => ({ equipo: e.equipoId, texto: `${e.jugador ?? "Roja"} ${e.minuto}` }));
}
