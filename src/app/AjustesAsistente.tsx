import { useState } from "react";
import { BorrarConConfirmacion, BotonTexto, Campo, Opciones } from "../components/Formulario";
import { api, mensajeDelAsistente } from "../lib/api";
import type { AjustesIa, TipoProveedor } from "../lib/tipos";

const PROVEEDORES = [
  { valor: "groq", texto: "Groq" },
  { valor: "gemini", texto: "Gemini" },
  { valor: "ollama", texto: "Ollama, en tu compu" },
] as const satisfies readonly { valor: TipoProveedor; texto: string }[];

const NOMBRE: Record<TipoProveedor, string> = { groq: "Groq", gemini: "Gemini", ollama: "Ollama" };

/** Dónde se saca cada key y cómo empieza (para el placeholder). */
const KEY: Record<"groq" | "gemini", { donde: string; empieza: string }> = {
  groq: { donde: "console.groq.com/keys", empieza: "gsk_…" },
  gemini: { donde: "aistudio.google.com, en «Claves de API»", empieza: "AIza…" },
};

/** "ninguno" en las opciones = `null` en los ajustes. */
type OpcionRespaldo = TipoProveedor | "ninguno";

interface Props {
  ia: AjustesIa;
  alCambiar: (ia: AjustesIa) => void;
  /** Los proveedores con key guardada en el llavero. */
  conKey: TipoProveedor[];
  /** Por proveedor: sin entrada = no se toca; "" = borrarla al guardar; otro texto = la nueva. */
  keysNuevas: Partial<Record<TipoProveedor, string>>;
  alCambiarKey: (proveedor: TipoProveedor, key: string | null) => void;
  /** Guarda lo de esta sección (para probar con lo que se ve en pantalla). */
  guardarIa: () => Promise<void>;
}

type Prueba = { estado: "nada" } | { estado: "probando" } | { estado: "bien" } | { estado: "mal"; mensaje: string };

