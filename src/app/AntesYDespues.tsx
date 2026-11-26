import type { LineaCambio, VistaCambio } from "../lib/tipos";

/**
 * El antes y después de un archivo (v0.7.1), debajo de la esfera mientras
 * espera el "sí". Sin recuadros ni letra de código: las líneas flotan sobre
 * el cielo como el resto del panel. Lo que sale va tenue y tachado; lo que
 * entra, con el resalte suave de la luz de la esfera.
 */
export function AntesYDespues({ vista }: { vista: VistaCambio }) {
  return (
    <figure className="mt-2.5 w-full text-left">
      <figcaption className="mb-2 text-center text-[13px] text-tinta-3">
        {vista.archivo}
        {vista.carpeta && <> · en {vista.carpeta}</>}
      </figcaption>
      {vista.lineas.length > 0 ? (
        <div className="flex max-h-[13rem] flex-col gap-1 overflow-y-auto text-[14.5px] leading-snug">
          {vista.lineas.map((linea, i) => (
            <Linea key={i} linea={linea} />
          ))}
        </div>
      ) : (
        <p className="text-center text-[14px] italic text-tinta-3">No es un texto que pueda mostrarte.</p>
      )}
      {vista.mas > 0 && (
        <p className="mt-1.5 text-center text-[13px] text-tinta-3">
          y {vista.mas} {vista.mas === 1 ? "línea" : "líneas"} más
        </p>
      )}
      {vista.nota && <p className="mt-1.5 text-center text-[13px] text-aviso">{vista.nota}</p>}
    </figure>
  );
}

function Vacia() {
  return <span className="italic text-tinta-3">(línea vacía)</span>;
}

function Linea({ linea }: { linea: LineaCambio }) {
  const { tipo, inicio, marca, fin } = linea;
  const texto = "whitespace-pre-wrap break-words";
  switch (tipo) {
    case "corte":
      return (
        <p className="py-0.5 pl-3 text-tinta-4" aria-label="Líneas sin cambios">
          <span aria-hidden>· · ·</span>
        </p>
      );
    case "igual":
      return <p className={`${texto} pl-3 text-tinta-3`}>{inicio || " "}</p>;
    case "sale":
      return (
        <p className={`${texto} border-l-2 border-tinta-4 pl-2.5 text-tinta-3`}>
          <span className="sr-only">Antes: </span>
          {inicio}
          {marca || !(inicio || fin) ? (
            <del className="decoration-tinta-3 decoration-1">{marca || <Vacia />}</del>
          ) : null}
          {fin}
        </p>
      );
    case "entra":
      return (
        <p className={`${texto} border-l-2 border-resalte pl-2.5 text-tinta`}>
          <span className="sr-only">Después: </span>
          {inicio}
          {marca || !(inicio || fin) ? (
            <ins className="rounded-[3px] bg-resalte px-[3px] no-underline">{marca || <Vacia />}</ins>
          ) : null}
          {fin}
        </p>
      );
  }
}
