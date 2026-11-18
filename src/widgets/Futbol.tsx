import { AnimatePresence, motion } from "framer-motion";
import { Seccion } from "../components/Seccion";
import { estadoExterno } from "../lib/externos";
import {
  competenciaConFase,
  cuandoJuega,
  esHoy,
  expulsados,
  golesDe,
  goleadores,
  lineaJugador,
  nombreCorto,
  rivalDe,
  textoPosicion,
  textoResultado,
} from "../lib/futbol";
import type { Partido, ResumenEquipo } from "../lib/tipos";
import { useDatos } from "../store/datos";
import { cambioDeVista } from "./animacionWidget";

/** Cuántos jugadores entran sin empujar al widget de abajo. */
const JUGADORES_VISIBLES = 4;

/** Boca de protagonista; abajo, la Selección y los jugadores que sigue. */
export function Futbol({ ahora }: { ahora: Date }) {
  const futbol = useDatos((s) => s.futbol);
  const estado = estadoExterno(futbol, ahora);
  const vista = futbol?.datos ?? null;
  const boca = vista?.equipos.find((e) => e.clave === "boca") ?? null;
  const seleccion = vista?.equipos.find((e) => e.clave === "seleccion") ?? null;
  const vivo = boca?.enVivo ?? seleccion?.enVivo ?? null;
  const protagonista = boca?.enVivo ? boca : seleccion?.enVivo ? seleccion : boca;
  // Los que jugaron hace poco primero; los que no juegan hace rato, al final.
  const jugadores = [...(vista?.jugadores ?? [])]
    .sort((a, b) => (b.ultimo?.fecha ?? "").localeCompare(a.ultimo?.fecha ?? ""))
    .slice(0, JUGADORES_VISIBLES);

  return (
    <Seccion
      titulo={protagonista?.clave === "seleccion" ? "La Selección" : "Boca"}
      detalle={vivo ? <EnVivoEtiqueta /> : boca ? textoPosicion(boca) : undefined}
      className="shrink-0"
    >
      {estado.tipo === "cargando" && <p className="text-[15px] text-tinta-3">Buscando los partidos…</p>}
      {estado.tipo === "sin_datos" && (
        <p className="text-[15px] leading-snug text-tinta-3">{estado.mensaje} Vuelvo a probar en un rato.</p>
      )}

      {/* `layout` anima el cambio de alto cuando entra o sale el modo en vivo */}
      <motion.div layout="size" transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}>
        <AnimatePresence mode="wait" initial={false}>
          {vivo && protagonista ? (
            <motion.div key="vivo" {...cambioDeVista}>
              <EnVivo partido={vivo} />
            </motion.div>
          ) : (
            protagonista && (
              <motion.div key="previa" {...cambioDeVista}>
                <Previa equipo={protagonista} ahora={ahora} />
              </motion.div>
            )
          )}
        </AnimatePresence>

        {vista && (
          <ul className="mt-5 flex flex-col text-[15px]">
            {seleccion && protagonista?.clave !== "seleccion" && <FilaSeleccion equipo={seleccion} ahora={ahora} />}
            {/* En vivo, el partido ocupa el lugar de los jugadores */}
            {!vivo &&
              jugadores.map((j) => (
                <li key={j.id} className="fila -mx-2 flex items-baseline gap-3 px-2 py-1">
                  <span className="min-w-0 flex-1 truncate">
                    {j.nombre} {j.club && <span className="text-tinta-3">en {j.club}</span>}
                  </span>
                  <span className="shrink-0 text-[14px] text-tinta-3">{lineaJugador(j, ahora)}</span>
                </li>
              ))}
          </ul>
        )}
      </motion.div>

      {estado.tipo === "listo" && estado.aviso && <p className="mt-3 text-[13px] text-aviso">{estado.aviso}</p>}
    </Seccion>
  );
}

function EnVivoEtiqueta() {
  return (
    <span className="inline-flex items-center gap-1.5 text-aviso">
      <span aria-hidden className="latido size-1.5 rounded-full bg-aviso" />
      en vivo
    </span>
  );
}

