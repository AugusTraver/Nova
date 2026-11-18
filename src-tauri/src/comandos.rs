//! Comandos que el frontend llama con `invoke` (ver src/lib/api.ts).
//!
//! Cada uno es una capa finita: toma la conexión, llama al repositorio y
//! devuelve el resultado. `State<'_, Db>` es la base que guardamos con
//! `app.manage(...)` al arrancar; Tauri se la pasa a cada comando.

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

use crate::asistente::ciclo::{self, Asistente, Avisos, Progreso, Turno};
use crate::asistente::herramientas::Fuente;
use crate::asistente::mensajes::{Mensaje, RespuestaModelo};
use crate::asistente::openai::ClienteCompatible;
use crate::asistente::proveedor::{AjustesIa, ProveedorIa, TipoProveedor};
use crate::clima::{AjustesClima, CiudadEncontrada, ClienteClima, VistaClima};
use crate::db::Db;
use crate::noticias::{self, AjustesNoticias, Noticia};
use crate::fondo::Fondo;
use crate::futbol::{self, JugadorEncontrado, JugadorSeguido};
use crate::futbol::espn::ClienteEspn;
use crate::futbol::proveedor::ProveedorFutbol;
use crate::futbol::vista::VistaFutbol;
use crate::repos::cache::Cacheado;
use crate::dominio::lenguaje::interpretar_confirmacion;
use crate::repos::mensajes;
use crate::{atajos, llavero};
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

// ---------- Asistente ----------

/// Avisos del turno hacia la interfaz. El progreso va por un `Channel` (un
/// caño directo al `invoke` que lo pidió); los cambios de datos van como
/// evento global `datos-cambiados`, así los escucha cualquier ventana, aunque
/// el cambio no venga de la barra de texto.
struct AvisosTauri {
    app: AppHandle,
    canal: Channel<Progreso>,
}

pub const EVENTO_DATOS_CAMBIADOS: &str = "datos-cambiados";

impl Avisos for AvisosTauri {
    fn progreso(&self, progreso: Progreso) {
        // Si la ventana ya no escucha, no pasa nada.
        let _ = self.canal.send(progreso);
    }

    fn cambios(&self, fuentes: &[Fuente]) {
        let _ = self.app.emit(EVENTO_DATOS_CAMBIADOS, fuentes);
    }
}

fn cliente_ia(db: &Db) -> Resultado<ClienteCompatible> {
    let ajustes = preferencias::ajustes_ia(&db.conexion())?;
    let key = match ajustes.proveedor.cuenta_llavero() {
        Some(cuenta) => llavero::leer(cuenta)?,
        None => None,
    };
    ajustes.cliente(key)
}

/// Un turno con el asistente. Es `async`: mientras se espera al modelo, el
/// resto de los comandos (los widgets) siguen andando.
#[tauri::command]
pub async fn enviar_mensaje(
    app: AppHandle,
    db: State<'_, Db>,
    asistente: State<'_, Asistente>,
    texto: String,
    progreso: Channel<Progreso>,
) -> Resultado<Turno> {
    let avisos = AvisosTauri { app, canal: progreso };
    let ahora = Ahora::del_sistema();
    // Un "sí" a un borrado pendiente no necesita al modelo (ni la key).
    if asistente.hay_pendiente() && interpretar_confirmacion(&texto).is_some() {
        return ciclo::responder(&SinModelo, &db, &asistente, &texto, &avisos, &ahora).await;
    }
    let cliente = cliente_ia(&db)?;
    ciclo::responder(&cliente, &db, &asistente, &texto, &avisos, &ahora).await
}

/// Los botones "Sí, borralo" / "No" debajo de la esfera.
#[tauri::command]
pub fn confirmar_accion(
    app: AppHandle,
    db: State<'_, Db>,
    asistente: State<'_, Asistente>,
    aceptar: bool,
    progreso: Channel<Progreso>,
) -> Resultado<Turno> {
    let avisos = AvisosTauri { app, canal: progreso };
    ciclo::resolver_pendiente(&db, &asistente, aceptar, &avisos, &Ahora::del_sistema())
}

#[tauri::command]
pub fn borrar_historial(db: State<'_, Db>) -> Resultado<()> {
    mensajes::borrar_todo(&db.conexion())
}

/// Lo que ve Ajustes: los ajustes y si hay una key guardada (nunca la key).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VistaAjustesIa {
    pub ajustes: AjustesIa,
    pub tiene_key: bool,
}

fn vista_ia(ajustes: AjustesIa) -> Resultado<VistaAjustesIa> {
    let tiene_key = match TipoProveedor::Groq.cuenta_llavero() {
        Some(cuenta) => llavero::leer(cuenta)?.is_some(),
        None => false,
    };
    Ok(VistaAjustesIa { ajustes, tiene_key })
}

