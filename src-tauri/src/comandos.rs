//! Comandos que el frontend llama con `invoke` (ver src/lib/api.ts).
//!
//! Cada uno es una capa finita: toma la conexión, llama al repositorio y
//! devuelve el resultado. `State<'_, Db>` es la base que guardamos con
//! `app.manage(...)` al arrancar; Tauri se la pasa a cada comando.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::atajos;
use crate::db::Db;
use crate::dominio::reglas::Reglas;
use crate::dominio::texto::OrigenResponsabilidad;
use crate::error::{Error, Resultado, invalido};
use crate::repos::eventos::{self, DatosEvento, Evento, Ocurrencia};
use crate::repos::materias::{self, DatosMateria, Materia};
use crate::repos::responsabilidades::{self, DatosResponsabilidad, Responsabilidad};
use crate::repos::{Ahora, automaticas, fecha, preferencias};

// ---------- Materias ----------

#[tauri::command]
pub fn listar_materias(db: State<'_, Db>) -> Resultado<Vec<Materia>> {
    materias::listar(&db.conexion())
}

#[tauri::command]
pub fn crear_materia(db: State<'_, Db>, datos: DatosMateria) -> Resultado<Materia> {
    materias::crear(&db.conexion(), datos, &Ahora::del_sistema())
}

#[tauri::command]
pub fn editar_materia(db: State<'_, Db>, id: i64, datos: DatosMateria) -> Resultado<Materia> {
    let mut conexion = db.conexion();
    let materia = materias::editar(&conexion, id, datos)?;
    // El nombre de la materia aparece en "Estudiar para {materia}".
    automaticas::sincronizar_todo(&mut conexion, &Ahora::del_sistema())?;
    Ok(materia)
}

#[tauri::command]
pub fn borrar_materia(db: State<'_, Db>, id: i64) -> Resultado<()> {
    let mut conexion = db.conexion();
    materias::borrar(&conexion, id)?;
    automaticas::sincronizar_todo(&mut conexion, &Ahora::del_sistema())
}

// ---------- Eventos ----------

/// Repeticiones entre dos fechas ("2026-11-17"), ambas incluidas.
#[tauri::command]
pub fn listar_ocurrencias(db: State<'_, Db>, desde: String, hasta: String) -> Resultado<Vec<Ocurrencia>> {
    let (Some(desde), Some(hasta)) = (fecha(&desde), fecha(&hasta)) else {
        return invalido("Rango de fechas inválido.");
    };
    if hasta < desde || (hasta - desde).num_days() > 400 {
        return invalido("El rango tiene que ir hacia adelante y durar como mucho 400 días.");
    }
    eventos::ocurrencias(&db.conexion(), desde, hasta)
}

#[tauri::command]
pub fn crear_evento(db: State<'_, Db>, datos: DatosEvento) -> Resultado<Evento> {
    eventos::crear(&mut db.conexion(), datos, &Ahora::del_sistema())
}

#[tauri::command]
pub fn editar_evento(db: State<'_, Db>, id: i64, datos: DatosEvento) -> Resultado<Evento> {
    eventos::editar(&mut db.conexion(), id, datos, &Ahora::del_sistema())
}

#[tauri::command]
pub fn borrar_evento(db: State<'_, Db>, id: i64) -> Resultado<()> {
    eventos::borrar(&mut db.conexion(), id)
}

/// Saltea una fecha de una serie (`saltear: true`) o la vuelve a poner.
#[tauri::command]
pub fn saltear_fecha(db: State<'_, Db>, id: i64, fecha: String, saltear: bool) -> Resultado<Evento> {
    eventos::cambiar_excepcion(&mut db.conexion(), id, &fecha, saltear, &Ahora::del_sistema())
}

// ---------- Responsabilidades ----------

#[tauri::command]
pub fn listar_responsabilidades(db: State<'_, Db>) -> Resultado<Vec<Responsabilidad>> {
    responsabilidades::listar(&db.conexion())
}

#[tauri::command]
pub fn crear_responsabilidad(db: State<'_, Db>, datos: DatosResponsabilidad) -> Resultado<Responsabilidad> {
    responsabilidades::crear(&db.conexion(), datos, OrigenResponsabilidad::Manual, &Ahora::del_sistema())
}

#[tauri::command]
pub fn editar_responsabilidad(db: State<'_, Db>, id: i64, datos: DatosResponsabilidad) -> Resultado<Responsabilidad> {
    responsabilidades::editar(&db.conexion(), id, datos, &Ahora::del_sistema())
}

#[tauri::command]
pub fn marcar_responsabilidad(db: State<'_, Db>, id: i64, hecha: bool) -> Resultado<Responsabilidad> {
    responsabilidades::marcar(&db.conexion(), id, hecha, &Ahora::del_sistema())
}

#[tauri::command]
pub fn borrar_responsabilidad(db: State<'_, Db>, id: i64) -> Resultado<()> {
    responsabilidades::borrar(&db.conexion(), id)
}

/// Pone al día las automáticas. El panel lo llama cada vez que se abre.
#[tauri::command]
pub fn sincronizar_automaticas(db: State<'_, Db>) -> Resultado<()> {
    automaticas::sincronizar_todo(&mut db.conexion(), &Ahora::del_sistema())
}

// ---------- Ajustes ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ajustes {
    /// Ej. "Ctrl+Shift+Space" (modificadores + código de tecla del navegador).
    pub atajo: String,
    pub reglas: Reglas,
}

