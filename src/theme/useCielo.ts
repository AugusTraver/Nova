import { useEffect, useState } from "react";
import { esMomento, type Momento, momentoDelDia } from "./cielo";

/**
 * Pone el cielo que corresponde a la hora en `<html data-cielo="...">`
 * (tokens.css cambia los colores según ese atributo) y lo devuelve.
 *
 * Se recalcula al abrir el panel y una vez por minuto mientras está abierto.
 * Para probar otros momentos se puede forzar con `?cielo=tarde` en la URL.
 */
export function useCielo(abierto: boolean): Momento {
  const [momento, setMomento] = useState<Momento>(() => calcular());

  useEffect(() => {
    if (!abierto) return;
    setMomento(calcular());
    const intervalo = window.setInterval(() => setMomento(calcular()), 60_000);
    return () => window.clearInterval(intervalo);
  }, [abierto]);

  useEffect(() => {
    document.documentElement.dataset.cielo = momento;
  }, [momento]);

  return momento;
}

function calcular(): Momento {
  const forzado = new URLSearchParams(window.location.search).get("cielo");
  return esMomento(forzado) ? forzado : momentoDelDia(new Date());
}
