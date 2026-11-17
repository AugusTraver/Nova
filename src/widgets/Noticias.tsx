import { Tarjeta } from "../components/Tarjeta";
import { noticias } from "../mocks/datos";

export function Noticias() {
  return (
    <Tarjeta titulo="Noticias" className="flex-1">
      <ul className="flex flex-col gap-3">
        {noticias.map((n) => (
          <li key={n.id} className="text-sm">
            {/* En v0.4 el clic abre la nota en el navegador */}
            <p className="line-clamp-2 leading-snug">{n.titulo}</p>
            <p className="mt-0.5 truncate text-xs text-texto-tenue">
              {n.fuente} · {n.resumen}
            </p>
          </li>
        ))}
      </ul>
    </Tarjeta>
  );
}
