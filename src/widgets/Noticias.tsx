import { noticias } from "../mocks/datos";

const VISIBLES = 3;

/** Tres titulares en fila, al pie del cielo. */
export function Noticias() {
  return (
    <section onClick={(e) => e.stopPropagation()} aria-label="Noticias">
      <ul className="grid grid-cols-3 gap-8">
        {noticias.slice(0, VISIBLES).map((n) => (
          // En v0.4 el clic abre la nota en el navegador
          <li key={n.id} className="fila -mx-2 px-2 py-1.5">
            <p className="line-clamp-2 text-[15px] leading-snug">{n.titulo}</p>
            <p className="mt-1 text-[13px] text-tinta-3">{n.fuente}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}
