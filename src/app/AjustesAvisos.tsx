import { BotonTexto, Campo, Interruptor, Opciones } from "../components/Formulario";
import { api } from "../lib/api";
import type { AjustesAvisos as TipoAjustesAvisos } from "../lib/tipos";

const MINUTOS_ANTES = [10, 15, 30, 60].map((m) => ({ valor: m, texto: m === 60 ? "1 hora" : `${m} min` }));

/** Avisos del sistema: qué notificar, cuándo, y el horario de "no molestar". */
export function AjustesAvisos({ avisos, alCambiar }: { avisos: TipoAjustesAvisos; alCambiar: (a: TipoAjustesAvisos) => void }) {
  const cambiar = (cambios: Partial<TipoAjustesAvisos>) => alCambiar({ ...avisos, ...cambios });
  return (
    <section className="mt-2 flex flex-col gap-4">
      <div>
        <h3 className="font-display text-[17px] italic text-tinta-2">Avisos</h3>
        <p className="mt-1 text-[13px] leading-snug text-tinta-3">
          Te aviso con una notificación de Windows aunque el panel esté cerrado. Si la tocás, se abre.{" "}
          <BotonTexto onClick={() => void api.probarAviso()}>Probar</BotonTexto>
        </p>
      </div>

      <Interruptor texto="Antes de cada evento" valor={avisos.eventos} alCambiar={(eventos) => cambiar({ eventos })} />
      {avisos.eventos && (
        <Opciones
          etiqueta="Cuánto antes"
          opciones={MINUTOS_ANTES}
          valor={avisos.minutosAntes}
          alCambiar={(minutosAntes) => cambiar({ minutosAntes })}
        />
      )}

      <Interruptor
        texto="Cuándo salir para la primera clase o examen del día"
        valor={avisos.salir}
        alCambiar={(salir) => cambiar({ salir })}
      />
      {avisos.salir && (
        <Campo
          etiqueta="Cuántos minutos tardás en llegar"
          type="number"
          min={5}
          max={240}
          step={5}
          valor={String(avisos.minutosViaje)}
          alCambiar={(v) => cambiar({ minutosViaje: Number(v) || 0 })}
          className="max-w-[10rem]"
        />
      )}

      <Interruptor
        texto="Lo que vence en el día, todo junto"
        valor={avisos.vencimientos}
        alCambiar={(vencimientos) => cambiar({ vencimientos })}
      />
      {avisos.vencimientos && (
        <Campo
          etiqueta="A qué hora"
          type="time"
          valor={avisos.horaVencimientos}
          alCambiar={(horaVencimientos) => cambiar({ horaVencimientos })}
          className="max-w-[10rem]"
        />
      )}

      <Interruptor
        texto="Cuando arranca Boca o la Selección"
        valor={avisos.partidos}
        alCambiar={(partidos) => cambiar({ partidos })}
      />
      <Interruptor
        texto="Además, que Nova lo diga en voz alta"
        valor={avisos.enVoz}
        alCambiar={(enVoz) => cambiar({ enVoz })}
      />

      <Interruptor texto="No molestar" valor={avisos.noMolestar} alCambiar={(noMolestar) => cambiar({ noMolestar })} />
      {avisos.noMolestar && (
        <>
          <div className="flex gap-6">
            <Campo
              etiqueta="Desde"
              type="time"
              valor={avisos.desde}
              alCambiar={(desde) => cambiar({ desde })}
              className="max-w-[8rem]"
            />
            <Campo
              etiqueta="Hasta"
              type="time"
              valor={avisos.hasta}
              alCambiar={(hasta) => cambiar({ hasta })}
              className="max-w-[8rem]"
            />
          </div>
          <Interruptor
            texto="En ese horario tampoco escuchar «Oye Nova»"
            valor={avisos.noMolestarPausaPalabra}
            alCambiar={(noMolestarPausaPalabra) => cambiar({ noMolestarPausaPalabra })}
          />
          <p className="text-[13px] leading-snug text-tinta-3">
            Lo que sigue valiendo cuando termina te llega en ese momento; lo que ya pasó, no.
          </p>
        </>
      )}
    </section>
  );
}
