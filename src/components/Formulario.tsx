// Piezas de formulario en estética "Atmósfera": sin cajas, texto sobre el
// cielo con un subrayado fino. Todas usan controles nativos (input, radio,
// checkbox, button) para que el teclado y los lectores de pantalla funcionen solos.
import { type ButtonHTMLAttributes, type FormEvent, type InputHTMLAttributes, type ReactNode, useId, useState } from "react";

const clasesEntrada =
  "w-full border-b border-tinta-4 bg-transparent pb-1.5 text-[16px] text-tinta caret-tinta-2 outline-none transition-colors duration-150 placeholder:text-tinta-3 focus:border-tinta-2 focus-visible:outline-none";

interface CampoProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange" | "value"> {
  etiqueta: string;
  valor: string;
  alCambiar: (valor: string) => void;
}

/** Campo de texto, fecha u hora con su etiqueta arriba. */
export function Campo({ etiqueta, valor, alCambiar, className = "", ...resto }: CampoProps) {
  const id = useId();
  return (
    <div className={`flex min-w-0 flex-col gap-1 ${className}`}>
      <label htmlFor={id} className="text-[13px] text-tinta-3">
        {etiqueta}
      </label>
      <input
        id={id}
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        spellCheck={false}
        autoComplete="off"
        className={clasesEntrada}
        {...resto}
      />
    </div>
  );
}

interface CampoLargoProps {
  etiqueta: string;
  valor: string;
  alCambiar: (valor: string) => void;
  /** Muestra "120 de 400" debajo. */
  maximo: number;
  placeholder?: string;
}

/** Texto de varias líneas, con el mismo subrayado que `Campo`. */
export function CampoLargo({ etiqueta, valor, alCambiar, maximo, placeholder }: CampoLargoProps) {
  const id = useId();
  const idCuenta = useId();
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <label htmlFor={id} className="text-[13px] text-tinta-3">
        {etiqueta}
      </label>
      <textarea
        id={id}
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        maxLength={maximo}
        rows={3}
        placeholder={placeholder}
        aria-describedby={idCuenta}
        className={`${clasesEntrada} resize-none leading-snug`}
      />
      <span id={idCuenta} className="self-end text-[12px] text-tinta-3">
        {valor.length} de {maximo}
      </span>
    </div>
  );
}

interface OpcionesProps<T extends string | number> {
  etiqueta: string;
  opciones: readonly { valor: T; texto: string; color?: string }[];
  valor: T;
  alCambiar: (valor: T) => void;
}

/**
 * Elegir una opción entre pocas, como texto: la elegida en tinta plena con un
 * punto adelante, las otras atenuadas. Por debajo son radios nativos.
 */
