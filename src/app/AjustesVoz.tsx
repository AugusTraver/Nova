import { type ReactNode, useEffect, useState } from "react";
import { BotonTexto, Campo, Interruptor, Opciones } from "../components/Formulario";
import { api, mensajeDeError, mensajeDelAsistente } from "../lib/api";
import type { AjustesVoz as TipoAjustesVoz, ProveedorVoz, VistaVoz, VozDeCuenta } from "../lib/tipos";
import { NOMBRE_PROVEEDOR, VOCES_AZURE, VOCES_OPENAI, proveedorListo, textoDemora, textoDescarga, vozQueHabla } from "../lib/voz";
import { useVoz } from "../store/voz";

interface Props {
  vista: VistaVoz;
  alCambiar: (ajustes: TipoAjustesVoz) => void;
  /** Se bajaron los modelos: hay que volver a pedir el estado. */
  alBajar: () => void;
  /** Se guardó o borró la key de un proveedor. */
  alCambiarKey: (proveedor: ProveedorVoz, tiene: boolean) => void;
}

type Prueba = { estado: "nada" } | { estado: "probando" } | { estado: "anda"; milisegundos: number } | { estado: "falla"; mensaje: string };

const PROVEEDORES: readonly ProveedorVoz[] = ["azure", "elevenlabs", "openai", "local"];
const sinPruebas = (): Record<ProveedorVoz, Prueba> => ({
  azure: { estado: "nada" },
  elevenlabs: { estado: "nada" },
  openai: { estado: "nada" },
  local: { estado: "nada" },
});

/** Sección "Voz" de Ajustes (SPEC 7): activarla, con qué voz habla (y
 * compararlas), velocidad, resumen y modelos. */
