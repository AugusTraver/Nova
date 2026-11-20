//! Las herramientas que el modelo puede usar (SPEC 5.2).
//!
//! Cada una lee sus argumentos (JSON), llama a los mismos `repos::*` que usa
//! la interfaz y devuelve un texto para el modelo. Los errores no cortan la
//! conversación: vuelven al modelo como texto, para que pregunte o corrija.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, SecondsFormat};
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::consultas;
use crate::dominio::fechas_relativas::{combinar, nombre_dia_con_tilde, nombre_mes, resolver_fecha, resolver_hora, resolver_hora_fin};
use crate::dominio::lenguaje::{interpretar_confirmacion, normalizar, vuelve_a_pedir_borrar};
use crate::dominio::momento::Momento;
use crate::dominio::superposicion;
use crate::dominio::texto::{EstadoResponsabilidad, OrigenResponsabilidad, Prioridad, Recurrencia, TipoEvento};
use crate::error::Error;
use crate::repos::eventos::{self, DatosEvento, Evento};
use crate::repos::materias::{self, DatosMateria, Materia, PALETA};
use crate::repos::responsabilidades::{self, DatosResponsabilidad, Responsabilidad};
use crate::repos::{Ahora, texto_fecha};

/// Qué datos cambió una herramienta, para que los widgets recarguen solo eso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fuente {
    Agenda,
    Responsabilidades,
    Materias,
    Futbol,
    Noticias,
    Clima,
}

/// Algo destructivo que espera el "sí" del usuario. Lo ejecuta Rust cuando el
/// usuario confirma; el modelo nunca puede confirmarlo solo.
#[derive(Debug, Clone, PartialEq)]
pub enum AccionPendiente {
    BorrarEvento { id: i64, titulo: String, pregunta: String },
    /// Cambiar, crear, borrar o deshacer un archivo (v0.7.1). Va en una
    /// `Box` (en el montón) porque es mucho más grande que la otra variante.
    Archivo(Box<crate::compu::archivos::Cambio>),
}

impl AccionPendiente {
    pub fn pregunta(&self) -> &str {
        match self {
            AccionPendiente::BorrarEvento { pregunta, .. } => pregunta,
            AccionPendiente::Archivo(cambio) => &cambio.pregunta,
        }
    }

    /// El antes y después, si es un cambio de archivo (para la pantalla).
    pub fn vista(&self) -> Option<&crate::compu::archivos::VistaCambio> {
        match self {
            AccionPendiente::Archivo(cambio) => Some(&cambio.vista),
            AccionPendiente::BorrarEvento { .. } => None,
        }
    }

    /// ¿El mensaje responde a esta pregunta? `Some(true)` = sí (un "sí" o
    /// volver a pedir borrar lo mismo), `Some(false)` = no, `None` = otra cosa.
    pub fn respuesta(&self, texto: &str) -> Option<bool> {
        match self {
            AccionPendiente::BorrarEvento { titulo, .. } => {
                interpretar_confirmacion(texto).or_else(|| vuelve_a_pedir_borrar(texto, titulo).then_some(true))
            }
            AccionPendiente::Archivo(_) => interpretar_confirmacion(texto),
        }
    }

    /// Para avisarle al modelo que la pregunta se descartó.
    pub fn descartada(&self) -> String {
        match self {
            AccionPendiente::BorrarEvento { titulo, .. } => format!(
                "Aviso: el borrado de «{titulo}» que esperaba confirmación quedó cancelado porque el usuario dijo otra cosa. \
                 NO está borrado. No digas que lo borraste; si lo quiere borrar, llamá de nuevo a borrar_evento."
            ),
            AccionPendiente::Archivo(cambio) => format!(
                "Aviso: el cambio en «{}» que esperaba confirmación quedó cancelado porque el usuario dijo otra cosa. \
                 NO está hecho. No digas que lo hiciste; si lo quiere, llamá de nuevo a la herramienta.",
                cambio.vista.archivo
            ),
        }
    }
}

/// Lo que devuelve una herramienta.
#[derive(Debug, Default)]
pub struct Salida {
    /// Texto (JSON) para el modelo.
    pub para_modelo: String,
    pub cambios: Vec<Fuente>,
    pub pendiente: Option<AccionPendiente>,
}

impl Salida {
    fn ok(valor: Value, cambios: &[Fuente]) -> Self {
        Salida { para_modelo: valor.to_string(), cambios: cambios.to_vec(), pendiente: None }
    }

    fn error(mensaje: impl Into<String>) -> Self {
        Salida { para_modelo: json!({ "error": mensaje.into() }).to_string(), ..Default::default() }
    }
}

