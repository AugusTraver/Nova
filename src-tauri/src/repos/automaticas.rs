//! Sincronización de responsabilidades automáticas con sus eventos.
//!
//! Reglas acordadas para v0.2:
//! * Cada automática queda atada a su evento, a la regla que la creó y a la
//!   fecha de la repetición (`evento_fecha`).
//! * Si el evento cambia, las automáticas pendientes que no tocaste se
//!   actualizan (título, fecha límite, prioridad, materia).
//! * Si ya no corresponden (cambió el tipo, se apagó la regla, el evento pasó),
//!   las pendientes que no tocaste se borran.
//! * Las hechas, las editadas a mano y las descartadas nunca se tocan.
//! * En los eventos que se repiten (cumpleaños), solo existe la de la próxima
//!   repetición.
//!
//! Sincronizar se puede repetir cuantas veces haga falta sin duplicar nada.

use rusqlite::{Connection, OptionalExtension, params};

use super::eventos::{self, Evento};
use super::{Ahora, fecha, preferencias, texto_fecha};
use crate::dominio::recurrencia;
use crate::dominio::reglas::{self, DatosEvento, Deseada, Reglas};
use crate::dominio::texto::{EstadoResponsabilidad, OrigenResponsabilidad, Recurrencia, TipoRegla};
use crate::error::Resultado;

/// Una automática que ya está en la base.
struct Existente {
    id: i64,
    regla: TipoRegla,
    evento_fecha: Option<String>,
    estado: EstadoResponsabilidad,
    editada_a_mano: bool,
    descartada: bool,
    creado_en: String,
}

impl Existente {
    /// La sincronización solo puede cambiar o borrar las que siguen "intactas".
    fn intacta(&self) -> bool {
        self.estado == EstadoResponsabilidad::Pendiente && !self.editada_a_mano && !self.descartada
    }
}

