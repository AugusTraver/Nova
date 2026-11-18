import { useId, useState } from "react";
import { BorrarConConfirmacion, Campo, Formulario } from "../../components/Formulario";
import { type ColorMateria, type Materia, PALETA_MATERIAS } from "../../lib/tipos";
import { useDatos } from "../../store/datos";

const NOMBRE_COLOR: Record<ColorMateria, string> = {
  celeste: "Celeste",
  coral: "Coral",
  menta: "Menta",
  lavanda: "Lavanda",
  ambar: "Ámbar",
  rosa: "Rosa",
};

interface Props {
  materia: Materia | null;
  alTerminar: () => void;
}

export function FormularioMateria({ materia, alTerminar }: Props) {
  const materias = useDatos((s) => s.materias);
  const guardar = useDatos((s) => s.guardarMateria);
  const borrar = useDatos((s) => s.borrarMateria);
  // Para una nueva, el primer color que nadie usa todavía.
  const [color, setColor] = useState<ColorMateria>(
    () => materia?.color ?? PALETA_MATERIAS.find((c) => !materias.some((m) => m.color === c)) ?? "celeste",
  );
  const [nombre, setNombre] = useState(materia?.nombre ?? "");
  const [error, setError] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);
  const grupo = useId();

  async function ejecutar(accion: () => Promise<string | null>) {
    setGuardando(true);
    const fallo = await accion();
    setGuardando(false);
    if (fallo) setError(fallo);
    else alTerminar();
  }

  function alGuardar() {
    if (!nombre.trim()) {
      setError("¿Cómo se llama la materia?");
      return;
    }
    void ejecutar(() => guardar(materia?.id ?? null, { nombre: nombre.trim(), color }));
  }

  return (
    <Formulario
      titulo={materia ? "Editar materia" : "Nueva materia"}
      error={error}
      guardando={guardando}
      alGuardar={alGuardar}
      alCancelar={alTerminar}
      extra={
        materia && (
          <BorrarConConfirmacion
            pregunta="¿La borro? Sus eventos quedan."
            alConfirmar={() => void ejecutar(() => borrar(materia.id))}
          />
        )
      }
    >
      <Campo etiqueta="Nombre" valor={nombre} alCambiar={setNombre} placeholder="Análisis Matemático II" autoFocus maxLength={80} />

      <fieldset>
        <legend className="mb-2 text-[13px] text-tinta-3">Color</legend>
        <div className="flex gap-3">
          {PALETA_MATERIAS.map((c) => (
            <label
              key={c}
              title={NOMBRE_COLOR[c]}
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-full has-[:focus-visible]:outline has-[:focus-visible]:outline-[1.5px] has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-tinta-2"
            >
              <input
                type="radio"
                name={grupo}
                checked={color === c}
                onChange={() => setColor(c)}
                className="sr-only"
                aria-label={NOMBRE_COLOR[c]}
              />
              <span
                aria-hidden
                className={`rounded-full transition-[width,height,box-shadow] duration-150 ${
                  color === c ? "h-5 w-5 shadow-[0_0_0_2px_var(--color-tinta-2)]" : "h-3.5 w-3.5"
                }`}
                style={{ backgroundColor: `var(--color-materia-${c})` }}
              />
            </label>
          ))}
        </div>
      </fieldset>
    </Formulario>
  );
}