export function Opciones<T extends string | number>({ etiqueta, opciones, valor, alCambiar }: OpcionesProps<T>) {
  const nombre = useId();
  return (
    <fieldset className="flex min-w-0 flex-col gap-1.5">
      <legend className="mb-1 text-[13px] text-tinta-3">{etiqueta}</legend>
      <div className="flex flex-wrap gap-x-3.5 gap-y-1.5">
        {opciones.map((o) => {
          const elegida = o.valor === valor;
          return (
            <label
              key={String(o.valor)}
              className={`flex cursor-pointer items-center gap-1.5 rounded-md text-[15px] transition-colors duration-150 has-[:focus-visible]:outline has-[:focus-visible]:outline-[1.5px] has-[:focus-visible]:outline-offset-4 has-[:focus-visible]:outline-tinta-2 ${
                elegida ? "text-tinta" : "text-tinta-3 hover:text-tinta-2"
              }`}
            >
              <input
                type="radio"
                name={nombre}
                checked={elegida}
                onChange={() => alCambiar(o.valor)}
                className="sr-only"
              />
              <span
                aria-hidden
                className={`h-1.5 w-1.5 rounded-full transition-opacity duration-150 ${elegida ? "opacity-100" : "opacity-0"}`}
                style={{ backgroundColor: o.color ?? "var(--color-tinta)" }}
              />
              {o.texto}
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}

/** Un sí/no como texto: "Todo el día", con un círculo que se llena. */
export function Interruptor({ texto, valor, alCambiar }: { texto: string; valor: boolean; alCambiar: (v: boolean) => void }) {
  return (
    <label className="flex w-fit cursor-pointer items-center gap-2 rounded-md text-[15px] text-tinta-2 has-[:focus-visible]:outline has-[:focus-visible]:outline-[1.5px] has-[:focus-visible]:outline-offset-4 has-[:focus-visible]:outline-tinta-2">
      <input type="checkbox" checked={valor} onChange={(e) => alCambiar(e.target.checked)} className="sr-only" />
      <span
        aria-hidden
        className={`grid h-3.5 w-3.5 place-items-center rounded-full border-[1.5px] transition-colors duration-150 ${
          valor ? "border-tinta" : "border-tinta-3"
        }`}
      >
        <span className={`h-1.5 w-1.5 rounded-full bg-tinta transition-transform duration-150 ${valor ? "scale-100" : "scale-0"}`} />
      </span>
      {texto}
    </label>
  );
}

type Tono = "principal" | "secundario" | "peligro";

const clasesTono: Record<Tono, string> = {
  principal: "font-display text-[17px] italic text-tinta hover:text-tinta",
  secundario: "text-[15px] text-tinta-3 hover:text-tinta-2",
  peligro: "text-[15px] text-tinta-3 hover:text-aviso",
};

/** Botón de texto, sin caja. Se achica apenas al apretarlo. */
export function BotonTexto({
  tono = "secundario",
  className = "",
  ...resto
}: { tono?: Tono } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={`rounded-md transition-[color,transform] duration-150 active:scale-[0.97] disabled:opacity-50 ${clasesTono[tono]} ${className}`}
      {...resto}
    />
  );
}

/** "Agregar" al lado del título de un widget. */
export function AccionTitulo({ children, ...resto }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className="rounded-md text-[13px] text-tinta-3 transition-colors duration-150 hover:text-tinta-2 active:scale-[0.97]"
      {...resto}
    >
      {children}
    </button>
  );
}

/**
 * Borrar con confirmación en el lugar: primero "Borrar", después
 * "¿Seguro? Sí, borrar · No". Sin ventanas emergentes.
 */
export function BorrarConConfirmacion({ texto = "Borrar", pregunta, alConfirmar }: { texto?: string; pregunta: string; alConfirmar: () => void }) {
  const [preguntando, setPreguntando] = useState(false);
  if (!preguntando) {
    return (
      <BotonTexto tono="peligro" onClick={() => setPreguntando(true)}>
        {texto}
      </BotonTexto>
    );
  }
  return (
    <span className="flex items-baseline gap-3 text-[15px]" role="group" aria-label={pregunta}>
      <span className="text-tinta-2">{pregunta}</span>
      <BotonTexto tono="peligro" className="text-aviso" autoFocus onClick={alConfirmar}>
        Sí, borrar
      </BotonTexto>
      <BotonTexto onClick={() => setPreguntando(false)}>No</BotonTexto>
    </span>
  );
}

interface FormularioProps {
  titulo: string;
  /** Si hay un error, se muestra arriba de los botones. */
  error: string | null;
  guardando: boolean;
  alGuardar: () => void;
  alCancelar: () => void;
  /** Acciones extra a la derecha (borrar, saltear esta fecha). */
  extra?: ReactNode;
  children: ReactNode;
}

/**
 * Marco común de los formularios dentro de un widget: título en itálica,
 * campos, mensaje de error y Guardar / Cancelar. Enter guarda.
 */
export function Formulario({ titulo, error, guardando, alGuardar, alCancelar, extra, children }: FormularioProps) {
  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!guardando) alGuardar();
  };
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3.5" aria-label={titulo} noValidate>
      <p className="font-display text-[19px] italic text-tinta">{titulo}</p>
      {children}
      <p aria-live="polite" className="min-h-5 text-[14px] text-aviso">
        {error}
      </p>
      <div className="-mt-2 flex flex-wrap items-baseline gap-x-5 gap-y-2">
        <BotonTexto type="submit" tono="principal" disabled={guardando}>
          {guardando ? "Guardando…" : "Guardar"}
        </BotonTexto>
        <BotonTexto onClick={alCancelar}>Cancelar</BotonTexto>
        {extra && <span className="ml-auto flex items-baseline gap-4">{extra}</span>}
      </div>
    </form>
  );
}

/** Mensaje de un widget cuando sus datos no cargan. */
export function ErrorWidget({ mensaje, alReintentar }: { mensaje: string; alReintentar: () => void }) {
  return (
    <p className="text-[15px] text-tinta-2">
      {mensaje}{" "}
      <BotonTexto className="underline decoration-tinta-4 underline-offset-4" onClick={alReintentar}>
        Reintentar
      </BotonTexto>
    </p>
  );
}
