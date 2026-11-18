import { Seccion } from "../components/Seccion";
import { noticias } from "../mocks/datos";

const VISIBLES = 3;

export function Noticias() {
  return (
    <Seccion titulo="Noticias" className="shrink-0">
      <ul className="flex flex-col gap-1">
        {noticias.slice(0, VISIBLES).map((n) => (
          // En v0.4 el clic abre la nota en el navegador
          <li key={n.id} className="fila -mx-2 px-2 py-1.5">
            <p className="line-clamp-2 text-[16px] leading-snug">{n.titulo}</p>
            <p className="mt-0.5 text-[13px] text-tinta-3">{n.fuente}</p>
          </li>
        ))}
      </ul>
    </Seccion>
  );
}