#[tauri::command]
pub fn obtener_ajustes_ia(db: State<'_, Db>) -> Resultado<VistaAjustesIa> {
    vista_ia(preferencias::ajustes_ia(&db.conexion())?)
}

#[tauri::command]
pub fn guardar_ajustes_ia(db: State<'_, Db>, ajustes: AjustesIa) -> Resultado<VistaAjustesIa> {
    let ajustes = ajustes.validar().map_err(Error::Validacion)?;
    preferencias::guardar(&db.conexion(), preferencias::IA, &ajustes)?;
    vista_ia(ajustes)
}

/// Guarda la API key de Groq en el llavero. Vacía = borrarla.
#[tauri::command]
pub fn guardar_api_key(key: String) -> Resultado<bool> {
    let cuenta = TipoProveedor::Groq.cuenta_llavero().unwrap_or_default();
    let key = key.trim();
    if key.is_empty() {
        llavero::borrar(cuenta)?;
        return Ok(false);
    }
    if key.len() > 300 || key.chars().any(char::is_whitespace) {
        return invalido("Esa key no parece válida.");
    }
    llavero::guardar(cuenta, key)?;
    Ok(true)
}

/// "Probar conexión" en Ajustes: un pedido mínimo al proveedor elegido.
#[tauri::command]
pub async fn probar_conexion(db: State<'_, Db>) -> Resultado<()> {
    let cliente = cliente_ia(&db)?;
    cliente.probar().await
}

// ---------- Datos de internet ----------

/// Lo que muestra el widget de fútbol (siempre desde la caché).
#[tauri::command]
pub fn obtener_futbol(db: State<'_, Db>) -> Resultado<Cacheado<VistaFutbol>> {
    futbol::vista::vista(&db.conexion(), &Ahora::del_sistema())
}

/// El panel se abrió: que la tarea de fondo mire si algo venció.
#[tauri::command]
pub fn refrescar_externos(fondo: State<'_, Fondo>) {
    fondo.despertar();
}

/// El buscador de jugadores de Ajustes.
#[tauri::command]
pub async fn buscar_jugadores(texto: String) -> Resultado<Vec<JugadorEncontrado>> {
    let texto = texto.trim();
    if texto.chars().count() < 3 {
        return Ok(Vec::new());
    }
    ClienteEspn::nuevo()?.buscar_jugadores(texto).await
}

#[tauri::command]
pub fn obtener_jugadores(db: State<'_, Db>) -> Resultado<Vec<JugadorSeguido>> {
    preferencias::jugadores(&db.conexion())
}

#[tauri::command]
pub fn guardar_jugadores(
    app: AppHandle,
    db: State<'_, Db>,
    fondo: State<'_, Fondo>,
    jugadores: Vec<JugadorSeguido>,
) -> Resultado<Vec<JugadorSeguido>> {
    if jugadores.len() > 20 {
        return invalido("Seguí como mucho 20 jugadores.");
    }
    let mut limpios: Vec<JugadorSeguido> = Vec::new();
    for j in jugadores {
        let nombre = j.nombre.trim().to_string();
        let id = j.id.trim().to_string();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) || nombre.is_empty() {
            return invalido("Hay un jugador sin datos válidos.");
        }
        if !limpios.iter().any(|l| l.id == id) {
            limpios.push(JugadorSeguido { id, nombre });
        }
    }
    preferencias::guardar(&db.conexion(), preferencias::JUGADORES, &limpios)?;
    fondo.despertar(); // los nuevos se descargan ya
    let _ = app.emit(EVENTO_DATOS_CAMBIADOS, [Fuente::Futbol]);
    Ok(limpios)
}

// ---------- Noticias ----------

#[tauri::command]
pub fn obtener_noticias(db: State<'_, Db>) -> Resultado<Cacheado<Vec<Noticia>>> {
    noticias::vista(&db.conexion())
}

#[tauri::command]
pub fn obtener_ajustes_noticias(db: State<'_, Db>) -> Resultado<AjustesNoticias> {
    preferencias::noticias(&db.conexion())
}

