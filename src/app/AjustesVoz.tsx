import { useState } from "react";
import { BotonTexto, Campo, Interruptor, Opciones } from "../components/Formulario";
import { api, mensajeDeError, mensajeDelAsistente } from "../lib/api";
import type { AjustesVoz as TipoAjustesVoz, VistaVoz } from "../lib/tipos";
import { VOCES_AZURE, textoDescarga, vozQueHabla } from "../lib/voz";
import { useVoz } from "../store/voz";

interface Props {
  vista: VistaVoz;
  alCambiar: (ajustes: TipoAjustesVoz) => void;
  /** Se bajaron los modelos: hay que volver a pedir el estado. */
  alBajar: () => void;
  /** Se guardó o borró la key de Azure. */
  alCambiarKeyAzure: (tiene: boolean) => void;
}

type Prueba = { estado: "nada" } | { estado: "probando" } | { estado: "anda" } | { estado: "falla"; mensaje: string };

/** Sección "Voz" de Ajustes (SPEC 7): activarla, velocidad, resumen y modelos. */
export function AjustesVoz({ vista, alCambiar, alBajar, alCambiarKeyAzure }: Props) {
  const { ajustes } = vista;
  const descarga = useVoz((s) => s.descarga);
  const [error, setError] = useState<string | null>(null);
  const bajando = descarga !== null && !descarga.terminado;
  const cambiar = (cambios: Partial<TipoAjustesVoz>) => {
    setPrueba({ estado: "nada" });
    alCambiar({ ...ajustes, ...cambios });
  };
  // La key: null = no se está cambiando (si hay una, solo se dice eso).
  const [keyNueva, setKeyNueva] = useState<string | null>(null);
  const [prueba, setPrueba] = useState<Prueba>({ estado: "nada" });
  const conAzure = vozQueHabla(ajustes.regionAzure, vista.tieneKeyAzure) === "azure";

  /** Guarda región, voz y key, y le pide a Nova que salude con esa voz. */
  async function probar() {
    setPrueba({ estado: "probando" });
    try {
      await api.guardarAjustesVoz(ajustes);
      if (keyNueva !== null) {
        alCambiarKeyAzure(await api.guardarKeyAzure(keyNueva));
        setKeyNueva(null);
      }
      await api.probarVozAzure();
      setPrueba({ estado: "anda" });
    } catch (e) {
      setPrueba({ estado: "falla", mensaje: mensajeDelAsistente(e) });
    }
  }

  async function bajar() {
    setError(null);
    useVoz.getState().ponerDescarga({ nombre: "Empezando", megasBajados: 0, megasTotales: vista.megasFaltantes, terminado: false, error: null });
    try {
      await api.bajarModelosVoz();
      alBajar();
    } catch (e) {
      setError(mensajeDeError(e));
      useVoz.getState().ponerDescarga(null);
    }
  }

  return (
    <section className="mt-2 flex flex-col gap-3">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Voz</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          Al abrir el panel, Nova te escucha sola y te contesta hablando. Lo que decís se entiende en tu compu: tu audio no sale de acá.
        </p>
      </div>

      <Interruptor texto="Escuchar y hablar" valor={ajustes.activada} alCambiar={(activada) => cambiar({ activada })} />

      {ajustes.activada && (
        <>
          {!vista.modelosListos && (
            <div className="flex flex-col gap-1.5">
              <p className="text-[15px] leading-snug">
                Falta bajar la voz: unos {new Intl.NumberFormat("es-AR").format(vista.megasFaltantes)} MB, una sola vez.
              </p>
              {bajando ? (
                <p aria-live="polite" className="text-[14px] text-tinta-2">
                  Bajando {descarga.nombre.toLowerCase()} · {textoDescarga(descarga.megasBajados, descarga.megasTotales)}
                </p>
              ) : (
                <BotonTexto onClick={() => void bajar()} className="self-start">
                  Bajar la voz
                </BotonTexto>
              )}
              {(error ?? descarga?.error) && <p className="text-[14px] text-aviso">{error ?? descarga?.error}</p>}
            </div>
          )}

          <label className="flex flex-col gap-1">
            <span className="text-[13px] text-tinta-3">Velocidad al hablar · {ajustes.velocidad.toFixed(1).replace(".", ",")}</span>
            <input
              type="range"
              min={0.7}
              max={1.4}
              step={0.1}
              value={ajustes.velocidad}
              onChange={(e) => cambiar({ velocidad: Number(e.target.value) })}
              className="w-full accent-[var(--color-tinta-2)]"
            />
          </label>

          <div className="flex flex-col gap-3">
            <div>
              <h4 className="text-[15px] text-tinta-2">Voz de Microsoft</h4>
              <p className="mt-1 text-[13px] leading-snug text-tinta-3">
                Mucho más natural que la local. Lo que dice Nova se manda a Microsoft para convertirlo en voz; tu micrófono no.
                Sin internet, habla con la voz local. Sacá la key gratis en portal.azure.com creando un recurso «Speech» con el
                plan F0.
              </p>
            </div>
            <Opciones
              etiqueta="Voz"
              opciones={VOCES_AZURE.map((v) => ({ valor: v.valor, texto: v.texto }))}
              valor={ajustes.vozAzure}
              alCambiar={(vozAzure) => cambiar({ vozAzure })}
            />
            <Campo
              etiqueta="Región"
              placeholder="eastus"
              valor={ajustes.regionAzure}
              alCambiar={(regionAzure) => cambiar({ regionAzure })}
              maxLength={40}
            />
            {vista.tieneKeyAzure && keyNueva === null ? (
              <div className="flex flex-col gap-1">
                <span className="text-[13px] text-tinta-3">Key</span>
                <div className="flex items-baseline gap-4 text-[15px]">
                  <span className="text-tinta-2">Guardada en el llavero de Windows</span>
                  <BotonTexto onClick={() => setKeyNueva("")}>Cambiar</BotonTexto>
                </div>
              </div>
            ) : (
              <div className="flex flex-col gap-1">
                <Campo
                  etiqueta="Key de Azure"
                  type="password"
                  placeholder={vista.tieneKeyAzure ? "Pegá la nueva (vacía, se borra)" : "Key 1 del recurso"}
                  valor={keyNueva ?? ""}
                  alCambiar={(key) => {
                    setPrueba({ estado: "nada" });
                    setKeyNueva(key);
                  }}
                  maxLength={200}
                />
                <p className="text-[13px] leading-snug text-tinta-3">
                  Queda en el llavero de Windows, nunca en un archivo.
                  {vista.tieneKeyAzure && (
                    <>
                      {" "}
                      <BotonTexto className="text-[13px]" onClick={() => setKeyNueva(null)}>
                        Dejar la que estaba
                      </BotonTexto>
                    </>
                  )}
                </p>
              </div>
            )}
            <div className="flex items-baseline gap-4">
              <BotonTexto onClick={() => void probar()} disabled={prueba.estado === "probando"} className="self-start">
                Guardar y probar
              </BotonTexto>
              <span aria-live="polite" className={`text-[14px] ${prueba.estado === "falla" ? "text-aviso" : "text-tinta-2"}`}>
                {prueba.estado === "probando" && "Probando…"}
                {prueba.estado === "anda" && "Anda: Nova te saluda con esa voz."}
                {prueba.estado === "falla" && prueba.mensaje}
                {prueba.estado === "nada" && (conAzure ? "Usando la voz de Microsoft." : "Por ahora habla con la voz local.")}
              </span>
            </div>
          </div>

          <Interruptor
            texto="Leerme el resumen la primera vez que abro el panel en el día"
            valor={ajustes.leerResumen}
            alCambiar={(leerResumen) => cambiar({ leerResumen })}
          />
          {vista.hayGpu && (
            <Interruptor
              texto="Usar la placa de video (entiende más rápido)"
              valor={ajustes.usarGpu}
              alCambiar={(usarGpu) => cambiar({ usarGpu })}
            />
          )}
        </>
      )}
    </section>
  );
}
