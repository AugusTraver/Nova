import { useState } from "react";
import { BotonTexto, Campo, Interruptor } from "../components/Formulario";
import { api, mensajeDelAsistente } from "../lib/api";
import { nombreDeCarpeta } from "../lib/compu";
import type { AjustesCompu as TipoAjustesCompu, PaginaGuardada } from "../lib/tipos";

interface Props {
  ajustes: TipoAjustesCompu;
  alCambiar: (ajustes: TipoAjustesCompu) => void;
  carpetasPorDefecto: string[];
  tieneKeyYoutube: boolean;
  /** `null` = no se toca; "" = borrarla al guardar; otro texto = la nueva. */
  keyNueva: string | null;
  alCambiarKey: (key: string | null) => void;
}

/**
 * Sección "La compu" de Ajustes (v0.7.1, SPEC 5.9): el interruptor, las
 * carpetas donde Nova puede tocar archivos, las páginas guardadas y la key de
 * YouTube. Las carpetas las valida Rust al guardar (no se aceptan las del
 * sistema).
 */
export function AjustesCompu({ ajustes, alCambiar, carpetasPorDefecto, tieneKeyYoutube, keyNueva, alCambiarKey }: Props) {
  const [aviso, setAviso] = useState<string | null>(null);
  const cambiar = (cambios: Partial<TipoAjustesCompu>) => alCambiar({ ...ajustes, ...cambios });
  const cambiarPagina = (i: number, cambios: Partial<PaginaGuardada>) =>
    cambiar({ paginas: ajustes.paginas.map((p, j) => (j === i ? { ...p, ...cambios } : p)) });

  async function agregarCarpeta() {
    setAviso(null);
    try {
      const elegida = await api.elegirCarpeta();
      if (elegida && !ajustes.carpetas.some((c) => c.toLowerCase() === elegida.toLowerCase())) {
        cambiar({ carpetas: [...ajustes.carpetas, elegida] });
      }
    } catch (e) {
      setAviso(mensajeDelAsistente(e));
    }
  }

  const mismasQueSiempre =
    carpetasPorDefecto.length > 0 &&
    carpetasPorDefecto.length === ajustes.carpetas.length &&
    carpetasPorDefecto.every((c) => ajustes.carpetas.includes(c));
  const mostrarKeyGuardada = tieneKeyYoutube && keyNueva === null;

  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">La compu</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          Nova abre páginas, videos, programas y archivos. Antes de cambiar, crear o borrar un archivo te muestra cómo
          queda y espera tu sí, y siempre lo podés deshacer.
        </p>
      </div>

      <Interruptor texto="Dejar que Nova use la compu" valor={ajustes.activa} alCambiar={(activa) => cambiar({ activa })} />

      {ajustes.activa && (
        <>
          <div className="flex flex-col gap-2">
            <span className="text-[13px] text-tinta-3">Carpetas donde puede abrir, leer y cambiar archivos</span>
            {ajustes.carpetas.length === 0 && (
              <p className="text-[14px] italic text-tinta-3">Ninguna: no va a tocar archivos.</p>
            )}
            <ul className="flex flex-col gap-1.5">
              {ajustes.carpetas.map((carpeta) => (
                <li key={carpeta} className="flex items-baseline justify-between gap-4">
                  <span className="min-w-0 truncate text-[15px] text-tinta" title={carpeta}>
                    {nombreDeCarpeta(carpeta)} <span className="text-[13px] text-tinta-3">{carpeta}</span>
                  </span>
                  <BotonTexto
                    tono="peligro"
                    aria-label={`Sacar ${carpeta}`}
                    onClick={() => cambiar({ carpetas: ajustes.carpetas.filter((c) => c !== carpeta) })}
                  >
                    Sacar
                  </BotonTexto>
                </li>
              ))}
            </ul>
            <div className="flex items-baseline gap-4">
              <BotonTexto onClick={() => void agregarCarpeta()}>Agregar una carpeta</BotonTexto>
              {!mismasQueSiempre && carpetasPorDefecto.length > 0 && (
                <BotonTexto onClick={() => cambiar({ carpetas: carpetasPorDefecto })}>Volver a las de siempre</BotonTexto>
              )}
            </div>
            {aviso && (
              <p aria-live="polite" className="text-[13px] text-aviso">
                {aviso}
              </p>
            )}
            <p className="text-[13px] leading-snug text-tinta-3">
              Nunca las de Windows, Program Files ni AppData. Lo que lee de un archivo lo toma como dato, no como órdenes.
            </p>
          </div>

          <div className="flex flex-col gap-2">
            <span className="text-[13px] text-tinta-3">Páginas guardadas («abrí la página de la facultad»)</span>
            {ajustes.paginas.map((pagina, i) => (
              <div key={i} className="grid grid-cols-[minmax(0,9rem)_minmax(0,1fr)_auto] items-end gap-4">
                <Campo
                  etiqueta="Nombre"
                  valor={pagina.nombre}
                  alCambiar={(nombre) => cambiarPagina(i, { nombre })}
                  placeholder="Facultad"
                  maxLength={60}
                />
                <Campo
                  etiqueta="Dirección"
                  valor={pagina.direccion}
                  alCambiar={(direccion) => cambiarPagina(i, { direccion })}
                  placeholder="campus.facultad.edu.ar"
                  maxLength={500}
                />
                <BotonTexto
                  tono="peligro"
                  aria-label={`Sacar ${pagina.nombre || "esta página"}`}
                  onClick={() => cambiar({ paginas: ajustes.paginas.filter((_, j) => j !== i) })}
                >
                  Sacar
                </BotonTexto>
              </div>
            ))}
            <BotonTexto className="w-fit" onClick={() => cambiar({ paginas: [...ajustes.paginas, { nombre: "", direccion: "" }] })}>
              Agregar una página
            </BotonTexto>
          </div>

          {mostrarKeyGuardada ? (
            <div className="flex flex-col gap-1">
              <span className="text-[13px] text-tinta-3">Key de YouTube</span>
              <div className="flex items-baseline gap-4 text-[15px]">
                <span className="text-tinta-2">Guardada en el llavero de Windows</span>
                <BotonTexto onClick={() => alCambiarKey("")}>Cambiar</BotonTexto>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-1">
              <Campo
                etiqueta="Key de YouTube (para abrir el primer video de una búsqueda)"
                type="password"
                placeholder={tieneKeyYoutube && keyNueva === "" ? "Pegá la nueva (vacía, se borra)" : "AIza…"}
                valor={keyNueva ?? ""}
                alCambiar={alCambiarKey}
                maxLength={100}
              />
              <p className="text-[13px] leading-snug text-tinta-3">
                Es gratis: en console.cloud.google.com, activá «YouTube Data API v3» y creá una clave de API. Alcanza para
                100 búsquedas por día. Sin key, Nova abre la lista de resultados.
                {tieneKeyYoutube && (
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
      )}
    </section>
  );
}