/// Guarda intereses y feeds, y fuerza a elegir de nuevo (con lo nuevo).
#[tauri::command]
pub fn guardar_ajustes_noticias(
    app: AppHandle,
    db: State<'_, Db>,
    fondo: State<'_, Fondo>,
    ajustes: AjustesNoticias,
) -> Resultado<AjustesNoticias> {
    let ajustes = ajustes.validar().map_err(Error::Validacion)?;
    {
        let conexion = db.conexion();
        let ahora = Ahora::del_sistema();
        let anteriores = preferencias::noticias(&conexion)?;
        preferencias::guardar(&conexion, preferencias::NOTICIAS, &ajustes)?;
        // Feeds distintos: hay que bajarlos. Intereses distintos: alcanza con
        // volver a elegir entre lo que ya está.
        if anteriores.feeds != ajustes.feeds {
            crate::repos::cache::vencer(&conexion, noticias::CLAVE_TITULARES, &ahora)?;
        }
        crate::repos::cache::borrar(&conexion, noticias::CLAVE_ELEGIDAS)?;
    }
    fondo.despertar();
    let _ = app.emit(EVENTO_DATOS_CAMBIADOS, [Fuente::Noticias]);
    Ok(ajustes)
}

/// Abre una nota en el navegador. Solo direcciones web: un feed no puede
/// hacer que se abra un programa o un archivo.
#[tauri::command]
pub fn abrir_enlace(app: AppHandle, url: String) -> Resultado<()> {
    use tauri_plugin_opener::OpenerExt;
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return invalido("Esa dirección no se puede abrir.");
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| Error::Sistema(format!("No pude abrir el navegador: {e}")))
}

// ---------- Clima ----------

#[tauri::command]
pub fn obtener_clima(db: State<'_, Db>) -> Resultado<VistaClima> {
    let conexion = db.conexion();
    let ajustes = preferencias::clima(&conexion)?;
    Ok(VistaClima { mostrar: ajustes.mostrar, clima: crate::repos::cache::leer(&conexion, crate::clima::CLAVE)? })
}

#[tauri::command]
pub fn obtener_ajustes_clima(db: State<'_, Db>) -> Resultado<AjustesClima> {
    preferencias::clima(&db.conexion())
}

#[tauri::command]
pub fn guardar_ajustes_clima(
    app: AppHandle,
    db: State<'_, Db>,
    fondo: State<'_, Fondo>,
    ajustes: AjustesClima,
) -> Resultado<AjustesClima> {
    let ajustes = ajustes.validar().map_err(Error::Validacion)?;
    {
        let conexion = db.conexion();
        let anteriores = preferencias::clima(&conexion)?;
        preferencias::guardar(&conexion, preferencias::CLIMA, &ajustes)?;
        if anteriores.latitud != ajustes.latitud || anteriores.longitud != ajustes.longitud {
            // Otra ciudad: lo guardado ya no sirve.
            crate::repos::cache::borrar(&conexion, crate::clima::CLAVE)?;
        }
    }
    fondo.despertar();
    let _ = app.emit(EVENTO_DATOS_CAMBIADOS, [Fuente::Clima]);
    Ok(ajustes)
}

#[tauri::command]
pub async fn buscar_ciudades(texto: String) -> Resultado<Vec<CiudadEncontrada>> {
    if texto.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    ClienteClima::nuevo()?.buscar(&texto).await
}

/// Para el "sí" a una confirmación: nunca llega a llamar al modelo.
struct SinModelo;

impl ProveedorIa for SinModelo {
    async fn completar(&self, _: &[Mensaje], _: &serde_json::Value) -> Resultado<RespuestaModelo> {
        Err(Error::Sistema("No hay modelo configurado.".into()))
    }
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

        let ia: AjustesIa = serde_json::from_value(json!({
            "proveedor": "ollama", "modeloGroq": "openai/gpt-oss-120b", "modeloOllama": "qwen3:8b",
            "urlOllama": "http://localhost:11434"
        }))
        .unwrap();
        assert_eq!(ia.proveedor, TipoProveedor::Ollama);
        assert_eq!(ia.modelo_ollama, "qwen3:8b");
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

        // Asistente (src/lib/tipos.ts: Turno, Progreso, Fuente, VistaAjustesIa).
        let turno = Turno { respuesta: "Listo.".into(), confirmacion: Some("¿Borro?".into()) };
        assert_eq!(claves(&serde_json::to_value(&turno).unwrap()), ["confirmacion", "respuesta"]);
        assert_eq!(serde_json::to_value(Progreso::Pensando).unwrap(), json!({ "tipo": "pensando" }));
        assert_eq!(
            serde_json::to_value(Progreso::Herramienta { nombre: "crear_evento".into() }).unwrap(),
            json!({ "tipo": "herramienta", "nombre": "crear_evento" })
        );
        assert_eq!(
            serde_json::to_value([Fuente::Agenda, Fuente::Responsabilidades, Fuente::Materias]).unwrap(),
            json!(["agenda", "responsabilidades", "materias"])
        );
        let vista = serde_json::to_value(VistaAjustesIa { ajustes: AjustesIa::default(), tiene_key: false }).unwrap();
        assert_eq!(claves(&vista), ["ajustes", "tieneKey"]);
        assert_eq!(claves(&vista["ajustes"]), ["modeloGroq", "modeloOllama", "proveedor", "urlOllama"]);
        assert_eq!(vista["ajustes"]["proveedor"], "groq");