/** Próximo partido grande y el último resultado chiquito. */
function Previa({ equipo, ahora }: { equipo: ResumenEquipo; ahora: Date }) {
  const p = equipo.proximo;
  const u = equipo.ultimo;
  return (
    <>
      {p ? (
        <>
          <p className="font-display text-[38px] font-[380] leading-[1.05] tracking-[-0.015em]">
            {nombreCorto(p.local)} <span className="text-tinta-3">y</span> {nombreCorto(p.visitante)}
          </p>
          <p className="mt-1.5 text-[16px] text-tinta-2">
            <span className={esHoy(p.fecha, ahora) ? "text-aviso" : ""}>{cuandoJuega(p.fecha, ahora)}</span>
            <span className="text-tinta-3"> · {competenciaConFase(p)}</span>
          </p>
        </>
      ) : (
        <p className="font-display text-[26px] font-[380] text-tinta-2">Sin partidos a la vista</p>
      )}
      {u && <p className="mt-1 text-[14px] text-tinta-3">{textoResultado(u, equipo.id)}</p>}
    </>
  );
}

/** Marcador grande, minuto y los goles de cada lado. */
function EnVivo({ partido: p }: { partido: Partido }) {
  const minuto = p.estado === "entretiempo" ? "Entretiempo" : (p.minuto ?? "Por empezar");
  const rojas = expulsados(p);
  return (
    <div aria-live="polite">
      <p className="sr-only">
        {nombreCorto(p.local)} {golesDe(p, p.local.id)}, {nombreCorto(p.visitante)} {golesDe(p, p.visitante.id)}, {minuto}
      </p>
      <div aria-hidden className="grid grid-cols-[1fr_auto_1fr] items-baseline gap-3">
        <span className="truncate text-[17px] text-tinta-2">{nombreCorto(p.local)}</span>
        <span className="font-display text-[56px] font-[380] leading-none tracking-[-0.02em] tabular-nums">
          {golesDe(p, p.local.id)}
          <span className="px-2 text-tinta-3">–</span>
          {golesDe(p, p.visitante.id)}
        </span>
        <span className="truncate text-right text-[17px] text-tinta-2">{nombreCorto(p.visitante)}</span>
      </div>
      <p className="mt-2 text-center text-[15px] text-aviso">
        {minuto} <span className="text-tinta-3">· {competenciaConFase(p)}</span>
      </p>
      <div className="mt-4 grid grid-cols-2 gap-4 text-[14px] leading-relaxed text-tinta-2">
        <ul>
          {goleadores(p, p.local.id).map((g) => (
            <li key={g}>{g}</li>
          ))}
        </ul>
        <ul className="text-right">
          {goleadores(p, p.visitante.id).map((g) => (
            <li key={g}>{g}</li>
          ))}
        </ul>
      </div>
      {rojas.length > 0 && (
        <p className="mt-2 text-[13px] text-tinta-3">
          Expulsados: {rojas.map((r) => `${r.texto} (${nombreCorto(r.equipo === p.local.id ? p.local : p.visitante)})`).join(", ")}
        </p>
      )}
    </div>
  );
}

/** La Selección en una fila: en vivo, próximo partido o último resultado. */
function FilaSeleccion({ equipo, ahora }: { equipo: ResumenEquipo; ahora: Date }) {
  const p = equipo.enVivo ?? equipo.proximo;
  let derecha: string;
  let texto: string;
  if (equipo.enVivo) {
    texto = `Argentina ${golesDe(equipo.enVivo, equipo.id)}, ${nombreCorto(rivalDe(equipo.enVivo, equipo.id))} ${golesDe(equipo.enVivo, rivalDe(equipo.enVivo, equipo.id).id)}`;
    derecha = equipo.enVivo.minuto ?? "en vivo";
  } else if (p) {
    texto = `Argentina y ${nombreCorto(rivalDe(p, equipo.id))}`;
    derecha = cuandoJuega(p.fecha, ahora);
  } else if (equipo.ultimo) {
    const resultado = textoResultado(equipo.ultimo, equipo.id);
    texto = `Argentina ${resultado.charAt(0).toLowerCase()}${resultado.slice(1)}`;
    derecha = "";
  } else {
    return null;
  }
  return (
    <li className="fila -mx-2 flex items-baseline gap-3 px-2 py-1">
      <span className="min-w-0 flex-1 truncate">{texto}</span>
      <span className={`shrink-0 text-[14px] ${equipo.enVivo ? "text-aviso" : "text-tinta-3"}`}>{derecha}</span>
    </li>
  );
}
