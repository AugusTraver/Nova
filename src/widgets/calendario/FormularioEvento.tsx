import { useState } from "react";
import {
  BorrarConConfirmacion,
  BotonTexto,
  Campo,
  Formulario,
  Interruptor,
  Opciones,
} from "../../components/Formulario";
import { desdeFecha, etiquetaDia } from "../../lib/fechas";
import {
  cambiarTipo,
  datosDesdeFormEvento,
  type FormEvento,
  formDesdeEvento,
  formEventoNuevo,
  TIPOS_CON_MATERIA,
} from "../../lib/formularios";
import type { Ocurrencia, Recurrencia, TipoEvento } from "../../lib/tipos";
import { useDatos } from "../../store/datos";
import { nombreTipoEvento } from "../../theme/eventos";

const TIPOS: TipoEvento[] = ["clase", "examen", "entrega", "cumpleanos", "partido", "otro"];

const REPETICIONES: { valor: Recurrencia; texto: string }[] = [
  { valor: "ninguna", texto: "No se repite" },
  { valor: "semanal", texto: "Cada semana" },
  { valor: "anual", texto: "Cada año" },
];

interface Props {
  /** La repetición que se tocó, o `null` para un evento nuevo. */
  ocurrencia: Ocurrencia | null;
  ahora: Date;
  alTerminar: () => void;
}

/**
 * Crear o editar un evento. Editar una repetición de una serie cambia toda la
 * serie; para "esta semana no hay clase" está "Saltear".
 */
export function FormularioEvento({ ocurrencia, ahora, alTerminar }: Props) {
  const evento = ocurrencia?.evento ?? null;
  const esNuevo = evento === null;
  const materias = useDatos((s) => s.materias);
  const guardarEvento = useDatos((s) => s.guardarEvento);
  const borrarEvento = useDatos((s) => s.borrarEvento);
  const saltearFecha = useDatos((s) => s.saltearFecha);

  const [form, setForm] = useState<FormEvento>(() => (evento ? formDesdeEvento(evento) : formEventoNuevo(ahora)));
  const [error, setError] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);

  const cambiar = <K extends keyof FormEvento>(clave: K, valor: FormEvento[K]) => setForm((f) => ({ ...f, [clave]: valor }));

  async function ejecutar(accion: () => Promise<string | null>) {
    setGuardando(true);
    const fallo = await accion();
    setGuardando(false);
    if (fallo) setError(fallo);
    else alTerminar();
  }

  function guardar() {
    const r = datosDesdeFormEvento(form);
    if (!r.ok) {
      setError(r.error);
      return;
    }
    void ejecutar(() => guardarEvento(evento?.id ?? null, r.datos));
  }

  const esSerie = evento !== null && evento.recurrencia !== "ninguna";
  const extra = evento && (
    <>
      {esSerie && ocurrencia && (
        <BotonTexto onClick={() => void ejecutar(() => saltearFecha(evento.id, ocurrencia.fecha))}>
          Saltear el {etiquetaDia(desdeFecha(ocurrencia.fecha), ahora).toLowerCase()}
        </BotonTexto>
      )}
      <BorrarConConfirmacion
        pregunta={esSerie ? "¿Borro toda la serie?" : "¿Lo borro?"}
        alConfirmar={() => void ejecutar(() => borrarEvento(evento.id))}
      />
    </>
  );

  return (
    <Formulario
      titulo={esNuevo ? "Nuevo evento" : esSerie ? "Editar la serie" : "Editar evento"}
      error={error}
      guardando={guardando}
      alGuardar={guardar}
      alCancelar={alTerminar}
      extra={extra}
    >
      <Campo
        etiqueta="Qué es"
        valor={form.titulo}
        alCambiar={(v) => cambiar("titulo", v)}
        placeholder="Clase de Análisis"
        autoFocus
        maxLength={200}
      />

      <Opciones
        etiqueta="Tipo"
        opciones={TIPOS.map((t) => ({ valor: t, texto: nombreTipoEvento[t] }))}
        valor={form.tipo}
        alCambiar={(t) => setForm((f) => cambiarTipo(f, t, esNuevo))}
      />

      <div className="grid grid-cols-[1fr_auto_auto] items-end gap-x-4 gap-y-3">
        <Campo etiqueta="Día" type="date" valor={form.fecha} alCambiar={(v) => cambiar("fecha", v)} />
        {form.todoElDia ? (
          <Campo
            etiqueta="Último día (opcional)"
            type="date"
            valor={form.fechaFin}
            alCambiar={(v) => cambiar("fechaFin", v)}
            min={form.fecha}
            className="col-span-2"
          />
        ) : (
          <>
            <Campo etiqueta="Desde" type="time" valor={form.horaInicio} alCambiar={(v) => cambiar("horaInicio", v)} className="w-[5.5rem]" />
            <Campo etiqueta="Hasta" type="time" valor={form.horaFin} alCambiar={(v) => cambiar("horaFin", v)} className="w-[5.5rem]" />
          </>
        )}
      </div>
      <Interruptor texto="Todo el día" valor={form.todoElDia} alCambiar={(v) => cambiar("todoElDia", v)} />

      <Opciones etiqueta="Repetición" opciones={REPETICIONES} valor={form.recurrencia} alCambiar={(v) => cambiar("recurrencia", v)} />
      {form.recurrencia !== "ninguna" && (
        <Campo
          etiqueta="Hasta cuándo (opcional)"
          type="date"
          valor={form.repetirHasta}
          alCambiar={(v) => cambiar("repetirHasta", v)}
          min={form.fecha}
        />
      )}

      {TIPOS_CON_MATERIA.includes(form.tipo) && materias.length > 0 && (
        <Opciones<number>
          etiqueta="Materia"
          opciones={[
            { valor: 0, texto: "Ninguna" },
            ...materias.map((m) => ({ valor: m.id, texto: m.nombre, color: `var(--color-materia-${m.color})` })),
          ]}
          valor={form.materiaId ?? 0}
          alCambiar={(id) => cambiar("materiaId", id === 0 ? null : id)}
        />
      )}

      {form.tipo === "cumpleanos" && (
        <Campo etiqueta="De quién" valor={form.persona} alCambiar={(v) => cambiar("persona", v)} placeholder="Juan" maxLength={80} />
      )}
    </Formulario>
  );
}