#[tauri::command]
pub fn obtener_ajustes(db: State<'_, Db>) -> Resultado<Ajustes> {
    let conexion = db.conexion();
    Ok(Ajustes { atajo: preferencias::atajo(&conexion)?, reglas: preferencias::reglas(&conexion)? })
}

/// Guarda los ajustes. El atajo se registra primero: si otra app lo tiene
/// tomado, no se guarda nada y se avisa.
#[tauri::command]
pub fn guardar_ajustes(app: AppHandle, db: State<'_, Db>, ajustes: Ajustes) -> Resultado<Ajustes> {
    ajustes.reglas.validar().map_err(Error::Validacion)?;
    let mut conexion = db.conexion();

    let actual = preferencias::atajo(&conexion)?;
    let atajo = atajos::cambiar(&app, &actual, &ajustes.atajo).map_err(Error::Validacion)?;

    preferencias::guardar(&conexion, preferencias::ATAJO, &atajo)?;
    preferencias::guardar(&conexion, preferencias::REGLAS, &ajustes.reglas)?;
    automaticas::sincronizar_todo(&mut conexion, &Ahora::del_sistema())?;
    Ok(Ajustes { atajo, reglas: ajustes.reglas })
}

/// Contrato con el frontend: los JSON que arma `src/lib/api.ts` (con los
/// nombres en camelCase de `src/lib/tipos.ts`) tienen que leerse bien acá, y lo
/// que devolvemos tiene que traer esos mismos nombres. Si alguien renombra un
/// campo de un lado solo, falla este test.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::db;
    use crate::dominio::texto::{Prioridad, Recurrencia, TipoEvento};

    fn claves(valor: &serde_json::Value) -> Vec<String> {
        let mut c: Vec<String> = valor.as_object().unwrap().keys().cloned().collect();
        c.sort();
        c
    }

    #[test]
    fn lee_lo_que_manda_la_interfaz() {
        let evento: DatosEvento = serde_json::from_value(json!({
            "titulo": "Parcial", "tipo": "examen", "inicio": "2026-11-20T09:00:00-03:00",
            "fin": null, "todoElDia": false, "materiaId": 1, "persona": null, "notas": null,
            "recurrencia": "ninguna", "repetirHasta": null
        }))
        .unwrap();
        assert_eq!(evento.tipo, TipoEvento::Examen);
        assert_eq!(evento.materia_id, Some(1));
        assert_eq!(evento.recurrencia, Recurrencia::Ninguna);

        let resp: DatosResponsabilidad = serde_json::from_value(json!({
            "titulo": "x", "descripcion": null, "fechaLimite": "2026-11-20",
            "prioridad": "alta", "materiaId": null, "eventoId": 3
        }))
        .unwrap();
        assert_eq!(resp.prioridad, Prioridad::Alta);
        assert_eq!(resp.evento_id, Some(3));

        let materia: DatosMateria = serde_json::from_value(json!({ "nombre": "Física", "color": "menta" })).unwrap();
        assert_eq!(materia.color, "menta");

        let ajustes: Ajustes = serde_json::from_value(json!({
            "atajo": "Ctrl+Shift+Space",
            "reglas": {
                "examen": { "activa": true, "diasAntes": 1, "plantilla": "Estudiar para {materia}" },
                "entrega": { "activa": false, "diasAntes": 2, "plantilla": "Terminar {entrega}" },
                "cumpleanos": { "activa": true, "diasAntes": 3, "plantilla": "Comprar regalo para {persona}" }
            }
        }))
        .unwrap();
        assert_eq!(ajustes.reglas.entrega.dias_antes, 2);
    }

    #[test]
    fn devuelve_los_nombres_que_espera_la_interfaz() {
        let mut base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let fisica = materias::crear(&base, DatosMateria { nombre: "Física".into(), color: "menta".into() }, &ahora).unwrap();
        let mut datos = crate::repos::eventos::tests::datos("Parcial", TipoEvento::Examen, "2026-11-20T09:00:00-03:00");
        datos.materia_id = Some(fisica.id);
        eventos::crear(&mut base, datos, &ahora).unwrap();

        let ocurrencia = serde_json::to_value(
            &eventos::ocurrencias(&base, fecha("2026-11-17").unwrap(), fecha("2026-11-30").unwrap()).unwrap()[0],
        )
        .unwrap();
        assert_eq!(claves(&ocurrencia), ["evento", "fecha", "fin", "inicio"]);
        assert_eq!(
            claves(&ocurrencia["evento"]),
            [
                "excepciones", "fin", "id", "inicio", "materiaId", "notas", "persona", "recurrencia",
                "repetirHasta", "tipo", "titulo", "todoElDia"
            ]
        );

        let resp = serde_json::to_value(&responsabilidades::listar(&base).unwrap()[0]).unwrap();
        assert_eq!(
            claves(&resp),
            [
                "creadoEn", "descripcion", "estado", "eventoId", "fechaLimite", "hechaEn", "id", "materiaId",
                "origen", "prioridad", "titulo"
            ]
        );
        assert_eq!(resp["origen"], "auto");

        assert_eq!(claves(&serde_json::to_value(&fisica).unwrap()), ["color", "id", "nombre"]);

        let error = serde_json::to_value(Error::Validacion("Ponele un título.".into())).unwrap();
        assert_eq!(error, json!({ "tipo": "validacion", "mensaje": "Ponele un título." }));
    }
}