/// Las definiciones que se le mandan al modelo, en el formato de "function
/// calling" que usan casi todos los proveedores (JSON Schema).
pub fn definiciones() -> Value {
    let fecha = "Como la dijo el usuario ('jueves', 'mañana', '20/11', 'el 25') o AAAA-MM-DD.";
    let hora = "Como la dijo el usuario ('9', '3 de la tarde', '18:30') o HH:MM. Sin hora = todo el día.";
    let herramientas = [
        (
            "crear_evento",
            "Agenda un evento. Los exámenes y entregas con materia crean solos la responsabilidad de estudiar o terminar.",
            json!({
                "titulo": { "type": "string", "description": "Corto, ej. 'Parcial de Análisis'." },
                "tipo": { "type": "string", "enum": ["clase", "examen", "entrega", "cumpleanos", "partido", "otro"] },
                "fecha": { "type": "string", "description": fecha },
                "hora": { "type": "string", "description": hora },
                "hora_fin": { "type": "string" },
                "duracion_minutos": { "type": "integer" },
                "materia": { "type": "string", "description": "Nombre de la materia, si es de la facu." },
                "persona": { "type": "string", "description": "Para cumpleaños." },
                "notas": { "type": "string" },
                "repetir": { "type": "string", "enum": ["ninguna", "semanal", "anual"] },
                "repetir_hasta": { "type": "string", "description": fecha },
                "aunque_se_superponga": { "type": "boolean", "description": "Solo si el usuario ya dijo que lo quiere igual aunque choque con otro evento." }
            }),
            vec!["titulo", "tipo", "fecha"],
        ),
        (
            "editar_evento",
            "Cambia un evento por id. Mandá solo lo que cambia; si se mueve, conserva la duración.",
            json!({
                "id": { "type": "integer" },
                "titulo": { "type": "string" },
                "tipo": { "type": "string", "enum": ["clase", "examen", "entrega", "cumpleanos", "partido", "otro"] },
                "fecha": { "type": "string", "description": fecha },
                "hora": { "type": "string", "description": hora },
                "hora_fin": { "type": "string" },
                "duracion_minutos": { "type": "integer" },
                "todo_el_dia": { "type": "boolean" },
                "materia": { "type": "string", "description": "'' para sacarla." },
                "persona": { "type": "string" },
                "notas": { "type": "string" },
                "aunque_se_superponga": { "type": "boolean", "description": "Solo si el usuario ya dijo que lo quiere igual aunque choque con otro evento." }
            }),
            vec!["id"],
        ),
        (
            "borrar_evento",
            "Pide borrar un evento por id. No borra: le muestra al usuario un botón para confirmar.",
            json!({ "id": { "type": "integer" } }),
            vec!["id"],
        ),
        (
            "listar_eventos",
            "Eventos entre dos fechas (incluidas).",
            json!({
                "desde": { "type": "string", "description": fecha },
                "hasta": { "type": "string", "description": "Por defecto, igual a desde." }
            }),
            vec!["desde"],
        ),
        (
            "crear_responsabilidad",
            "Anota algo para hacer.",
            json!({
                "titulo": { "type": "string" },
                "fecha_limite": { "type": "string", "description": fecha },
                "prioridad": { "type": "string", "enum": ["baja", "media", "alta"] },
                "materia": { "type": "string" },
                "evento_id": { "type": "integer" }
            }),
            vec!["titulo"],
        ),
        (
            "completar_responsabilidad",
            "Marca como hecha una pendiente, por id o buscando por texto.",
            json!({
                "id": { "type": "integer" },
                "texto": { "type": "string", "description": "Palabras del título, ej. 'regalo Juan'." }
            }),
            vec![],
        ),
        (
            "listar_responsabilidades",
            "Lista responsabilidades.",
            json!({ "filtro": { "type": "string", "enum": ["pendientes", "hoy", "vencidas", "hechas", "todas"] } }),
            vec![],
        ),
        (
            "crear_materia",
            "Agrega una materia de la facu.",
            json!({
                "nombre": { "type": "string" },
                "color": { "type": "string", "enum": PALETA }
            }),
            vec!["nombre"],
        ),
        ("listar_materias", "Lista las materias.", json!({}), vec![]),
        (
            "consultar_partidos",
            "Próximo partido, último resultado y si está jugando ahora.",
            json!({ "equipo": { "type": "string", "enum": ["boca", "seleccion"] } }),
            vec![],
        ),
        ("consultar_tabla", "Posición de Boca en el torneo local.", json!({}), vec![]),
        (
            "consultar_jugador",
            "Último partido y temporada de un jugador seguido.",
            json!({ "nombre": { "type": "string", "description": "Ej. 'Messi', 'Barco'." } }),
            vec!["nombre"],
        ),
        ("consultar_noticias", "Los titulares elegidos para el usuario.", json!({}), vec![]),
        (
            "resumen_del_dia",
            "El resumen de hoy (agenda, pendientes, fútbol, clima y una noticia). Para '¿qué tengo hoy?'.",
            json!({}),
            vec![],
        ),
    ];

    Value::Array(
        herramientas
            .into_iter()
            .map(|(nombre, descripcion, propiedades, requeridos)| funcion(nombre, descripcion, propiedades, &requeridos))
            .collect(),
    )
}

/// Una definición en el formato de "function calling" (también la usan las
/// herramientas de la compu).
pub(crate) fn funcion(nombre: &str, descripcion: &str, mut propiedades: Value, requeridos: &[&str]) -> Value {
    aceptar_null(&mut propiedades, requeridos);
    json!({
        "type": "function",
        "function": {
            "name": nombre,
            "description": descripcion,
            "parameters": { "type": "object", "properties": propiedades, "required": requeridos }
        }
    })
}

/// Los modelos a veces mandan `null` en un campo opcional ("materia": null).
/// Groq valida la llamada contra el esquema y, si no lo admite, rechaza el
/// pedido entero (y lo cobra igual del límite por minuto). Por eso cada campo
/// opcional acepta también `null`.
fn aceptar_null(propiedades: &mut Value, requeridos: &[&str]) {
    let Some(propiedades) = propiedades.as_object_mut() else { return };
    for (nombre, esquema) in propiedades.iter_mut() {
        if requeridos.contains(&nombre.as_str()) {
            continue;
        }
        if let Some(tipo) = esquema.get("type").and_then(Value::as_str).map(String::from) {
            esquema["type"] = json!([tipo, "null"]);
        }
        if let Some(opciones) = esquema.get_mut("enum").and_then(Value::as_array_mut) {
            opciones.push(Value::Null);
        }
    }
}

/// Solo las definiciones nombradas (`None` = todas). Ver `seleccion`.
pub fn definiciones_de(nombres: Option<&[&str]>) -> Value {
    let todas = definiciones();
    let Some(nombres) = nombres else { return todas };
    Value::Array(
        todas
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| d["function"]["name"].as_str().is_some_and(|n| nombres.contains(&n)))
            .cloned()
            .collect(),
    )
}

/// Ejecuta una herramienta. Nunca falla: los errores vuelven como texto.
pub fn ejecutar(conexion: &mut Connection, nombre: &str, argumentos: &str, ahora: &Ahora) -> Salida {
    let resultado = match nombre {
        "crear_evento" => leer(argumentos).and_then(|a| crear_evento(conexion, a, ahora)),
        "editar_evento" => leer(argumentos).and_then(|a| editar_evento(conexion, a, ahora)),
        "borrar_evento" => leer(argumentos).and_then(|a| borrar_evento(conexion, a)),
        "listar_eventos" => leer(argumentos).and_then(|a| listar_eventos(conexion, a, ahora)),
        "crear_responsabilidad" => leer(argumentos).and_then(|a| crear_responsabilidad(conexion, a, ahora)),
        "completar_responsabilidad" => leer(argumentos).and_then(|a| completar_responsabilidad(conexion, a, ahora)),
        "listar_responsabilidades" => leer(argumentos).and_then(|a| listar_responsabilidades(conexion, a, ahora)),
        "crear_materia" => leer(argumentos).and_then(|a| crear_materia(conexion, a, ahora)),
        "listar_materias" => listar_materias(conexion),
        "consultar_partidos" => leer(argumentos).and_then(|a| consultas::consultar_partidos(conexion, a, ahora)).map(solo_lectura),
        "consultar_tabla" => consultas::consultar_tabla(conexion, ahora).map(solo_lectura),
        "consultar_jugador" => leer(argumentos).and_then(|a| consultas::consultar_jugador(conexion, a, ahora)).map(solo_lectura),
        "consultar_noticias" => consultas::consultar_noticias(conexion).map(solo_lectura),
        "resumen_del_dia" => consultas::resumen_del_dia(conexion, ahora).map(solo_lectura),
        otro => Err(format!("No existe la herramienta «{otro}».")),
    };
    resultado.unwrap_or_else(Salida::error)
}

