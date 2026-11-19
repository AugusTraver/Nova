import { Campo, CampoLargo, Interruptor, Opciones } from "../components/Formulario";
import { LARGO_SOBRE_VOS, type Perfil, type Tono } from "../lib/tipos";

const TONOS: { valor: Tono; texto: string }[] = [
  { valor: "con_onda", texto: "Con onda" },
  { valor: "informal", texto: "Bien informal" },
  { valor: "sobrio", texto: "Más tranqui" },
];

/** "Sobre vos": cómo te llama Nova, cómo te habla y lo que querés que sepa. */
export function AjustesPerfil({ perfil, alCambiar }: { perfil: Perfil; alCambiar: (p: Perfil) => void }) {
  const cambiar = (cambios: Partial<Perfil>) => alCambiar({ ...perfil, ...cambios });
  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Sobre vos</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          Nova lo tiene en cuenta en cada respuesta. Se queda en tu compu y solo viaja con lo que le preguntás.
        </p>
      </div>
      <Campo
        etiqueta="Cómo querés que te llame"
        valor={perfil.comoLlamarte}
        alCambiar={(comoLlamarte) => cambiar({ comoLlamarte })}
        maxLength={40}
        className="max-w-xs"
      />
      <Opciones etiqueta="Cómo te habla" opciones={TONOS} valor={perfil.tono} alCambiar={(tono) => cambiar({ tono })} />
      {perfil.tono !== "sobrio" && (
        <Interruptor texto="Algún que otro chiste" valor={perfil.chistes} alCambiar={(chistes) => cambiar({ chistes })} />
      )}
      <CampoLargo
        etiqueta="Lo que querés que sepa"
        valor={perfil.sobreVos}
        alCambiar={(sobreVos) => cambiar({ sobreVos })}
        maximo={LARGO_SOBRE_VOS}
        placeholder="Qué estudiás, dónde, lo que quieras que tenga en cuenta…"
      />
    </section>
  );
}