export function AjustesVoz({ vista, alCambiar, alBajar, alCambiarKey }: Props) {
  const { ajustes, conKey } = vista;
  const proveedor = ajustes.proveedor;
  const descarga = useVoz((s) => s.descarga);
  const [error, setError] = useState<string | null>(null);
  const bajando = descarga !== null && !descarga.terminado;
  // Lo que dio cada prueba queda a la vista, para comparar al ir y volver.
  const [pruebas, setPruebas] = useState(sinPruebas);
  const ponerPrueba = (p: ProveedorVoz, prueba: Prueba) => setPruebas((t) => ({ ...t, [p]: prueba }));
  // Las keys: null = no se está cambiando (si hay una, solo se dice eso).
  const [keysNuevas, setKeysNuevas] = useState<Record<ProveedorVoz, string | null>>({ azure: null, elevenlabs: null, openai: null, local: null });
  const [voces, setVoces] = useState<VozDeCuenta[] | null>(null);
  const [errorVoces, setErrorVoces] = useState<string | null>(null);

  const cambiar = (cambios: Partial<TipoAjustesVoz>) => {
    ponerPrueba(proveedor, { estado: "nada" });
    alCambiar({ ...ajustes, ...cambios });
  };

  /** Si se escribió una key nueva para `p`, la guarda en el llavero. */
  async function guardarKeySiHay(p: ProveedorVoz) {
    const key = keysNuevas[p];
    if (key === null) return;
    alCambiarKey(p, await api.guardarKeyVoz(p, key));
    setKeysNuevas((k) => ({ ...k, [p]: null }));
  }

  /** Guarda todo y Nova dice la frase de prueba con la voz elegida. */
  async function escuchar() {
    const p = proveedor;
    ponerPrueba(p, { estado: "probando" });
    try {
      await api.guardarAjustesVoz(ajustes);
      await guardarKeySiHay(p);
      const { milisegundos } = await api.probarVoz(p);
      ponerPrueba(p, { estado: "anda", milisegundos });
    } catch (e) {
      ponerPrueba(p, { estado: "falla", mensaje: mensajeDelAsistente(e) });
    }
  }

  async function traerVoces() {
    setErrorVoces(null);
    try {
      await guardarKeySiHay("elevenlabs");
      const lista = await api.vocesElevenlabs();
      setVoces(lista);
      const primera = lista[0];
      if (ajustes.vozEleven === "" && primera) cambiar({ vozEleven: primera.id });
      if (lista.length === 0) setErrorVoces("Tu cuenta no tiene voces. Agregá alguna desde la Voice Library.");
    } catch (e) {
      setErrorVoces(mensajeDelAsistente(e));
    }
  }

  // Con la key ya guardada, las voces de ElevenLabs se traen solas al mirarlas.
  const tieneKeyEleven = conKey.includes("elevenlabs");
  useEffect(() => {
    if (proveedor === "elevenlabs" && tieneKeyEleven && voces === null) void traerVoces();
    // `traerVoces` cambia en cada render; alcanza con mirar estas tres.
  }, [proveedor, tieneKeyEleven, voces]);

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

  const campoKey = (p: Exclude<ProveedorVoz, "local">, placeholder: string) => (
    <CampoKey
      tiene={conKey.includes(p)}
      valor={keysNuevas[p]}
      placeholder={placeholder}
      etiqueta={`Key de ${NOMBRE_PROVEEDOR[p]}`}
      alCambiar={(key) => {
        ponerPrueba(p, { estado: "nada" });
        setKeysNuevas((k) => ({ ...k, [p]: key }));
      }}
    />
  );

  const prueba = pruebas[proveedor];
  const habla = vozQueHabla(ajustes, conKey);
  const esHd = proveedor === "azure" && ajustes.vozAzure.includes(":DragonHD");

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

          <div className="flex flex-col gap-3">
            <div>
              <h4 className="text-[15px] text-tinta-2">Con qué voz te habla</h4>
              <p className="mt-1 text-[13px] leading-snug text-tinta-3">
                Elegí una y tocá «Escuchar»: todas dicen la misma frase, así las comparás. A las de internet les llega solo lo que dice
                Nova, nunca tu micrófono. Sin internet, habla la local.
              </p>
            </div>
            <Opciones
              etiqueta="Habla con"
              opciones={PROVEEDORES.map((p) => ({
                valor: p,
                // Las que ya se probaron muestran cuánto tardaron, para comparar.
                texto: pruebas[p].estado === "anda" ? `${NOMBRE_PROVEEDOR[p]} · ${pruebas[p].milisegundos} ms` : NOMBRE_PROVEEDOR[p],
              }))}
              valor={proveedor}
              alCambiar={(p) => alCambiar({ ...ajustes, proveedor: p })}
            />

            {proveedor === "azure" && (
              <>
                <Ayuda>
                  Las argentinas tienen nuestro acento; las HD entonan solas según lo que dicen, pero no cambian de velocidad. Sacá la key
                  gratis en portal.azure.com creando un recurso «Speech» con el plan F0.
                </Ayuda>
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
                {campoKey("azure", "Key 1 del recurso")}
              </>
            )}

            {proveedor === "elevenlabs" && (
              <>
                <Ayuda>
                  La más natural. Sacá la key en elevenlabs.io, en Developers → API Keys, con permiso para Text to Speech y para leer
                  Voices. Para voces argentinas, buscá «Argentina» en la Voice Library y agregalas a tus voces. El plan gratis alcanza para
                  probar; después se paga.
                </Ayuda>
                {campoKey("elevenlabs", "sk_…")}
                {voces && voces.length > 0 ? (
                  <Opciones
                    etiqueta="Voz"
                    opciones={voces.map((v) => ({ valor: v.id, texto: v.detalle ? `${v.nombre} (${v.detalle.split(",")[0]})` : v.nombre }))}
                    valor={ajustes.vozEleven}
                    alCambiar={(vozEleven) => cambiar({ vozEleven })}
                  />
                ) : null}
                <div className="flex flex-col gap-1">
                  <BotonTexto onClick={() => void traerVoces()} className="self-start">
                    {voces ? "Volver a traer mis voces" : "Traer mis voces"}
                  </BotonTexto>
                  {errorVoces && <p className="text-[14px] text-aviso">{errorVoces}</p>}
                </div>
                <Opciones
                  etiqueta="Modelo"
                  opciones={[
                    { valor: "eleven_flash_v2_5", texto: "Rápido" },
                    { valor: "eleven_multilingual_v2", texto: "Más expresivo (tarda más)" },
                  ]}
                  valor={ajustes.modeloEleven}
                  alCambiar={(modeloEleven) => cambiar({ modeloEleven })}
                />
              </>
            )}

            {proveedor === "openai" && (
              <>
                <Ayuda>
                  Se le pide en palabras que hable en rioplatense, cálida y relajada. Sacá la key en platform.openai.com, en API keys. No
                  tiene plan gratis: se paga por uso, y una respuesta corta sale menos de un centavo de dólar.
                </Ayuda>
                <Opciones
                  etiqueta="Voz"
                  opciones={VOCES_OPENAI.map((v) => ({ valor: v.valor, texto: v.texto }))}
                  valor={ajustes.vozOpenai}
                  alCambiar={(vozOpenai) => cambiar({ vozOpenai })}
                />
                {campoKey("openai", "sk-…")}
              </>
            )}

            {proveedor === "local" && <Ayuda>Piper, en tu compu: anda sin internet y contesta al toque, pero suena más robótica.</Ayuda>}

            <div className="flex items-baseline gap-4">
              <BotonTexto onClick={() => void escuchar()} disabled={prueba.estado === "probando"} className="self-start">
                Escuchar
              </BotonTexto>
              <span aria-live="polite" className={`text-[14px] ${prueba.estado === "falla" ? "text-aviso" : "text-tinta-2"}`}>
                {prueba.estado === "probando" && "Preparando la voz…"}
                {prueba.estado === "anda" && textoDemora(prueba.milisegundos)}
                {prueba.estado === "falla" && prueba.mensaje}
                {prueba.estado === "nada" &&
                  (habla === proveedor
                    ? `Ahora habla con ${NOMBRE_PROVEEDOR[habla]}.`
                    : proveedorListo(proveedor, ajustes, conKey)
                      ? ""
                      : "Falta completar esto: mientras, habla la local.")}
              </span>
            </div>
          </div>

          <label className="flex flex-col gap-1">
            <span className="text-[13px] text-tinta-3">
              Velocidad al hablar · {ajustes.velocidad.toFixed(1).replace(".", ",")}
              {esHd && " (las voces HD hablan a su ritmo)"}
            </span>
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
            texto="Abrir con «Oye Nova» aunque el panel esté cerrado"
            valor={ajustes.palabraClave}
            alCambiar={(palabraClave) => cambiar({ palabraClave })}
          />
          {ajustes.palabraClave && (
            <Ayuda>El micrófono queda prendido y lo escucha la compu, nada sale a internet.</Ayuda>
          )}
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

function Ayuda({ children }: { children: ReactNode }) {
  return <p className="text-[13px] leading-snug text-tinta-3">{children}</p>;
}

interface CampoKeyProps {
  tiene: boolean;
  /** null = no se está cambiando. */
  valor: string | null;
  etiqueta: string;
  placeholder: string;
  alCambiar: (key: string | null) => void;
}

/** La key de un proveedor: si ya hay una, solo se dice eso (nunca se muestra). */
function CampoKey({ tiene, valor, etiqueta, placeholder, alCambiar }: CampoKeyProps) {
  if (tiene && valor === null) {
    return (
      <div className="flex flex-col gap-1">
        <span className="text-[13px] text-tinta-3">{etiqueta}</span>
        <div className="flex items-baseline gap-4 text-[15px]">
          <span className="text-tinta-2">Guardada en el llavero de Windows</span>
          <BotonTexto onClick={() => alCambiar("")}>Cambiar</BotonTexto>
        </div>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-1">
      <Campo
        etiqueta={etiqueta}
        type="password"
        placeholder={tiene ? "Pegá la nueva (vacía, se borra)" : placeholder}
        valor={valor ?? ""}
        alCambiar={alCambiar}
        maxLength={300}
      />
      <p className="text-[13px] leading-snug text-tinta-3">
        Queda en el llavero de Windows, nunca en un archivo.
        {tiene && (
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