/** Sección "Asistente" de Ajustes: qué IA usa, su respaldo y sus keys (SPEC 5.3 y 5.10). */
export function AjustesAsistente({ ia, alCambiar, conKey, keysNuevas, alCambiarKey, guardarIa }: Props) {
  const [prueba, setPrueba] = useState<Prueba>({ estado: "nada" });
  const [historial, setHistorial] = useState<string | null>(null);

  const cambiar = (cambios: Partial<AjustesIa>) => {
    setPrueba({ estado: "nada" });
    alCambiar({ ...ia, ...cambios });
  };

  async function probar() {
    setPrueba({ estado: "probando" });
    try {
      await guardarIa();
      await api.probarConexion();
      setPrueba({ estado: "bien" });
    } catch (e) {
      setPrueba({ estado: "mal", mensaje: mensajeDelAsistente(e) });
    }
  }

  async function borrarHistorial() {
    try {
      await api.borrarHistorial();
      setHistorial("Listo, no me acuerdo de nada.");
    } catch (e) {
      setHistorial(mensajeDelAsistente(e));
    }
  }

  const opcionesRespaldo = [
    { valor: "ninguno" as OpcionRespaldo, texto: "Ninguno" },
    ...PROVEEDORES.filter((p) => p.valor !== ia.proveedor),
  ];
  const respaldo = ia.respaldo !== ia.proveedor ? ia.respaldo : null;

  /** El modelo y la key (o la dirección, en Ollama) de un proveedor. */
  function datosDe(tipo: TipoProveedor) {
    if (tipo === "ollama") {
      return (
        <>
          <Campo etiqueta="Modelo de Ollama" valor={ia.modeloOllama} alCambiar={(modeloOllama) => cambiar({ modeloOllama })} maxLength={100} />
          <Campo etiqueta="Dirección" valor={ia.urlOllama} alCambiar={(urlOllama) => cambiar({ urlOllama })} maxLength={200} />
          <p className="text-[13px] leading-snug text-tinta-3">
            Ollama tiene que estar abierto y con el modelo bajado ({`ollama pull ${ia.modeloOllama || "modelo"}`}).
          </p>
        </>
      );
    }
    const modelo = tipo === "groq" ? ia.modeloGroq : ia.modeloGemini;
    return (
      <>
        <Campo
          etiqueta={`Modelo de ${NOMBRE[tipo]}`}
          valor={modelo}
          alCambiar={(m) => cambiar(tipo === "groq" ? { modeloGroq: m } : { modeloGemini: m })}
          maxLength={100}
        />
        <KeyEnLlavero
          proveedor={tipo}
          guardada={conKey.includes(tipo)}
          nueva={keysNuevas[tipo]}
          alCambiar={(key) => {
            setPrueba({ estado: "nada" });
            alCambiarKey(tipo, key);
          }}
        />
      </>
    );
  }

  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Asistente</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          El cerebro que entiende lo que le pedís. Groq es el más rápido; Gemini tiene más cupo gratis; con Ollama nada sale
          de tu compu.
        </p>
      </div>

      <Opciones etiqueta="Usar" opciones={PROVEEDORES} valor={ia.proveedor} alCambiar={(proveedor) => cambiar({ proveedor })} />
      {datosDe(ia.proveedor)}

      <div className="flex flex-col gap-3">
        <Opciones
          etiqueta={`Si a ${NOMBRE[ia.proveedor]} no le alcanza el cupo, usar`}
          opciones={opcionesRespaldo}
          valor={respaldo ?? "ninguno"}
          alCambiar={(valor) => cambiar({ respaldo: valor === "ninguno" ? null : valor })}
        />
        {respaldo && datosDe(respaldo)}
        {respaldo && (
          <p className="text-[13px] leading-snug text-tinta-3">
            Cuando a {NOMBRE[ia.proveedor]} se le acaban los tokens del minuto o del día, el pedido va a{" "}
            {NOMBRE[respaldo]} sin que tengas que hacer nada. Lo de fondo, como elegir noticias, va primero a{" "}
            {NOMBRE[respaldo]}.
          </p>
        )}
      </div>

      <div className="flex items-baseline gap-4">
        <BotonTexto onClick={() => void probar()} disabled={prueba.estado === "probando"}>
          Probar conexión
        </BotonTexto>
        <span aria-live="polite" className="min-w-0 text-[14px] leading-snug">
          {prueba.estado === "probando" && <span className="italic text-tinta-3">Probando…</span>}
          {prueba.estado === "bien" && <span className="text-tinta-2">{respaldo ? "Andan los dos." : "Anda bien."}</span>}
          {prueba.estado === "mal" && <span className="text-aviso">{prueba.mensaje}</span>}
        </span>
      </div>

      <div className="flex items-baseline gap-4">
        <BorrarConConfirmacion
          texto="Borrar el historial"
          pregunta="¿Borro toda la conversación?"
          alConfirmar={() => void borrarHistorial()}
        />
        {historial && (
          <span aria-live="polite" className="text-[14px] text-tinta-2">
            {historial}
          </span>
        )}
      </div>
    </section>
  );
}

/** La key de un proveedor: si hay una guardada y no se está cambiando, solo se dice eso. */
function KeyEnLlavero({
  proveedor,
  guardada,
  nueva,
  alCambiar,
}: {
  proveedor: "groq" | "gemini";
  guardada: boolean;
  nueva: string | undefined;
  alCambiar: (key: string | null) => void;
}) {
  const nombre = NOMBRE[proveedor];
  if (guardada && nueva === undefined) {
    return (
      <div className="flex flex-col gap-1">
        <span className="text-[13px] text-tinta-3">API key de {nombre}</span>
        <div className="flex items-baseline gap-4 text-[15px]">
          <span className="text-tinta-2">Guardada en el llavero de Windows</span>
          <BotonTexto onClick={() => alCambiar("")}>Cambiar</BotonTexto>
        </div>
      </div>
    );
  }
  const seBorraAlGuardar = guardada && nueva === "";
  return (
    <div className="flex flex-col gap-1">
      <Campo
        etiqueta={`API key de ${nombre}`}
        type="password"
        placeholder={seBorraAlGuardar ? "Pegá la nueva (vacía, se borra)" : KEY[proveedor].empieza}
        valor={nueva ?? ""}
        alCambiar={alCambiar}
        maxLength={300}
      />
      <p className="text-[13px] leading-snug text-tinta-3">
        Sacala gratis en {KEY[proveedor].donde}. Queda en el llavero de Windows, nunca en un archivo.
        {guardada && (
          <>
            {" "}
            <BotonTexto className="text-[13px]" onClick={() => alCambiar(null)}>
              Dejar la que estaba
            </BotonTexto>
          </>
        )}
      </p>
    </div>
  );
}