/// Una consulta no cambia nada: ningún widget tiene que recargar.
fn solo_lectura(valor: Value) -> Salida {
    Salida::ok(valor, &[])
}

/// Lo que sale mal adentro de una herramienta: un texto para el modelo.
type Paso<T> = Result<T, String>;

fn leer<T: DeserializeOwned>(argumentos: &str) -> Paso<T> {
    // Algunos modelos mandan "" en vez de "{}" cuando no hay argumentos.
    let texto = if argumentos.trim().is_empty() { "{}" } else { argumentos };
    let mut valor: Value = serde_json::from_str(texto).map_err(|e| format!("Argumentos inválidos: {e}"))?;
    // Un campo en `null` es lo mismo que no mandarlo.
    if let Some(campos) = valor.as_object_mut() {
        campos.retain(|_, v| !v.is_null());
    }
    serde_json::from_value(valor).map_err(|e| format!("Argumentos inválidos: {e}"))
}

/// Los errores de los repos ya vienen con textos en español para mostrar.
fn texto_error(e: Error) -> String {
    e.to_string()
}

/// Convierte "" en `None` (los modelos a veces mandan campos vacíos).
fn con_valor(texto: Option<String>) -> Option<String> {
    texto.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

// ---------- Materias ----------

/// Busca una materia por nombre, sin importar tildes ni mayúsculas.
/// "análisis" encuentra "Análisis II" si es la única que coincide.
fn buscar_materia(conexion: &Connection, nombre: &str) -> Paso<Materia> {
    let buscada = normalizar(nombre);
    let todas = materias::listar(conexion).map_err(texto_error)?;
    if let Some(exacta) = todas.iter().find(|m| normalizar(&m.nombre) == buscada) {
        return Ok(exacta.clone());
    }
    let parecidas: Vec<&Materia> = todas
        .iter()
        .filter(|m| {
            let n = normalizar(&m.nombre);
            n.contains(&buscada) || buscada.contains(&n)
        })
        .collect();
    match parecidas.as_slice() {
        [una] => Ok((*una).clone()),
        [] => Err(format!(
            "No existe la materia «{nombre}». Preguntale al usuario si la creás con crear_materia o si va sin materia."
        )),
        varias => Err(format!(
            "«{nombre}» puede ser {}. Preguntale al usuario cuál.",
            varias.iter().map(|m| format!("«{}»", m.nombre)).collect::<Vec<_>>().join(" o ")
        )),
    }
}

fn nombre_materia(conexion: &Connection, id: Option<i64>) -> Option<String> {
    id.and_then(|id| materias::obtener(conexion, id).ok()).map(|m| m.nombre)
}

#[derive(Deserialize)]
struct ArgsCrearMateria {
    nombre: String,
    color: Option<String>,
}

fn crear_materia(conexion: &mut Connection, a: ArgsCrearMateria, ahora: &Ahora) -> Paso<Salida> {
    let existentes = materias::listar(conexion).map_err(texto_error)?;
    if existentes.iter().any(|m| normalizar(&m.nombre) == normalizar(&a.nombre)) {
        return Err(format!("Ya existe la materia «{}».", a.nombre.trim()));
    }
    // Si no eligió color, el primero de la paleta que nadie usa.
    let color = con_valor(a.color).filter(|c| PALETA.contains(&c.as_str())).unwrap_or_else(|| {
        PALETA
            .iter()
            .find(|c| !existentes.iter().any(|m| m.color == **c))
            .unwrap_or(&PALETA[existentes.len() % PALETA.len()])
            .to_string()
    });
    let materia = materias::crear(conexion, DatosMateria { nombre: a.nombre, color }, ahora).map_err(texto_error)?;
    Ok(Salida::ok(json!({ "creada": materia }), &[Fuente::Materias]))
}

fn listar_materias(conexion: &mut Connection) -> Paso<Salida> {
    let lista = materias::listar(conexion).map_err(texto_error)?;
    Ok(Salida::ok(json!({ "materias": lista }), &[]))
}

// ---------- Eventos ----------

#[derive(Deserialize)]
struct ArgsCrearEvento {
    titulo: String,
    tipo: Option<String>,
    fecha: String,
    hora: Option<String>,
    hora_fin: Option<String>,
    duracion_minutos: Option<i64>,
    materia: Option<String>,
    persona: Option<String>,
    notas: Option<String>,
    repetir: Option<String>,
    repetir_hasta: Option<String>,
    #[serde(default)]
    aunque_se_superponga: bool,
}

fn tipo(texto: Option<String>) -> Paso<TipoEvento> {
    match con_valor(texto) {
        None => Ok(TipoEvento::Otro),
        Some(t) => TipoEvento::desde_texto(&normalizar(&t)).ok_or_else(|| format!("Tipo de evento inválido: «{t}».")),
    }
}

fn hora_iso(fecha: NaiveDate, hora: NaiveTime, ahora: &Ahora) -> String {
    combinar(fecha, hora, *ahora.momento.offset()).to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// El fin de un evento con hora: por `hora_fin` (el mismo día, o el
/// siguiente si es más temprano: "de 22 a 2") o por duración.
fn fin_con_hora(
    fecha: NaiveDate,
    inicio: NaiveTime,
    hora_fin: Option<String>,
    duracion: Option<i64>,
    ahora: &Ahora,
) -> Paso<Option<String>> {
    if let Some(texto) = con_valor(hora_fin) {
        let (fin, otro_dia) = resolver_hora_fin(&texto, inicio)?;
        let dia_fin = if otro_dia { fecha + Duration::days(1) } else { fecha };
        return Ok(Some(hora_iso(dia_fin, fin, ahora)));
    }
    match duracion {
        Some(minutos) if (1..=24 * 60).contains(&minutos) => {
            let fin = combinar(fecha, inicio, *ahora.momento.offset()) + Duration::minutes(minutos);
            Ok(Some(fin.to_rfc3339_opts(SecondsFormat::Secs, false)))
        }
        Some(_) => Err("La duración tiene que ser de 1 minuto a 24 horas.".into()),
        None => Ok(None),
    }
}

/// "jueves 19 de noviembre a las 09:00", para que el modelo lo repita bien.
fn describir(inicio: &str) -> String {
    match Momento::parsear(inicio) {
        Ok(Momento::Fecha(f)) => describir_fecha(f),
        Ok(Momento::FechaHora(fh)) => format!("{} a las {}", describir_fecha(fh.date_naive()), fh.format("%H:%M")),
        Err(_) => inicio.to_string(),
    }
}

fn describir_fecha(f: NaiveDate) -> String {
    format!("{} {} de {}", nombre_dia_con_tilde(f.weekday()), f.day(), nombre_mes(f.month()))
}

fn evento_para_modelo(conexion: &Connection, e: &Evento) -> Value {
    json!({
        "id": e.id,
        "titulo": e.titulo,
        "tipo": e.tipo,
        "cuando": describir(&e.inicio),
        "inicio": e.inicio,
        "fin": e.fin,
        "materia": nombre_materia(conexion, e.materia_id),
        "persona": e.persona,
        "repite": e.recurrencia,
    })
}

/// Las responsabilidades automáticas que generó un evento (para contarlas).
fn automaticas_de(conexion: &Connection, evento_id: i64) -> Vec<String> {
    responsabilidades::listar(conexion)
        .map(|lista| lista.into_iter().filter(|r| r.evento_id == Some(evento_id)).map(|r| r.titulo).collect())
        .unwrap_or_default()
}

/// Si el evento (con hora) se pisa con otro de ese día, no se guarda: el
/// modelo recibe con qué choca y le tiene que preguntar al usuario.
/// `excepto`: el mismo evento cuando se edita.
fn choques(conexion: &Connection, inicio: &str, fin: Option<&str>, excepto: Option<i64>) -> Paso<Option<Salida>> {
    let Some((desde, hasta)) = superposicion::rango(inicio, fin) else { return Ok(None) };
    // Un día antes por los eventos que cruzan la medianoche.
    let ocurrencias =
        eventos::ocurrencias(conexion, desde.date_naive() - Duration::days(1), hasta.date_naive()).map_err(texto_error)?;
    let pisados: Vec<Value> = ocurrencias
        .iter()
        .filter(|o| Some(o.evento.id) != excepto)
        .filter(|o| superposicion::se_superponen((inicio, fin), (&o.inicio, o.fin.as_deref())))
        .map(|o| json!({ "id": o.evento.id, "titulo": o.evento.titulo, "cuando": describir(&o.inicio), "fin": o.fin }))
        .collect();
    if pisados.is_empty() {
        return Ok(None);
    }
    Ok(Some(Salida::ok(
        json!({
            "no_guardado": true,
            "se_superpone_con": pisados,
            "nota": "No lo guardé porque choca con lo de arriba. Decile al usuario con qué choca y preguntale si lo quiere igual,                      en otro horario o si cambia el otro. Si dice que igual, llamá de nuevo con aunque_se_superponga: true."
        }),
        &[],
    )))
}

fn crear_evento(conexion: &mut Connection, a: ArgsCrearEvento, ahora: &Ahora) -> Paso<Salida> {
    let tipo = tipo(a.tipo)?;
    let fecha = resolver_fecha(&a.fecha, ahora.hoy)?;
    // Sin hora es de todo el día y va con fecha sola.
    let (inicio, fin, todo_el_dia) = match con_valor(a.hora) {
        Some(texto) => {
            let hora = resolver_hora(&texto)?;
            (hora_iso(fecha, hora, ahora), fin_con_hora(fecha, hora, a.hora_fin, a.duracion_minutos, ahora)?, false)
        }
        None => (texto_fecha(fecha), None, true),
    };
    let materia_id = match con_valor(a.materia) {
        Some(nombre) => Some(buscar_materia(conexion, &nombre)?.id),
        None => None,
    };
    let recurrencia = match con_valor(a.repetir) {
        None => Recurrencia::Ninguna,
        Some(r) => Recurrencia::desde_texto(&normalizar(&r)).ok_or_else(|| format!("Repetición inválida: «{r}»."))?,
    };
    let repetir_hasta = match con_valor(a.repetir_hasta) {
        Some(t) => Some(texto_fecha(resolver_fecha(&t, ahora.hoy)?)),
        None => None,
    };

    let datos = DatosEvento {
        titulo: a.titulo,
        tipo,
        inicio,
        fin,
        todo_el_dia,
        materia_id,
        persona: a.persona,
        notas: a.notas,
        recurrencia,
        repetir_hasta,
    };
    if !a.aunque_se_superponga
        && let Some(choque) = choques(conexion, &datos.inicio, datos.fin.as_deref(), None)?
    {
        return Ok(choque);
    }
    let evento = eventos::crear(conexion, datos, ahora).map_err(texto_error)?;
    let automaticas = automaticas_de(conexion, evento.id);
    Ok(Salida::ok(
        json!({ "creado": evento_para_modelo(conexion, &evento), "responsabilidades_automaticas": automaticas }),
        &[Fuente::Agenda, Fuente::Responsabilidades],
    ))
}

#[derive(Deserialize)]
struct ArgsEditarEvento {
    id: i64,
    titulo: Option<String>,
    tipo: Option<String>,
    fecha: Option<String>,
    hora: Option<String>,
    hora_fin: Option<String>,
    duracion_minutos: Option<i64>,
    todo_el_dia: Option<bool>,
    materia: Option<String>,
    persona: Option<String>,
    notas: Option<String>,
    #[serde(default)]
    aunque_se_superponga: bool,
}

fn editar_evento(conexion: &mut Connection, a: ArgsEditarEvento, ahora: &Ahora) -> Paso<Salida> {
    let anterior = eventos::obtener(conexion, a.id).map_err(texto_error)?;
    let corrupto = |e: String| format!("El evento guardado está roto: {e}");
    let inicio_anterior = Momento::parsear(&anterior.inicio).map_err(corrupto)?;
    let fin_anterior = anterior.fin.as_deref().map(Momento::parsear).transpose().map_err(corrupto)?;

    let fecha = match con_valor(a.fecha) {
        Some(t) => resolver_fecha(&t, ahora.hoy)?,
        None => inicio_anterior.fecha(),
    };
    let hora_anterior = match inicio_anterior {
        Momento::FechaHora(fh) => Some(fh.time()),
        Momento::Fecha(_) => None,
    };
    let hora = match (con_valor(a.hora), a.todo_el_dia) {
        (_, Some(true)) => None,
        (Some(t), _) => Some(resolver_hora(&t)?),
        (None, _) => hora_anterior,
    };

    let (inicio, fin) = match hora {
        Some(h) => {
            let inicio = hora_iso(fecha, h, ahora);
            let fin = if a.hora_fin.is_some() || a.duracion_minutos.is_some() {
                fin_con_hora(fecha, h, a.hora_fin, a.duracion_minutos, ahora)?
            } else {
                // Conserva la duración que tenía (si tenía hora de fin).
                match (inicio_anterior, fin_anterior) {
                    (Momento::FechaHora(i), Some(Momento::FechaHora(f))) => {
                        let nuevo = Momento::parsear(&inicio).map_err(corrupto)?;
                        let Momento::FechaHora(n) = nuevo else { unreachable!("tiene hora") };
                        Some((n + (f - i)).to_rfc3339_opts(SecondsFormat::Secs, false))
                    }
                    _ => None,
                }
            };
            (inicio, fin)
        }
        None => {
            // Todo el día: si duraba varios días, se mueve con el mismo largo.
            let fin = match (inicio_anterior, fin_anterior) {
                (Momento::Fecha(i), Some(Momento::Fecha(f))) => Some(texto_fecha(fecha + (f - i))),
                _ => None,
            };
            (texto_fecha(fecha), fin)
        }
    };

    let materia_id = match a.materia {
        Some(nombre) if nombre.trim().is_empty() => None,
        Some(nombre) => Some(buscar_materia(conexion, &nombre)?.id),
        None => anterior.materia_id,
    };
    let tipo = match a.tipo {
        Some(t) => tipo(Some(t))?,
        None => anterior.tipo,
    };

    let datos = DatosEvento {
        titulo: con_valor(a.titulo).unwrap_or(anterior.titulo),
        tipo,
        todo_el_dia: hora.is_none(),
        inicio,
        fin,
        materia_id,
        persona: a.persona.or(anterior.persona),
        notas: a.notas.or(anterior.notas),
        recurrencia: anterior.recurrencia,
        repetir_hasta: anterior.repetir_hasta,
    };
    if !a.aunque_se_superponga
        && let Some(choque) = choques(conexion, &datos.inicio, datos.fin.as_deref(), Some(a.id))?
    {
        return Ok(choque);
    }
    let evento = eventos::editar(conexion, a.id, datos, ahora).map_err(texto_error)?;
    Ok(Salida::ok(
        json!({ "editado": evento_para_modelo(conexion, &evento) }),
        &[Fuente::Agenda, Fuente::Responsabilidades],
    ))
}

#[derive(Deserialize)]
struct ArgsId {
    id: i64,
}

fn borrar_evento(conexion: &mut Connection, a: ArgsId) -> Paso<Salida> {
    let evento = eventos::obtener(conexion, a.id).map_err(texto_error)?;
    let serie = match evento.recurrencia {
        Recurrencia::Ninguna => "",
        Recurrencia::Semanal => " (con todas sus repeticiones)",
        Recurrencia::Anual => " (de todos los años)",
    };
    let pregunta = format!("¿Borro «{}» del {}{serie}?", evento.titulo, describir(&evento.inicio));
    Ok(Salida {
        para_modelo: json!({
            "esperando_confirmacion": pregunta,
            "nota": "Todavía no se borró. El usuario ve un botón para confirmar: preguntale si lo borrás."
        })
        .to_string(),
        cambios: Vec::new(),
        pendiente: Some(AccionPendiente::BorrarEvento { id: evento.id, titulo: evento.titulo, pregunta }),
    })
}

/// Ejecuta una acción ya confirmada por el usuario. Devuelve el texto de
/// respuesta y qué cambió.
pub fn ejecutar_pendiente(conexion: &mut Connection, accion: &AccionPendiente) -> Result<(String, Vec<Fuente>), Error> {
    match accion {
        AccionPendiente::BorrarEvento { id, .. } => {
            let evento = eventos::obtener(conexion, *id)?;
            eventos::borrar(conexion, *id)?;
            Ok((format!("Listo, borré «{}».", evento.titulo), vec![Fuente::Agenda, Fuente::Responsabilidades]))
        }
        // Los de archivos los hace `compu::herramientas::aplicar` (ver ciclo.rs).
        AccionPendiente::Archivo(_) => Err(Error::Sistema("Ese cambio lo hace la compu.".into())),
    }
}

#[derive(Deserialize)]
struct ArgsListarEventos {
    desde: String,
    hasta: Option<String>,
}

fn listar_eventos(conexion: &mut Connection, a: ArgsListarEventos, ahora: &Ahora) -> Paso<Salida> {
    let desde = resolver_fecha(&a.desde, ahora.hoy)?;
    let hasta = match con_valor(a.hasta) {
        Some(t) => resolver_fecha(&t, ahora.hoy)?,
        None => desde,
    };
    if hasta < desde || (hasta - desde).num_days() > 62 {
        return Err("El rango tiene que ir hacia adelante y durar como mucho dos meses.".into());
    }
    let lista = eventos::ocurrencias(conexion, desde, hasta).map_err(texto_error)?;
    let lista: Vec<Value> = lista
        .iter()
        .map(|o| {
            json!({
                "id": o.evento.id,
                "titulo": o.evento.titulo,
                "tipo": o.evento.tipo,
                "cuando": describir(&o.inicio),
                "materia": nombre_materia(conexion, o.evento.materia_id),
                "repite": o.evento.recurrencia != Recurrencia::Ninguna,
            })
        })
        .collect();
    Ok(Salida::ok(json!({ "desde": texto_fecha(desde), "hasta": texto_fecha(hasta), "eventos": lista }), &[]))
}

// ---------- Responsabilidades ----------

#[derive(Deserialize)]
struct ArgsCrearResponsabilidad {
    titulo: String,
    fecha_limite: Option<String>,
    prioridad: Option<String>,
    materia: Option<String>,
    evento_id: Option<i64>,
}

fn responsabilidad_para_modelo(conexion: &Connection, r: &Responsabilidad) -> Value {
    json!({
        "id": r.id,
        "titulo": r.titulo,
        "fecha_limite": r.fecha_limite.as_deref().map(describir),
        "prioridad": r.prioridad,
        "estado": r.estado,
        "materia": nombre_materia(conexion, r.materia_id),
    })
}

fn crear_responsabilidad(conexion: &mut Connection, a: ArgsCrearResponsabilidad, ahora: &Ahora) -> Paso<Salida> {
    let fecha_limite = match con_valor(a.fecha_limite) {
        Some(t) => Some(texto_fecha(resolver_fecha(&t, ahora.hoy)?)),
        None => None,
    };
    let prioridad = match con_valor(a.prioridad) {
        Some(p) => Prioridad::desde_texto(&normalizar(&p)).ok_or_else(|| format!("Prioridad inválida: «{p}»."))?,
        None => Prioridad::Media,
    };
    let materia_id = match con_valor(a.materia) {
        Some(nombre) => Some(buscar_materia(conexion, &nombre)?.id),
        None => None,
    };
    let datos = DatosResponsabilidad {
        titulo: a.titulo,
        descripcion: None,
        fecha_limite,
        prioridad,
        materia_id,
        evento_id: a.evento_id,
    };
    let creada = responsabilidades::crear(conexion, datos, OrigenResponsabilidad::Asistente, ahora)
        .map_err(texto_error)?;
    Ok(Salida::ok(
        json!({ "creada": responsabilidad_para_modelo(conexion, &creada) }),
        &[Fuente::Responsabilidades],
    ))
}

#[derive(Deserialize)]
struct ArgsCompletar {
    id: Option<i64>,
    texto: Option<String>,
}

fn completar_responsabilidad(conexion: &mut Connection, a: ArgsCompletar, ahora: &Ahora) -> Paso<Salida> {
    let id = match (a.id, con_valor(a.texto)) {
        (Some(id), _) => id,
        (None, Some(texto)) => {
            // Todas las palabras de la búsqueda tienen que estar en el título.
            let palabras: Vec<String> = normalizar(&texto).split_whitespace().map(String::from).collect();
            let pendientes = responsabilidades::listar(conexion).map_err(texto_error)?;
            let coinciden: Vec<&Responsabilidad> = pendientes
                .iter()
                .filter(|r| r.estado == EstadoResponsabilidad::Pendiente)
                .filter(|r| {
                    let titulo = normalizar(&r.titulo);
                    palabras.iter().all(|p| titulo.contains(p.as_str()))
                })
                .collect();
            match coinciden.as_slice() {
                [una] => una.id,
                [] => return Err(format!("No hay ninguna pendiente que diga «{texto}».")),
                varias => {
                    let opciones: Vec<Value> = varias.iter().map(|r| responsabilidad_para_modelo(conexion, r)).collect();
                    return Ok(Salida::ok(
                        json!({ "varias_coinciden": opciones, "nota": "Preguntale al usuario cuál." }),
                        &[],
                    ));
                }
            }
        }
        (None, None) => return Err("Decime el id o un texto para buscarla.".into()),
    };
    let hecha = responsabilidades::marcar(conexion, id, true, ahora).map_err(texto_error)?;
    Ok(Salida::ok(
        json!({ "completada": responsabilidad_para_modelo(conexion, &hecha) }),
        &[Fuente::Responsabilidades],
    ))
}

#[derive(Deserialize)]
struct ArgsListarResponsabilidades {
    filtro: Option<String>,
}

fn listar_responsabilidades(conexion: &mut Connection, a: ArgsListarResponsabilidades, ahora: &Ahora) -> Paso<Salida> {
    let filtro = con_valor(a.filtro).map(|f| normalizar(&f)).unwrap_or_else(|| "pendientes".into());
    let hoy = texto_fecha(ahora.hoy);
    let pendiente = |r: &Responsabilidad| r.estado == EstadoResponsabilidad::Pendiente;
    let todas = responsabilidades::listar(conexion).map_err(texto_error)?;
    let mut elegidas: Vec<&Responsabilidad> = match filtro.as_str() {
        "pendientes" => todas.iter().filter(|r| pendiente(r)).collect(),
        "hoy" => todas.iter().filter(|r| pendiente(r) && r.fecha_limite.as_deref() == Some(hoy.as_str())).collect(),
        "vencidas" => todas
            .iter()
            .filter(|r| pendiente(r) && r.fecha_limite.as_deref().is_some_and(|f| f < hoy.as_str()))
            .collect(),
        "hechas" => todas.iter().filter(|r| !pendiente(r)).collect(),
        "todas" => todas.iter().collect(),
        otro => return Err(format!("Filtro inválido: «{otro}».")),
    };
    // Primero las que tienen fecha, de la más cercana; las sin fecha al final.
    elegidas.sort_by(|a, b| (a.fecha_limite.is_none(), &a.fecha_limite).cmp(&(b.fecha_limite.is_none(), &b.fecha_limite)));
    let total = elegidas.len();
    let lista: Vec<Value> = elegidas.iter().take(30).map(|r| responsabilidad_para_modelo(conexion, r)).collect();
    Ok(Salida::ok(json!({ "total": total, "responsabilidades": lista }), &[]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn ahora() -> Ahora {
        Ahora::fijo_a_las("2026-11-17", "14:00")
    }

    fn correr(base: &mut Connection, nombre: &str, args: Value) -> (Value, Salida) {
        let salida = ejecutar(base, nombre, &args.to_string(), &ahora());
        let valor: Value = serde_json::from_str(&salida.para_modelo).unwrap();
        (valor, salida)
    }

    fn con_analisis() -> Connection {
        let base = db::en_memoria();
        materias::crear(&base, DatosMateria { nombre: "Análisis II".into(), color: "celeste".into() }, &ahora()).unwrap();
        base
    }

    #[test]
    fn acepta_null_en_los_campos_opcionales() {
        // El esquema lo permite, para que Groq no rechace la llamada…
        let definiciones = definiciones_de(Some(&["crear_responsabilidad"]));
        let propiedades = &definiciones[0]["function"]["parameters"]["properties"];
        assert_eq!(propiedades["titulo"]["type"], "string");
        assert_eq!(propiedades["materia"]["type"], json!(["string", "null"]));
        assert_eq!(propiedades["prioridad"]["enum"], json!(["baja", "media", "alta", null]));

        // …y Rust lo trata como si no viniera (también en los `bool`).
        let mut base = con_analisis();
        let (v, salida) = correr(
            &mut base,
            "crear_responsabilidad",
            json!({ "titulo": "Comprar el regalo de Juan", "fecha_limite": "viernes", "materia": null, "prioridad": null }),
        );
        assert!(v.get("error").is_none(), "{v}");
        assert_eq!(salida.cambios, [Fuente::Responsabilidades]);
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Dentista", "fecha": "jueves", "aunque_se_superponga": null }));
        assert!(v.get("error").is_none(), "{v}");
    }

    #[test]
    fn no_guarda_un_evento_que_se_pisa_con_otro_salvo_que_lo_quiera_igual() {
        let mut base = db::en_memoria();
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Entrenamiento", "tipo": "partido", "fecha": "jueves", "hora": "18:30", "hora_fin": "20:30", "repetir": "semanal" }));
        let entrenamiento = v["creado"]["id"].clone();

        let perro = json!({ "titulo": "Pasear al perro", "fecha": "jueves", "hora": "18", "hora_fin": "19" });
        let (v, salida) = correr(&mut base, "crear_evento", perro.clone());
        assert_eq!(v["no_guardado"], true);
        assert_eq!(v["se_superpone_con"][0]["titulo"], "Entrenamiento");
        assert_eq!(v["se_superpone_con"][0]["cuando"], "jueves 19 de noviembre a las 18:30");
        assert!(salida.cambios.is_empty());
        assert_eq!(eventos::listar_todos(&base).unwrap().len(), 1);

        // Justo antes no choca; y si el usuario lo quiere igual, se guarda.
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Merienda", "fecha": "jueves", "hora": "17", "hora_fin": "18:30" }));
        let merienda = v["creado"]["id"].clone();
        assert!(merienda.is_i64());
        let mut igual = perro;
        igual["aunque_se_superponga"] = json!(true);
        let (v, _) = correr(&mut base, "crear_evento", igual);
        assert!(v["creado"]["id"].is_i64());

        // Mover la merienda encima del entrenamiento tampoco se puede, y
        // editarla sin moverla no choca consigo misma.
        let (v, _) = correr(&mut base, "editar_evento", json!({ "id": merienda, "hora": "19" }));
        assert_eq!(v["no_guardado"], true);
        assert_ne!(v["se_superpone_con"][0]["id"], merienda);
        let (v, _) = correr(&mut base, "editar_evento", json!({ "id": entrenamiento, "titulo": "Entrenamiento italiano", "aunque_se_superponga": true }));
        assert!(v["editado"].is_object());
    }

    #[test]
    fn el_criterio_de_terminado_de_v0_3() {
        // "tengo parcial de Análisis el jueves a las 9"
        let mut base = con_analisis();
        let (valor, salida) = correr(
            &mut base,
            "crear_evento",
            json!({ "titulo": "Parcial de Análisis", "tipo": "examen", "fecha": "el jueves", "hora": "9", "materia": "análisis" }),
        );
        assert_eq!(valor["creado"]["inicio"], "2026-11-19T09:00:00-03:00");
        assert_eq!(valor["creado"]["cuando"], "jueves 19 de noviembre a las 09:00");
        assert_eq!(valor["creado"]["materia"], "Análisis II");
        assert_eq!(valor["responsabilidades_automaticas"], json!(["Estudiar para Análisis II"]));
        assert_eq!(salida.cambios, vec![Fuente::Agenda, Fuente::Responsabilidades]);

        let resp = responsabilidades::listar(&base).unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].fecha_limite.as_deref(), Some("2026-11-18"));
    }

    #[test]
    fn sin_hora_es_de_todo_el_dia_y_con_duracion_tiene_fin() {
        let mut base = db::en_memoria();
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Cumple de Juan", "tipo": "cumpleaños", "fecha": "20/11", "persona": "Juan" }));
        assert_eq!(v["creado"]["inicio"], "2026-11-20");
        assert_eq!(v["creado"]["tipo"], "cumpleanos");

        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Clase", "tipo": "clase", "fecha": "mañana", "hora": "3 de la tarde", "duracion_minutos": 90, "repetir": "semanal" }));
        assert_eq!(v["creado"]["inicio"], "2026-11-18T15:00:00-03:00");
        assert_eq!(v["creado"]["fin"], "2026-11-18T16:30:00-03:00");
        assert_eq!(v["creado"]["repite"], "semanal");

        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Fiesta", "fecha": "sábado", "hora": "22", "hora_fin": "2" }));
        assert_eq!(v["creado"]["fin"], "2026-11-22T02:00:00-03:00");
        assert_eq!(v["creado"]["tipo"], "otro");
    }

    #[test]
    fn materia_inexistente_o_ambigua_hace_preguntar() {
        let mut base = con_analisis();
        materias::crear(&base, DatosMateria { nombre: "Análisis I".into(), color: "coral".into() }, &ahora()).unwrap();
        let (v, salida) = correr(&mut base, "crear_evento", json!({ "titulo": "Parcial", "tipo": "examen", "fecha": "jueves", "hora": "9", "materia": "Análisis" }));
        assert!(v["error"].as_str().unwrap().contains("Preguntale al usuario cuál"));
        assert!(salida.cambios.is_empty());

        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Parcial", "tipo": "examen", "fecha": "jueves", "materia": "Química" }));
        assert!(v["error"].as_str().unwrap().contains("No existe la materia «Química»"));
        assert!(eventos::listar_todos(&base).unwrap().is_empty());
    }

    #[test]
    fn errores_de_fecha_y_de_argumentos_vuelven_como_texto() {
        let mut base = db::en_memoria();
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "X", "fecha": "el finde" }));
        assert!(v["error"].as_str().unwrap().contains("No entendí la fecha"));
        let (v, _) = correr(&mut base, "crear_evento", json!({ "fecha": "hoy" }));
        assert!(v["error"].as_str().unwrap().starts_with("Argumentos inválidos"));
        let (v, _) = correr(&mut base, "volar", json!({}));
        assert!(v["error"].as_str().unwrap().contains("No existe la herramienta"));
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "  ", "fecha": "hoy" }));
        assert_eq!(v["error"], "El evento necesita un título.");
    }

    #[test]
    fn editar_mueve_el_evento_y_conserva_la_duracion() {
        let mut base = con_analisis();
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Parcial", "tipo": "examen", "fecha": "jueves", "hora": "9", "hora_fin": "11", "materia": "Análisis II" }));
        let id = v["creado"]["id"].as_i64().unwrap();

        let (v, salida) = correr(&mut base, "editar_evento", json!({ "id": id, "fecha": "viernes", "hora": "10" }));
        assert_eq!(v["editado"]["inicio"], "2026-11-20T10:00:00-03:00");
        assert_eq!(v["editado"]["fin"], "2026-11-20T12:00:00-03:00");
        assert_eq!(v["editado"]["materia"], "Análisis II");
        assert!(salida.cambios.contains(&Fuente::Responsabilidades));
        // La automática se mueve con el evento.
        assert_eq!(responsabilidades::listar(&base).unwrap()[0].fecha_limite.as_deref(), Some("2026-11-19"));

        let (v, _) = correr(&mut base, "editar_evento", json!({ "id": id, "todo_el_dia": true, "materia": "" }));
        assert_eq!(v["editado"]["inicio"], "2026-11-20");
        assert_eq!(v["editado"]["fin"], Value::Null);
        assert_eq!(v["editado"]["materia"], Value::Null);

        let (v, _) = correr(&mut base, "editar_evento", json!({ "id": 999, "titulo": "x" }));
        assert_eq!(v["error"], "no existe ese evento");
    }

    #[test]
    fn borrar_no_borra_hasta_confirmar() {
        let mut base = db::en_memoria();
        let (v, _) = correr(&mut base, "crear_evento", json!({ "titulo": "Dentista", "fecha": "jueves", "hora": "16" }));
        let id = v["creado"]["id"].as_i64().unwrap();

        let (v, salida) = correr(&mut base, "borrar_evento", json!({ "id": id }));
        let pendiente = salida.pendiente.expect("queda pendiente");
        assert_eq!(pendiente.pregunta(), "¿Borro «Dentista» del jueves 19 de noviembre a las 16:00?");
        assert_eq!(v["esperando_confirmacion"], pendiente.pregunta());
        assert!(salida.cambios.is_empty());
        assert_eq!(eventos::listar_todos(&base).unwrap().len(), 1);

        let (texto, cambios) = ejecutar_pendiente(&mut base, &pendiente).unwrap();
        assert_eq!(texto, "Listo, borré «Dentista».");
        assert!(cambios.contains(&Fuente::Agenda));
        assert!(eventos::listar_todos(&base).unwrap().is_empty());
    }

    #[test]
    fn listar_eventos_por_rango() {
        let mut base = db::en_memoria();
        correr(&mut base, "crear_evento", json!({ "titulo": "A", "fecha": "mañana", "hora": "9" }));
        correr(&mut base, "crear_evento", json!({ "titulo": "B", "fecha": "el lunes" }));
        let (v, _) = correr(&mut base, "listar_eventos", json!({ "desde": "mañana" }));
        assert_eq!(v["eventos"].as_array().unwrap().len(), 1);
        assert_eq!(v["eventos"][0]["cuando"], "miércoles 18 de noviembre a las 09:00");
        let (v, _) = correr(&mut base, "listar_eventos", json!({ "desde": "hoy", "hasta": "en una semana" }));
        assert_eq!(v["eventos"].as_array().unwrap().len(), 2);
        let (v, _) = correr(&mut base, "listar_eventos", json!({ "desde": "hoy", "hasta": "ayer" }));
        assert!(v["error"].is_string());
    }

    #[test]
    fn responsabilidades_crear_completar_y_listar() {
        let mut base = db::en_memoria();
        let (v, salida) = correr(&mut base, "crear_responsabilidad", json!({ "titulo": "Comprar el regalo de Juan", "fecha_limite": "viernes", "prioridad": "alta" }));
        assert_eq!(v["creada"]["fecha_limite"], "viernes 20 de noviembre");
        assert_eq!(salida.cambios, vec![Fuente::Responsabilidades]);
        assert_eq!(responsabilidades::listar(&base).unwrap()[0].origen, OrigenResponsabilidad::Asistente);
        correr(&mut base, "crear_responsabilidad", json!({ "titulo": "Comprar pan", "fecha_limite": "ayer" }));
        correr(&mut base, "crear_responsabilidad", json!({ "titulo": "Llamar a mamá" }));

        let (v, _) = correr(&mut base, "listar_responsabilidades", json!({ "filtro": "vencidas" }));
        assert_eq!(v["total"], 1);
        let (v, _) = correr(&mut base, "listar_responsabilidades", json!({}));
        assert_eq!(v["total"], 3);
        assert_eq!(v["responsabilidades"][2]["titulo"], "Llamar a mamá"); // sin fecha, al final

        // "comprar" coincide con dos: pregunta.
        let (v, salida) = correr(&mut base, "completar_responsabilidad", json!({ "texto": "comprar" }));
        assert_eq!(v["varias_coinciden"].as_array().unwrap().len(), 2);
        assert!(salida.cambios.is_empty());

        let (v, _) = correr(&mut base, "completar_responsabilidad", json!({ "texto": "regalo juan" }));
        assert_eq!(v["completada"]["estado"], "hecha");
        let (v, _) = correr(&mut base, "completar_responsabilidad", json!({ "texto": "regalo juan" }));
        assert!(v["error"].as_str().unwrap().contains("No hay ninguna pendiente"));
        let (v, _) = correr(&mut base, "listar_responsabilidades", json!({ "filtro": "hechas" }));
        assert_eq!(v["total"], 1);
    }

    #[test]
    fn materias_crear_con_color_libre_y_sin_duplicar() {
        let mut base = con_analisis(); // usa celeste
        let (v, salida) = correr(&mut base, "crear_materia", json!({ "nombre": "Física" }));
        assert_eq!(v["creada"]["color"], "coral");
        assert_eq!(salida.cambios, vec![Fuente::Materias]);
        let (v, _) = correr(&mut base, "crear_materia", json!({ "nombre": "fisica" }));
        assert!(v["error"].as_str().unwrap().contains("Ya existe"));
        let (v, _) = correr(&mut base, "listar_materias", json!({}));
        assert_eq!(v["materias"].as_array().unwrap().len(), 2);
        // Argumentos vacíos ("") cuentan como "{}".
        let salida = ejecutar(&mut base, "listar_materias", "", &ahora());
        assert!(salida.para_modelo.contains("\"materias\""));
    }

    #[test]
    fn las_definiciones_tienen_las_catorce_herramientas() {
        let nombres: Vec<String> = definiciones()
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["function"]["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            nombres,
            [
                "crear_evento", "editar_evento", "borrar_evento", "listar_eventos", "crear_responsabilidad",
                "completar_responsabilidad", "listar_responsabilidades", "crear_materia", "listar_materias",
                "consultar_partidos", "consultar_tabla", "consultar_jugador", "consultar_noticias", "resumen_del_dia"
            ]
        );
        // Groq gratis tiene un límite de tokens por minuto: las definiciones
        // van en cada pedido, así que tienen que seguir siendo cortas.
        assert!(definiciones().to_string().len() < 6500, "{}", definiciones().to_string().len());
    }

    #[test]
    fn las_consultas_no_cambian_nada() {
        let mut base = db::en_memoria();
        let (v, salida) = correr(&mut base, "resumen_del_dia", json!({}));
        assert!(v["texto"].as_str().unwrap().contains("no tenés nada"));
        assert!(salida.cambios.is_empty());
    }
}
