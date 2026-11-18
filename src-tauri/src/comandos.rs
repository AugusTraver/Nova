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
