import { useState } from "react";
import { BorrarConConfirmacion, BotonTexto, Campo, Opciones } from "../components/Formulario";
import { api, mensajeDelAsistente } from "../lib/api";
import type { AjustesIa, TipoProveedor } from "../lib/tipos";

const PROVEEDORES = [
  { valor: "groq", texto: "Groq, en internet" },
  { valor: "ollama", texto: "Ollama, en tu compu" },
] as const satisfies readonly { valor: TipoProveedor; texto: string }[];

interface Props {
  ia: AjustesIa;
  alCambiar: (ia: AjustesIa) => void;
  /** Hay una key guardada en el llavero. */
  tieneKey: boolean;
  /** `null` = no se toca; "" = borrarla al guardar; otro texto = la nueva. */
  keyNueva: string | null;
  alCambiarKey: (key: string | null) => void;
  /** Guarda lo de esta sección (para probar con lo que se ve en pantalla). */
  guardarIa: () => Promise<void>;
}

type Prueba = { estado: "nada" } | { estado: "probando" } | { estado: "bien" } | { estado: "mal"; mensaje: string };

/** Sección "Asistente" de Ajustes: qué IA usa y su API key (SPEC 5.3 y 7). */
export function AjustesAsistente({ ia, alCambiar, tieneKey, keyNueva, alCambiarKey, guardarIa }: Props) {
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

  // La key: si hay una guardada y no se está cambiando, solo se dice eso.
  const mostrarGuardada = tieneKey && keyNueva === null;
  const seBorraAlGuardar = tieneKey && keyNueva === "";

  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Asistente</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          El cerebro que entiende lo que le pedís. Groq es más rápido; con Ollama nada sale de tu compu.
        </p>
      </div>

      <Opciones etiqueta="Usar" opciones={PROVEEDORES} valor={ia.proveedor} alCambiar={(proveedor) => cambiar({ proveedor })} />

      {ia.proveedor === "groq" ? (
        <>
          <Campo etiqueta="Modelo" valor={ia.modeloGroq} alCambiar={(modeloGroq) => cambiar({ modeloGroq })} maxLength={100} />
          {mostrarGuardada ? (
            <div className="flex flex-col gap-1">
              <span className="text-[13px] text-tinta-3">API key</span>
              <div className="flex items-baseline gap-4 text-[15px]">
                <span className="text-tinta-2">Guardada en el llavero de Windows</span>
                <BotonTexto onClick={() => alCambiarKey("")}>Cambiar</BotonTexto>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-1">
              <Campo
                etiqueta="API key de Groq"
                type="password"
                placeholder={seBorraAlGuardar ? "Pegá la nueva (vacía, se borra)" : "gsk_…"}
                valor={keyNueva ?? ""}
                alCambiar={(key) => {
                  setPrueba({ estado: "nada" });
                  alCambiarKey(key);
                }}
                maxLength={300}
              />
              <p className="text-[13px] leading-snug text-tinta-3">
                Sacala gratis en console.groq.com/keys. Queda en el llavero de Windows, nunca en un archivo.
                {tieneKey && (
                  <>
                    {" "}
                    <BotonTexto className="text-[13px]" onClick={() => alCambiarKey(null)}>
                      Dejar la que estaba
                    </BotonTexto>
                  </>
                )}
              </p>
            </div>
          )}
        </>
      ) : (
        <>
          <Campo etiqueta="Modelo" valor={ia.modeloOllama} alCambiar={(modeloOllama) => cambiar({ modeloOllama })} maxLength={100} />
          <Campo etiqueta="Dirección" valor={ia.urlOllama} alCambiar={(urlOllama) => cambiar({ urlOllama })} maxLength={200} />
          <p className="text-[13px] leading-snug text-tinta-3">
            Ollama tiene que estar abierto y con el modelo bajado ({`ollama pull ${ia.modeloOllama || "modelo"}`}).
          </p>
        </>
      )}

      <div className="flex items-baseline gap-4">
        <BotonTexto onClick={() => void probar()} disabled={prueba.estado === "probando"}>
          Probar conexión
        </BotonTexto>
        <span aria-live="polite" className="min-w-0 text-[14px] leading-snug">
          {prueba.estado === "probando" && <span className="italic text-tinta-3">Probando…</span>}
          {prueba.estado === "bien" && <span className="text-tinta-2">Anda bien.</span>}
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
