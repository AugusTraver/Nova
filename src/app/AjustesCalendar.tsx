import { type KeyboardEvent, useEffect, useState } from "react";
import { BotonTexto, Campo } from "../components/Formulario";
import { api, mensajeDelAsistente } from "../lib/api";
import { estadoCalendar, textoPrimeraVez } from "../lib/google";
import type { VistaGoogle } from "../lib/tipos";
import { useDatos } from "../store/datos";

/**
 * Sección "Google Calendar" de Ajustes (v0.7.2, SPEC 4.9). Va por su cuenta,
 * sin esperar a "Guardar": conectar abre el navegador y desconectar le saca
 * el permiso a Nova en el momento. El ID, el secreto y el permiso se guardan
 * en el llavero de Windows y nunca vuelven a la interfaz.
 */
export function AjustesCalendar() {
  const recargar = useDatos((s) => s.recargar);
  const [google, setGoogle] = useState<VistaGoogle | null>(null);
  const [editandoCliente, setEditandoCliente] = useState(false);
  const [id, setId] = useState("");
  const [secreto, setSecreto] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [preguntando, setPreguntando] = useState(false);
  const [ahora, setAhora] = useState(() => new Date());

  useEffect(() => {
    api.obtenerGoogle().then(setGoogle, (e: unknown) => setAviso(mensajeDelAsistente(e)));
    // "Al día · hace 2 min" se mantiene al día mientras Ajustes está abierto.
    const reloj = window.setInterval(() => setAhora(new Date()), 30_000);
    return () => window.clearInterval(reloj);
  }, []);

  if (!google) return null;

  async function hacer(paso: () => Promise<VistaGoogle | void>) {
    setAviso(null);
    setOcupado(true);
    try {
      const vista = await paso();
      if (vista) setGoogle(vista);
    } catch (e) {
      setAviso(mensajeDelAsistente(e));
    } finally {
      setOcupado(false);
      setAhora(new Date());
    }
  }

  const guardarCliente = () =>
    hacer(async () => {
      const vista = await api.guardarClienteGoogle(id, secreto);
      setEditandoCliente(false);
      setId("");
      setSecreto("");
      return vista;
    });

  const conectar = () =>
    hacer(async () => {
      setGoogle({ ...google, conectando: true });
      try {
        const vista = await api.conectarGoogle();
        void recargar("agenda", "responsabilidades");
        return vista;
      } catch (e) {
        setGoogle((g) => (g ? { ...g, conectando: false } : g));
        throw e;
      }
    });

  const desconectar = () =>
    hacer(async () => {
      setPreguntando(false);
      const vista = await api.desconectarGoogle();
      void recargar("agenda");
      return vista;
    });

  const pedirCliente = !google.tieneCliente || editandoCliente;
  // Ajustes es un formulario: Enter en estos campos guarda el cliente, no todo Ajustes.
  const alApretar = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (id.trim() && secreto.trim() && !ocupado) void guardarCliente();
    }
  };

  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Google Calendar</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          Lo que agendás en Nova aparece en tu Google Calendar, y lo que cargás allá (desde el celular, por ejemplo)
          aparece acá. Las responsabilidades automáticas quedan solo en Nova.
        </p>
      </div>

      <p aria-live="polite" className="text-[15px] text-tinta-2">
        {estadoCalendar(google, ahora)}
      </p>

      {pedirCliente && !google.conectando && (
        <div className="flex flex-col gap-3">
          <Campo
            etiqueta="ID del cliente"
            valor={id}
            alCambiar={setId}
            placeholder="…apps.googleusercontent.com"
            maxLength={200}
            onKeyDown={alApretar}
          />
          <Campo
            etiqueta="Secreto del cliente"
            type="password"
            valor={secreto}
            alCambiar={setSecreto}
            placeholder="GOCSPX-…"
            maxLength={200}
            onKeyDown={alApretar}
          />
          <p className="text-[13px] leading-snug text-tinta-3">
            Están en console.cloud.google.com, en Google Auth Platform → Clientes → «Nova escritorio». Van al llavero de
            Windows, nunca a un archivo.
          </p>
          <div className="flex items-baseline gap-4">
            <BotonTexto tono="principal" disabled={ocupado || !id.trim() || !secreto.trim()} onClick={() => void guardarCliente()}>
              Guardar
            </BotonTexto>
            {editandoCliente && <BotonTexto onClick={() => setEditandoCliente(false)}>Dejar el que estaba</BotonTexto>}
          </div>
        </div>
      )}

      {google.conectando && (
        <div className="flex items-baseline gap-4">
          <p className="text-[13px] leading-snug text-tinta-3">
            Si Google dice «Google no verificó esta app», tocá «Avanzado» y después «Ir a Nova»: la app es tuya.
          </p>
          <BotonTexto onClick={() => void api.cancelarConexionGoogle()}>Cancelar</BotonTexto>
        </div>
      )}

      {!pedirCliente && !google.conectando && !google.conectado && (
        <div className="flex items-baseline gap-4">
          <BotonTexto tono="principal" disabled={ocupado} onClick={() => void conectar()}>
            Conectar con Google
          </BotonTexto>
          <BotonTexto onClick={() => setEditandoCliente(true)}>Cambiar el cliente</BotonTexto>
        </div>
      )}

      {google.conectado && !google.conectando && (
        <div className="flex flex-col gap-2">
          {google.primeraVez && <p className="text-[13px] text-tinta-3">{textoPrimeraVez(google.primeraVez)}</p>}
          {preguntando ? (
            <span className="flex flex-wrap items-baseline gap-3 text-[15px]" role="group" aria-label="¿Desconecto Google Calendar?">
              <span className="text-tinta-2">¿Desconecto? Tus eventos quedan en los dos lados.</span>
              <BotonTexto tono="peligro" className="text-aviso" autoFocus disabled={ocupado} onClick={() => void desconectar()}>
                Sí, desconectar
              </BotonTexto>
              <BotonTexto onClick={() => setPreguntando(false)}>No</BotonTexto>
            </span>
          ) : (
            <div className="flex items-baseline gap-4">
              <BotonTexto
                disabled={ocupado}
                onClick={() =>
                  void hacer(async () => {
                    await api.sincronizarGoogle();
                    setGoogle(await api.obtenerGoogle());
                  })
                }
              >
                Sincronizar ahora
              </BotonTexto>
              <BotonTexto tono="peligro" onClick={() => setPreguntando(true)}>
                Desconectar
              </BotonTexto>
            </div>
          )}
        </div>
      )}

      {aviso && (
        <p aria-live="polite" className="text-[13px] text-aviso">
          {aviso}
        </p>
      )}
    </section>
  );
}
