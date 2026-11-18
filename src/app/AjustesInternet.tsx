import { useEffect, useState } from "react";
import { BotonTexto, Campo, Interruptor } from "../components/Formulario";
import { api, mensajeDeError } from "../lib/api";
import type { AjustesClima, AjustesNoticias, JugadorSeguido } from "../lib/tipos";

interface Props {
  jugadores: JugadorSeguido[];
  alCambiarJugadores: (j: JugadorSeguido[]) => void;
  noticias: AjustesNoticias;
  alCambiarNoticias: (n: AjustesNoticias) => void;
  clima: AjustesClima;
  alCambiarClima: (c: AjustesClima) => void;
}

const MAX_JUGADORES = 20;

function Titulo({ titulo, ayuda }: { titulo: string; ayuda: string }) {
  return (
    <div>
      <h3 className="font-display text-[17px] italic text-tinta-2">{titulo}</h3>
      <p className="mt-1 text-[13px] leading-snug text-tinta-3">{ayuda}</p>
    </div>
  );
}

/** Lista de cosas con "Sacar" al lado. */
function Lista<T>({ items, clave, texto, alSacar }: { items: T[]; clave: (t: T) => string; texto: (t: T) => string; alSacar: (t: T) => void }) {
  return (
    <ul className="flex flex-col text-[15px]">
      {items.map((t) => (
        <li key={clave(t)} className="fila -mx-2 flex items-baseline gap-3 px-2 py-0.5">
          <span className="min-w-0 flex-1 truncate">{texto(t)}</span>
          <BotonTexto className="text-[13px]" onClick={() => alSacar(t)}>
            Sacar
          </BotonTexto>
        </li>
      ))}
    </ul>
  );
}

/**
 * Un campo que busca en internet mientras escribís (espera a que pares un
 * momento, para no pedir en cada letra) y muestra los resultados para elegir.
 */
