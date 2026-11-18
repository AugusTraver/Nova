import { useState } from "react";
import { BorrarConConfirmacion, BotonTexto, Campo, Formulario, Opciones } from "../../components/Formulario";
import { aFecha, sumarDias } from "../../lib/fechas";
import {
  datosDesdeFormResponsabilidad,
  type FormResponsabilidad,
  formDesdeResponsabilidad,
  formResponsabilidadNueva,
} from "../../lib/formularios";
import type { Prioridad, Responsabilidad } from "../../lib/tipos";
import { useDatos } from "../../store/datos";

const PRIORIDADES: { valor: Prioridad; texto: string }[] = [
  { valor: "baja", texto: "Baja" },
  { valor: "media", texto: "Media" },
  { valor: "alta", texto: "Alta" },
];

interface Props {
  responsabilidad: Responsabilidad | null;
  ahora: Date;
  alTerminar: () => void;
}

export function FormularioResponsabilidad({ responsabilidad, ahora, alTerminar }: Props) {
  const materias = useDatos((s) => s.materias);
  const guardar = useDatos((s) => s.guardarResponsabilidad);
  const borrar = useDatos((s) => s.borrarResponsabilidad);

  const [form, setForm] = useState<FormResponsabilidad>(() =>
    responsabilidad ? formDesdeResponsabilidad(responsabilidad) : formResponsabilidadNueva(),
  );
  const [error, setError] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);
  const cambiar = <K extends keyof FormResponsabilidad>(clave: K, valor: FormResponsabilidad[K]) =>
    setForm((f) => ({ ...f, [clave]: valor }));

  async function ejecutar(accion: () => Promise<string | null>) {
    setGuardando(true);
    const fallo = await accion();
    setGuardando(false);
    if (fallo) setError(fallo);
    else alTerminar();
  }

  function alGuardar() {
    const r = datosDesdeFormResponsabilidad(form, responsabilidad ?? undefined);
    if (!r.ok) {
      setError(r.error);
      return;
    }
    void ejecutar(() => guardar(responsabilidad?.id ?? null, r.datos));
  }

  const hoy = aFecha(ahora);
  const atajosFecha = [
    { texto: "Hoy", valor: hoy },
    { texto: "Mañana", valor: sumarDias(hoy, 1) },
    { texto: "Sin fecha", valor: "" },
  ];

  return (
    <Formulario
      titulo={responsabilidad ? "Editar" : "Algo para hacer"}
      error={error}
      guardando={guardando}
      alGuardar={alGuardar}
      alCancelar={alTerminar}
      extra={
        responsabilidad && (
          <BorrarConConfirmacion pregunta="¿La borro?" alConfirmar={() => void ejecutar(() => borrar(responsabilidad.id))} />
        )
      }
    >
      <Campo
        etiqueta="Qué tenés que hacer"
        valor={form.titulo}
        alCambiar={(v) => cambiar("titulo", v)}
        placeholder="Llamar al dentista"
        autoFocus
        maxLength={200}
      />

      <div className="flex flex-col gap-2">
        <Campo etiqueta="Para cuándo" type="date" valor={form.fechaLimite} alCambiar={(v) => cambiar("fechaLimite", v)} />
        <div className="flex gap-4">
          {atajosFecha.map((a) => (
            <BotonTexto
              key={a.texto}
              onClick={() => cambiar("fechaLimite", a.valor)}
              className={form.fechaLimite === a.valor ? "text-tinta" : ""}
              aria-pressed={form.fechaLimite === a.valor}
            >
              {a.texto}
            </BotonTexto>
          ))}
        </div>
      </div>

      <Opciones etiqueta="Prioridad" opciones={PRIORIDADES} valor={form.prioridad} alCambiar={(v) => cambiar("prioridad", v)} />

      {materias.length > 0 && (
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

      {responsabilidad?.origen === "auto" && (
        <p className="text-[13px] leading-snug text-tinta-3">
          La creó Nova a partir de un evento. Si la cambiás, deja de actualizarse sola.
        </p>
      )}
    </Formulario>
  );
}
