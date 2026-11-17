import { useEffect, useState } from "react";

/**
 * Fecha y hora actuales, actualizadas cada `intervaloMs`.
 * Se sincroniza con el cambio de segundo para que el reloj no vaya "corrido".
 */
export function useAhora(intervaloMs = 1000): Date {
  const [ahora, setAhora] = useState(() => new Date());

  useEffect(() => {
    let intervalo: number | undefined;
    const espera = intervaloMs - (Date.now() % intervaloMs);
    const arranque = window.setTimeout(() => {
      setAhora(new Date());
      intervalo = window.setInterval(() => setAhora(new Date()), intervaloMs);
    }, espera);
    return () => {
      window.clearTimeout(arranque);
      window.clearInterval(intervalo);
    };
  }, [intervaloMs]);

  return ahora;
}