function Buscador<T>({
  etiqueta,
  buscar,
  clave,
  texto,
  alElegir,
  minimo,
}: {
  etiqueta: string;
  buscar: (texto: string) => Promise<T[]>;
  clave: (t: T) => string;
  texto: (t: T) => string;
  alElegir: (t: T) => void;
  minimo: number;
}) {
  const [consulta, setConsulta] = useState("");
  const [resultados, setResultados] = useState<T[]>([]);
  const [estado, setEstado] = useState<"nada" | "buscando" | "vacio" | string>("nada");

  useEffect(() => {
    const limpia = consulta.trim();
    if (limpia.length < minimo) {
      setResultados([]);
      setEstado("nada");
      return;
    }
    let vigente = true;
    const espera = window.setTimeout(() => {
      setEstado("buscando");
      buscar(limpia).then(
        (r) => {
          if (!vigente) return;
          setResultados(r);
          setEstado(r.length === 0 ? "vacio" : "nada");
        },
        (e: unknown) => vigente && setEstado(mensajeDeError(e)),
      );
    }, 400);
    return () => {
      vigente = false;
      window.clearTimeout(espera);
    };
  }, [consulta, buscar, minimo]);

  return (
    <div className="flex flex-col gap-1">
      <Campo etiqueta={etiqueta} valor={consulta} alCambiar={setConsulta} maxLength={60} />
      <div aria-live="polite" className="text-[14px]">
        {estado === "buscando" && <span className="italic text-tinta-3">Buscando…</span>}
        {estado === "vacio" && <span className="text-tinta-3">No encontré nada.</span>}
        {!["nada", "buscando", "vacio"].includes(estado) && <span className="text-aviso">{estado}</span>}
      </div>
      {resultados.length > 0 && (
        <ul className="flex flex-col">
          {resultados.map((r) => (
            <li key={clave(r)}>
              <button
                type="button"
                onClick={() => {
                  alElegir(r);
                  setConsulta("");
                }}
                className="fila -mx-2 w-full px-2 py-0.5 text-left text-[15px] text-tinta-2 hover:text-tinta"
              >
                {texto(r)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Ajustes de lo que viene de internet: jugadores, noticias y clima. */
export function AjustesInternet({ jugadores, alCambiarJugadores, noticias, alCambiarNoticias, clima, alCambiarClima }: Props) {
  const [feedNuevo, setFeedNuevo] = useState({ nombre: "", url: "" });
  const urlValida = /^https?:\/\/\S+\.\S+/.test(feedNuevo.url.trim());

  return (
    <>
      <section className="mt-2 flex flex-col gap-3">
        <Titulo titulo="Jugadores" ayuda="Los que seguís: Nova te cuenta si jugaron y si hicieron goles o asistencias." />
        <Lista items={jugadores} clave={(j) => j.id} texto={(j) => j.nombre} alSacar={(j) => alCambiarJugadores(jugadores.filter((x) => x.id !== j.id))} />
        {jugadores.length < MAX_JUGADORES && (
          <Buscador
            etiqueta="Agregar un jugador"
            minimo={3}
            buscar={api.buscarJugadores}
            clave={(j) => j.id}
            texto={(j) => (j.club ? `${j.nombre} · ${j.club}` : j.nombre)}
            alElegir={(j) => {
              if (!jugadores.some((x) => x.id === j.id)) alCambiarJugadores([...jugadores, { id: j.id, nombre: j.nombre }]);
            }}
          />
        )}
      </section>

      <section className="mt-2 flex flex-col gap-3">
        <Titulo titulo="Noticias" ayuda="La IA elige entre los titulares de tus feeds según lo que te interesa." />
        <label className="flex flex-col gap-1">
          <span className="text-[13px] text-tinta-3">Me interesa</span>
          <textarea
            value={noticias.intereses}
            onChange={(e) => alCambiarNoticias({ ...noticias, intereses: e.target.value })}
            maxLength={300}
            rows={2}
            className="w-full resize-none border-b border-tinta-4 bg-transparent pb-1.5 text-[16px] leading-snug text-tinta outline-none focus:border-tinta-2"
          />
        </label>
        <Lista
          items={noticias.feeds}
          clave={(f) => f.url}
          texto={(f) => `${f.nombre} · ${f.url.replace(/^https?:\/\/(www\.)?/, "")}`}
          alSacar={(f) => alCambiarNoticias({ ...noticias, feeds: noticias.feeds.filter((x) => x.url !== f.url) })}
        />
        <div className="grid grid-cols-[7rem_minmax(0,1fr)_auto] items-end gap-3">
          <Campo etiqueta="Nombre" valor={feedNuevo.nombre} alCambiar={(nombre) => setFeedNuevo((f) => ({ ...f, nombre }))} maxLength={40} />
          <Campo etiqueta="Dirección del feed" valor={feedNuevo.url} alCambiar={(url) => setFeedNuevo((f) => ({ ...f, url }))} maxLength={300} placeholder="https://…/rss" />
          <BotonTexto
            disabled={!urlValida || feedNuevo.nombre.trim() === ""}
            onClick={() => {
              alCambiarNoticias({ ...noticias, feeds: [...noticias.feeds, { nombre: feedNuevo.nombre.trim(), url: feedNuevo.url.trim() }] });
              setFeedNuevo({ nombre: "", url: "" });
            }}
          >
            Agregar
          </BotonTexto>
        </div>
      </section>

      <section className="mt-2 flex flex-col gap-3">
        <Titulo titulo="Clima" ayuda="Temperatura y cielo al lado de la fecha (de Open-Meteo, gratis)." />
        <Interruptor texto="Mostrar el clima" valor={clima.mostrar} alCambiar={(mostrar) => alCambiarClima({ ...clima, mostrar })} />
        {clima.mostrar && (
          <>
            <p className="text-[15px]">
              <span className="text-tinta-3">Ciudad:</span> {clima.ciudad}
            </p>
            <Buscador
              etiqueta="Cambiar de ciudad"
              minimo={2}
              buscar={api.buscarCiudades}
              clave={(c) => `${c.latitud},${c.longitud}`}
              texto={(c) => `${c.nombre} · ${c.region}`}
              alElegir={(c) => alCambiarClima({ ...clima, ciudad: c.nombre, latitud: c.latitud, longitud: c.longitud })}
            />
          </>
        )}
      </section>
    </>
  );
}