        let error = serde_json::to_value(Error::Validacion("Ponele un título.".into())).unwrap();
        assert_eq!(error, json!({ "tipo": "validacion", "mensaje": "Ponele un título." }));
    }

    #[test]
    fn contrato_de_internet() {
        use crate::clima::{AjustesClima, Clima};
        use crate::dominio::futbol::tests::partido;
        use crate::futbol::{EstadoJugador, EstadoPartido, EventoPartido, TipoEventoPartido};

        // Cacheado<T>, Partido, EventoPartido, EstadoPartido (src/lib/tipos.ts)
        let mut p = partido("1", "2026-11-17T23:30:00Z", EstadoPartido::EnJuego, Some((1, 0)));
        p.eventos.push(EventoPartido { minuto: "12'".into(), tipo: TipoEventoPartido::GolEnContra, equipo_id: "5".into(), jugador: None });
        let c = Cacheado { datos: Some(p), obtenido_en: None, error: None };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(claves(&v), ["datos", "error", "obtenidoEn"]);
        assert_eq!(
            claves(&v["datos"]),
            [
                "competencia", "estado", "eventos", "fase", "fecha", "golesLocal", "golesVisitante", "id", "liga", "local",
                "minuto", "penalesLocal", "penalesVisitante", "visitante"
            ]
        );
        assert_eq!(v["datos"]["estado"], "en_juego");
        assert_eq!(claves(&v["datos"]["local"]), ["abreviatura", "escudo", "id", "nombre"]);
        assert_eq!(v["datos"]["eventos"][0], json!({ "minuto": "12'", "tipo": "gol_en_contra", "equipoId": "5", "jugador": null }));

        // VistaFutbol, ResumenEquipo, EstadoJugador
        let vista = VistaFutbol {
            equipos: vec![crate::dominio::futbol::ResumenEquipo {
                clave: "boca".into(),
                id: "5".into(),
                nombre: "Boca".into(),
                proximo: None,
                ultimo: None,
                en_vivo: None,
                posicion: None,
            }],
            jugadores: vec![EstadoJugador { id: "1".into(), nombre: "M".into(), club: None, ultimo: None, partidos: 1, goles: 0, asistencias: 0 }],
        };
        let v = serde_json::to_value(&vista).unwrap();
        assert_eq!(claves(&v["equipos"][0]), ["clave", "enVivo", "id", "nombre", "posicion", "proximo", "ultimo"]);
        assert_eq!(claves(&v["jugadores"][0]), ["asistencias", "club", "goles", "id", "nombre", "partidos", "ultimo"]);

        // Noticia, AjustesNoticias, VistaClima, AjustesClima, AvisoGol, Fuente
        let n = Noticia { id: "a".into(), titulo: "T".into(), resumen: "".into(), fuente: "Olé".into(), url: "https://x".into(), publicado: None };
        assert_eq!(claves(&serde_json::to_value(&n).unwrap()), ["fuente", "id", "publicado", "resumen", "titulo", "url"]);
        let a = serde_json::to_value(AjustesNoticias::default()).unwrap();
        assert_eq!(claves(&a), ["feeds", "intereses"]);
        assert_eq!(claves(&a["feeds"][0]), ["nombre", "url"]);
        let clima = Clima {
            ciudad: "BA".into(), temperatura: 20.0, sensacion: None, codigo: 0, descripcion: "despejado".into(),
            es_de_dia: true, maxima: None, minima: None, lluvia: None,
        };
        let v = serde_json::to_value(VistaClima { mostrar: true, clima: Cacheado { datos: Some(clima), obtenido_en: None, error: None } }).unwrap();
        assert_eq!(claves(&v), ["clima", "mostrar"]);
        assert_eq!(
            claves(&v["clima"]["datos"]),
            ["ciudad", "codigo", "descripcion", "esDeDia", "lluvia", "maxima", "minima", "sensacion", "temperatura"]
        );
        assert_eq!(claves(&serde_json::to_value(AjustesClima::default()).unwrap()), ["ciudad", "latitud", "longitud", "mostrar"]);
        let gol = serde_json::to_value(crate::fondo::AvisoGol { equipo: "boca".into(), goles: 1 }).unwrap();
        assert_eq!(gol, json!({ "equipo": "boca", "goles": 1 }));
        assert_eq!(serde_json::to_value([Fuente::Futbol, Fuente::Noticias, Fuente::Clima]).unwrap(), json!(["futbol", "noticias", "clima"]));
    }
}