fn existentes(conexion: &Connection, evento_id: i64) -> Resultado<Vec<Existente>> {
    let mut consulta = conexion.prepare(
        "SELECT id, regla, evento_fecha, estado, editada_a_mano, descartada, creado_en
         FROM responsabilidades WHERE evento_id = ?1 AND regla IS NOT NULL ORDER BY id",
    )?;
    let lista = consulta
        .query_map([evento_id], |f| {
            Ok(Existente {
                id: f.get(0)?,
                regla: f.get(1)?,
                evento_fecha: f.get(2)?,
                estado: f.get(3)?,
                editada_a_mano: f.get(4)?,
                descartada: f.get(5)?,
                creado_en: f.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(lista)
}

/// Qué automática debería tener hoy este evento (si alguna).
fn deseada_para(conexion: &Connection, evento: &Evento, reglas: &Reglas, ahora: &Ahora) -> Resultado<Option<Deseada>> {
    let materia: Option<String> = match evento.materia_id {
        Some(id) => conexion
            .query_row("SELECT nombre FROM materias WHERE id = ?1", [id], |f| f.get(0))
            .optional()?,
        None => None,
    };
    let proxima = evento.con_serie(|serie| recurrencia::proxima(serie, ahora.hoy))?;
    let datos = DatosEvento {
        tipo: evento.tipo,
        titulo: &evento.titulo,
        persona: evento.persona.as_deref(),
        materia: materia.as_deref(),
    };
    Ok(reglas::deseada(&datos, proxima.map(|r| r.fecha()), reglas))
}

/// Pone al día las automáticas de un evento. Se llama dentro de la misma
/// transacción que creó o editó el evento.
pub fn sincronizar_evento(conexion: &Connection, evento: &Evento, ahora: &Ahora) -> Resultado<()> {
    let reglas = preferencias::reglas(conexion)?;
    sincronizar_con(conexion, evento, &reglas, ahora)
}

fn sincronizar_con(conexion: &Connection, evento: &Evento, reglas: &Reglas, ahora: &Ahora) -> Resultado<()> {
    let deseada = deseada_para(conexion, evento, reglas, ahora)?;
    let existentes = existentes(conexion, evento.id)?;
    let es_serie = evento.recurrencia != Recurrencia::Ninguna;

    // ¿Cuál de las existentes es "la misma" que la deseada? En una serie, la de
    // la misma repetición; en un evento común, cualquiera de la misma regla
    // (así, si movés la fecha del parcial, se mueve la que ya estaba).
    let coincidente = deseada.as_ref().and_then(|d| {
        let fecha_deseada = texto_fecha(d.evento_fecha);
        existentes
            .iter()
            .find(|e| e.regla == d.regla && (!es_serie || e.evento_fecha.as_deref() == Some(fecha_deseada.as_str())))
    });

    if let Some(d) = &deseada {
        match coincidente {
            Some(e) if e.intacta() => {
                let piso = fecha(&e.creado_en).unwrap_or(ahora.hoy);
                conexion.execute(
                    "UPDATE responsabilidades
                     SET titulo = ?1, fecha_limite = ?2, prioridad = ?3, materia_id = ?4, evento_fecha = ?5,
                         actualizado_en = ?6
                     WHERE id = ?7",
                    params![
                        d.titulo,
                        texto_fecha(reglas::fecha_limite(d.fecha_limite_ideal, piso)),
                        d.prioridad,
                        evento.materia_id,
                        texto_fecha(d.evento_fecha),
                        ahora.marca,
                        e.id
                    ],
                )?;
            }
            Some(_) => {} // hecha, editada o descartada: no se toca
            None => {
                conexion.execute(
                    "INSERT INTO responsabilidades (titulo, fecha_limite, prioridad, evento_id, materia_id, origen,
                                                    regla, evento_fecha, creado_en, actualizado_en)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![
                        d.titulo,
                        texto_fecha(reglas::fecha_limite(d.fecha_limite_ideal, ahora.hoy)),
                        d.prioridad,
                        evento.id,
                        evento.materia_id,
                        OrigenResponsabilidad::Auto,
                        d.regla,
                        texto_fecha(d.evento_fecha),
                        ahora.marca
                    ],
                )?;
            }
        }
    }

    // Las intactas que sobran (no son la deseada) ya no corresponden.
    let conservar = coincidente.map(|e| e.id);
    for e in existentes.iter().filter(|e| e.intacta() && Some(e.id) != conservar) {
        conexion.execute("DELETE FROM responsabilidades WHERE id = ?1", [e.id])?;
    }
    Ok(())
}

/// Pone al día todas. Se llama al abrir el panel (para que los cumpleaños
/// pasen al año siguiente) y al cambiar las reglas.
pub fn sincronizar_todo(conexion: &mut Connection, ahora: &Ahora) -> Resultado<()> {
    let tx = conexion.transaction()?;
    let reglas = preferencias::reglas(&tx)?;
    for evento in eventos::listar_todos(&tx)? {
        sincronizar_con(&tx, &evento, &reglas, ahora)?;
    }
    tx.commit()?;
    Ok(())
}

/// Antes de borrar un evento: se van sus automáticas intactas y las
/// descartadas. Las hechas y las editadas quedan, sin evento (`ON DELETE SET NULL`).
pub fn al_borrar_evento(conexion: &Connection, evento_id: i64) -> Resultado<()> {
    conexion.execute(
        "DELETE FROM responsabilidades
         WHERE evento_id = ?1 AND regla IS NOT NULL
           AND (descartada = 1 OR (estado = 'pendiente' AND editada_a_mano = 0))",
        [evento_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::dominio::texto::{Prioridad, TipoEvento};
    use crate::repos::eventos::tests::datos as evento;
    use crate::repos::materias::{self, DatosMateria};
    use crate::repos::responsabilidades::{self as resp, Responsabilidad};

    fn pendientes(base: &Connection) -> Vec<Responsabilidad> {
        resp::listar(base).unwrap()
    }

    fn resumen(base: &Connection) -> Vec<String> {
        pendientes(base)
            .into_iter()
            .map(|r| format!("{} | {}", r.titulo, r.fecha_limite.unwrap_or_default()))
            .collect()
    }

    #[test]
    fn crear_un_examen_crea_estudiar() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let fisica = materias::crear(&base, DatosMateria { nombre: "Física".into(), color: "menta".into() }, &ahora).unwrap();
        let mut parcial = evento("Primer parcial", TipoEvento::Examen, "2026-11-20T09:00:00-03:00");
        parcial.materia_id = Some(fisica.id);
        let parcial = eventos::crear(&mut base, parcial, &ahora).unwrap();

        let lista = pendientes(&base);
        assert_eq!(lista.len(), 1);
        let r = &lista[0];
        assert_eq!(r.titulo, "Estudiar para Física");
        assert_eq!(r.fecha_limite.as_deref(), Some("2026-11-19"));
        assert_eq!(r.origen, OrigenResponsabilidad::Auto);
        assert_eq!(r.prioridad, Prioridad::Alta);
        assert_eq!(r.evento_id, Some(parcial.id));
        assert_eq!(r.materia_id, Some(fisica.id));
    }

    #[test]
    fn mover_el_evento_mueve_la_automatica_intacta() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let tp = eventos::crear(&mut base, evento("TP 2", TipoEvento::Entrega, "2026-11-20"), &ahora).unwrap();
        let id_antes = pendientes(&base)[0].id;

        eventos::editar(&mut base, tp.id, evento("TP 2 final", TipoEvento::Entrega, "2026-11-27"), &ahora).unwrap();
        let lista = pendientes(&base);
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].id, id_antes, "es la misma fila, no una nueva");
        assert_eq!(resumen(&base), ["Terminar TP 2 final | 2026-11-26"]);
    }

    #[test]
    fn la_editada_a_mano_no_se_toca() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let tp = eventos::crear(&mut base, evento("TP 2", TipoEvento::Entrega, "2026-11-20"), &ahora).unwrap();
        let auto = pendientes(&base)[0].clone();
        let mut cambios = resp::tests::datos("Terminar el informe del TP", Some("2026-11-18"));
        cambios.evento_id = auto.evento_id; // la interfaz manda de vuelta el vínculo
        resp::editar(&base, auto.id, cambios, &ahora).unwrap();

        eventos::editar(&mut base, tp.id, evento("TP 2", TipoEvento::Entrega, "2026-11-27"), &ahora).unwrap();
        assert_eq!(resumen(&base), ["Terminar el informe del TP | 2026-11-18"], "ni se cambia ni se duplica");

        // Si se borra el evento, la editada queda, sin evento
        eventos::borrar(&mut base, tp.id).unwrap();
        let lista = pendientes(&base);
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].evento_id, None);
    }

    #[test]
    fn la_hecha_queda_y_no_se_regenera() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let parcial = eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        let auto = pendientes(&base)[0].clone();
        resp::marcar(&base, auto.id, true, &ahora).unwrap();

        eventos::editar(&mut base, parcial.id, evento("Parcial", TipoEvento::Examen, "2026-11-21"), &ahora).unwrap();
        let lista = pendientes(&base);
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].estado, EstadoResponsabilidad::Hecha);

        eventos::borrar(&mut base, parcial.id).unwrap();
        assert_eq!(pendientes(&base).len(), 1, "las hechas quedan para el progreso");
    }

    #[test]
    fn borrar_el_evento_borra_la_intacta() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let parcial = eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        eventos::borrar(&mut base, parcial.id).unwrap();
        let filas: i64 = base.query_row("SELECT count(*) FROM responsabilidades", [], |f| f.get(0)).unwrap();
        assert_eq!(filas, 0);
    }

    #[test]
    fn cambiar_el_tipo_borra_la_intacta() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let e = eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        eventos::editar(&mut base, e.id, evento("Consulta", TipoEvento::Otro, "2026-11-20"), &ahora).unwrap();
        assert!(pendientes(&base).is_empty());
    }

    #[test]
    fn la_descartada_no_vuelve() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let e = eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        resp::borrar(&base, pendientes(&base)[0].id).unwrap();
        assert!(pendientes(&base).is_empty());

        eventos::editar(&mut base, e.id, evento("Parcial", TipoEvento::Examen, "2026-11-24"), &ahora).unwrap();
        sincronizar_todo(&mut base, &ahora).unwrap();
        assert!(pendientes(&base).is_empty());
    }

    #[test]
    fn cumpleanos_pasa_al_anio_siguiente() {
        let mut base = db::en_memoria();
        let mut cumple = evento("Cumple de Juan", TipoEvento::Cumpleanos, "2000-11-20");
        cumple.persona = Some("Juan".into());
        cumple.recurrencia = Recurrencia::Anual;
        eventos::crear(&mut base, cumple, &Ahora::fijo("2026-11-01")).unwrap();
        assert_eq!(resumen(&base), ["Comprar regalo para Juan | 2026-11-17"]);

        // Se compró el regalo; pasa el cumple y se abre el panel al otro día
        resp::marcar(&base, pendientes(&base)[0].id, true, &Ahora::fijo("2026-11-18")).unwrap();
        sincronizar_todo(&mut base, &Ahora::fijo("2026-11-21")).unwrap();
        let lista = pendientes(&base);
        assert_eq!(lista.len(), 2);
        assert_eq!(lista[1].fecha_limite.as_deref(), Some("2027-11-17"));
        assert_eq!(lista[1].estado, EstadoResponsabilidad::Pendiente);
    }

    #[test]
    fn la_intacta_de_un_evento_pasado_se_va() {
        let mut base = db::en_memoria();
        eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &Ahora::fijo("2026-11-17")).unwrap();
        sincronizar_todo(&mut base, &Ahora::fijo("2026-11-21")).unwrap();
        assert!(pendientes(&base).is_empty());
    }

    #[test]
    fn la_fecha_limite_no_queda_vencida_al_crearla_tarde() {
        let mut base = db::en_memoria();
        // Cumple pasado mañana, la regla avisa 3 días antes: vence hoy, no ayer.
        let mut cumple = evento("Cumple de Ana", TipoEvento::Cumpleanos, "2026-11-19");
        cumple.persona = Some("Ana".into());
        eventos::crear(&mut base, cumple, &Ahora::fijo("2026-11-17")).unwrap();
        assert_eq!(resumen(&base), ["Comprar regalo para Ana | 2026-11-17"]);

        // Y sincronizar al día siguiente no la corre (el piso es cuando se creó)
        sincronizar_todo(&mut base, &Ahora::fijo("2026-11-18")).unwrap();
        assert_eq!(resumen(&base), ["Comprar regalo para Ana | 2026-11-17"]);
    }

    #[test]
    fn cambiar_las_reglas_y_sincronizar() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-01");
        eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();

        let mut reglas = Reglas::default();
        reglas.examen.dias_antes = 7;
        preferencias::guardar(&base, preferencias::REGLAS, &reglas).unwrap();
        sincronizar_todo(&mut base, &ahora).unwrap();
        assert_eq!(resumen(&base), ["Estudiar para Parcial | 2026-11-13"]);

        reglas.examen.activa = false;
        preferencias::guardar(&base, preferencias::REGLAS, &reglas).unwrap();
        sincronizar_todo(&mut base, &ahora).unwrap();
        assert!(pendientes(&base).is_empty());
    }

    #[test]
    fn sincronizar_dos_veces_no_duplica() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        eventos::crear(&mut base, evento("Parcial", TipoEvento::Examen, "2026-11-20"), &ahora).unwrap();
        sincronizar_todo(&mut base, &ahora).unwrap();
        sincronizar_todo(&mut base, &ahora).unwrap();
        assert_eq!(pendientes(&base).len(), 1);
    }
}
