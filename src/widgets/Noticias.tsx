import { Bloque } from "../components/Bloque";
import { noticias } from "../mocks/datos";

export function Noticias() {
  return (
    <Bloque etiqueta="NOTICIAS" className="flex-1">
      <ol>
        {noticias.map((n, i) => (
          // En v0.4 el clic abre la nota en el navegador
          <li key={n.id} className="fila -mx-2 flex gap-3 px-2 py-1.5">
            <span className="w-5 shrink-0 pt-px font-mono text-[11px] text-texto-3">
              {String(i + 1).padStart(2, "0")}
            </span>
            <div className="min-w-0">
              <p className="line-clamp-2 text-[14px] leading-snug">{n.titulo}</p>
              <p className="mt-0.5 truncate font-mono text-[11px] text-texto-3">{n.fuente.toUpperCase()}</p>
            </div>
          </li>
        ))}
      </ol>
    </Bloque>
  );
}
