import { useEffect, useState } from "react";
import { BotonTexto, Formulario, Interruptor } from "../components/Formulario";
import { api, mensajeDeError, mensajeDelAsistente } from "../lib/api";
import { atajoDesdeTecla, nombreAtajo } from "../lib/atajo";
import type { AjustesIa, Ajustes as TipoAjustes, Regla, Reglas } from "../lib/tipos";
import { useDatos } from "../store/datos";
import { AjustesAsistente } from "./AjustesAsistente";

const REGLAS: { clave: keyof Reglas; texto: string; ayuda: string }[] = [
  { clave: "examen", texto: "Antes de un examen", ayuda: "{materia} es el nombre de la materia" },
  { clave: "entrega", texto: "Antes de una entrega", ayuda: "{entrega} es el título de la entrega" },
  { clave: "cumpleanos", texto: "Antes de un cumpleaños", ayuda: "{persona} es de quién es el cumple" },
];

/** Ajustes: el atajo, las reglas de responsabilidades automáticas y el asistente. */
export function Ajustes({ alTerminar }: { alTerminar: () => void }) {
  const recargar = useDatos((s) => s.recargar);
  const [ajustes, setAjustes] = useState<TipoAjustes | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);
  const [capturando, setCapturando] = useState(false);
  const [ia, setIa] = useState<AjustesIa | null>(null);
  const [tieneKey, setTieneKey] = useState(false);
  const [keyNueva, setKeyNueva] = useState<string | null>(null);

  useEffect(() => {
    api.obtenerAjustes().then(setAjustes, (e: unknown) => setError(mensajeDeError(e)));
    api.obtenerAjustesIa().then(
      (vista) => {
        setIa(vista.ajustes);
        setTieneKey(vista.tieneKey);
      },
      (e: unknown) => setError(mensajeDelAsistente(e)),
    );
  }, []);

  /** Guarda proveedor, modelo y key (la key va derecho al llavero). */
  async function guardarIa() {
    if (!ia) return;
    const vista = await api.guardarAjustesIa(ia);
    setIa(vista.ajustes);
    if (keyNueva !== null) {
      setTieneKey(await api.guardarApiKey(keyNueva));
      setKeyNueva(null);
    }
  }

  // Mientras se elige el atajo, la próxima combinación con Ctrl/Alt/Shift
  // queda como nuevo atajo. Esc cancela. Se escucha en la fase de captura y se
  // frena el evento para que no lo use nada más (ni el Esc que cierra el panel).
  useEffect(() => {
    if (!capturando) return;
    const alTeclear = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setCapturando(false);
        return;
      }
      const atajo = atajoDesdeTecla(e);
      if (atajo) {
        setAjustes((a) => (a ? { ...a, atajo } : a));
        setCapturando(false);
      }
    };
    window.addEventListener("keydown", alTeclear, { capture: true });
    return () => window.removeEventListener("keydown", alTeclear, { capture: true });
  }, [capturando]);

  const cambiarRegla = (clave: keyof Reglas, cambios: Partial<Regla>) =>
    setAjustes((a) => (a ? { ...a, reglas: { ...a.reglas, [clave]: { ...a.reglas[clave], ...cambios } } } : a));

  async function guardar() {
    if (!ajustes) return;
    setGuardando(true);
    setError(null);
    try {
      await api.guardarAjustes(ajustes);
      await guardarIa();
      // Las reglas pueden haber creado o borrado automáticas
      await recargar("responsabilidades");
      alTerminar();
    } catch (e) {
      setError(mensajeDelAsistente(e));
    } finally {
      setGuardando(false);
    }
  }

  if (!ajustes) {
    return (
      <p className="text-[15px] text-tinta-3">
        {error ?? "Cargando ajustes…"}{" "}
        {error && <BotonTexto onClick={alTerminar}>Volver</BotonTexto>}
      </p>
    );
  }

  return (
    <Formulario titulo="Ajustes" error={error} guardando={guardando} alGuardar={() => void guardar()} alCancelar={alTerminar}>
      <section className="flex flex-col gap-1.5">
        <h3 className="text-[13px] text-tinta-3">Atajo para abrir y cerrar Nova</h3>
        <div className="flex items-baseline gap-4">
          <span aria-live="polite" className="font-display text-[22px] font-[350]">
            {capturando ? <span className="italic text-tinta-2">Apretá la combinación…</span> : nombreAtajo(ajustes.atajo)}
          </span>
          <BotonTexto onClick={() => setCapturando((c) => !c)}>{capturando ? "Cancelar" : "Cambiar"}</BotonTexto>
        </div>
        <p className="text-[13px] text-tinta-3">Tiene que llevar Ctrl, Alt o Shift.</p>
      </section>

      <section className="mt-2 flex flex-col gap-4">
        <div>
          <h3 className="font-display text-[17px] italic text-tinta-2">Responsabilidades automáticas</h3>
          <p className="mt-1 text-[13px] leading-snug text-tinta-3">
            Cuando cargás un examen, una entrega o un cumpleaños, Nova te anota qué hacer antes.
          </p>
        </div>
        {REGLAS.map(({ clave, texto, ayuda }) => {
          const regla = ajustes.reglas[clave];
          return (
            <div key={clave} className="flex flex-col gap-2">
              <Interruptor texto={texto} valor={regla.activa} alCambiar={(activa) => cambiarRegla(clave, { activa })} />
              {regla.activa && (
                <div className="grid grid-cols-[1fr_auto] items-end gap-4 pl-[1.375rem]">
                  <label className="flex min-w-0 flex-col gap-1">
                    <span className="text-[13px] text-tinta-3">Título · {ayuda}</span>
                    <input
                      value={regla.plantilla}
                      onChange={(e) => cambiarRegla(clave, { plantilla: e.target.value })}
                      spellCheck={false}
                      maxLength={120}
                      className="w-full border-b border-tinta-4 bg-transparent pb-1.5 text-[16px] text-tinta outline-none focus:border-tinta-2 focus-visible:outline-none"
                    />
                  </label>
                  <label className="flex flex-col gap-1">
                    <span className="text-[13px] text-tinta-3">Días antes</span>
                    <input
                      type="number"
                      min={0}
                      max={60}
                      value={regla.diasAntes}
                      onChange={(e) => cambiarRegla(clave, { diasAntes: Math.max(0, Math.min(60, Number(e.target.value) || 0)) })}
                      className="w-16 border-b border-tinta-4 bg-transparent pb-1.5 text-[16px] text-tinta outline-none focus:border-tinta-2 focus-visible:outline-none"
                    />
                  </label>
                </div>
              )}
            </div>
          );
        })}
      </section>

      {ia && (
        <AjustesAsistente
          ia={ia}
          alCambiar={setIa}
          tieneKey={tieneKey}
          keyNueva={keyNueva}
          alCambiarKey={setKeyNueva}
          guardarIa={guardarIa}
        />
      )}
    </Formulario>
  );
}
