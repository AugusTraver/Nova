import { useState } from "react";
import { BotonTexto, Interruptor } from "../components/Formulario";
import { api, mensajeDeError } from "../lib/api";
import type { AjustesVoz as TipoAjustesVoz, VistaVoz } from "../lib/tipos";
import { textoDescarga } from "../lib/voz";
import { useVoz } from "../store/voz";

interface Props {
  vista: VistaVoz;
  alCambiar: (ajustes: TipoAjustesVoz) => void;
  /** Se bajaron los modelos: hay que volver a pedir el estado. */
  alBajar: () => void;
}

/** Sección "Voz" de Ajustes (SPEC 7): activarla, velocidad, resumen y modelos. */
export function AjustesVoz({ vista, alCambiar, alBajar }: Props) {
  const { ajustes } = vista;
  const descarga = useVoz((s) => s.descarga);
  const [error, setError] = useState<string | null>(null);
  const bajando = descarga !== null && !descarga.terminado;
  const cambiar = (cambios: Partial<TipoAjustesVoz>) => alCambiar({ ...ajustes, ...cambios });

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
          Al abrir el panel, Nova te escucha sola y te contesta hablando. Todo pasa en tu compu: el audio no sale de acá.
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
