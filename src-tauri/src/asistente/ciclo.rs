//! El ciclo de un turno (SPEC 5.1): mensaje → modelo → herramientas → modelo
//! → … → respuesta final.
//!
//! La base se toma solo mientras se lee o se ejecuta una herramienta, nunca
//! mientras se espera al modelo: así los widgets siguen respondiendo.

use std::sync::Mutex;

use chrono::Duration;
use serde::Serialize;

use super::herramientas::{self, AccionPendiente, Fuente};
use super::mensajes::Mensaje;
use super::prompt::{self, Contexto, DIAS_DE_AGENDA};
use super::proveedor::ProveedorIa;
use super::seleccion;
use crate::compu::archivos::VistaCambio;
use crate::compu::{self, Compu};
use crate::db::Db;
use crate::dominio::lenguaje::sin_repetir;
use crate::dominio::texto::EstadoResponsabilidad;
use crate::error::{Resultado, invalido};
use crate::repos::{Ahora, eventos, materias, mensajes, preferencias, responsabilidades};

/// Cuántas vueltas modelo ↔ herramientas como mucho por turno.
const MAXIMO_VUELTAS: usize = 6;
/// Historial que se manda: los últimos mensajes de las últimas horas.
const MENSAJES_DE_HISTORIAL: usize = 12;
const HORAS_DE_HISTORIAL: i64 = 2;
const LARGO_MAXIMO: usize = 2000;

/// Estado del asistente entre turnos (Tauri lo guarda con `app.manage`).
pub struct Asistente {
    /// Un borrado esperando el "sí". `Mutex` porque lo pueden tocar dos
    /// comandos a la vez.
    pendiente: Mutex<Option<AccionPendiente>>,
    /// Lo que hace cosas en la compu (v0.7.1).
    pub compu: Compu,
}

/// En los tests, una compu que solo anota lo que le piden.
#[cfg(test)]
impl Default for Asistente {
    fn default() -> Self {
        let (compu, _) = Compu::de_prueba(Vec::new(), std::env::temp_dir().join("nova-copias-de-prueba"));
        Asistente::new(compu)
    }
}

impl Asistente {
    pub fn new(compu: Compu) -> Self {
        Asistente { pendiente: Mutex::default(), compu }
    }

    fn tomar_pendiente(&self) -> Option<AccionPendiente> {
        self.pendiente.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn dejar_pendiente(&self, accion: AccionPendiente) {
        *self.pendiente.lock().unwrap_or_else(|e| e.into_inner()) = Some(accion);
    }

    /// ¿El mensaje responde al borrado pendiente? (Sin sacarlo.)
    pub fn responde_al_pendiente(&self, texto: &str) -> bool {
        self.pendiente.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|a| a.respuesta(texto)).is_some()
    }
}

/// Avisos mientras corre el turno: para el texto debajo de la esfera y para
/// que los widgets recarguen apenas cambia algo (sin esperar el final).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Progreso {
    Pensando,
    Herramienta { nombre: String },
}

pub trait Avisos: Send + Sync {
    fn progreso(&self, progreso: Progreso);
    fn cambios(&self, fuentes: &[Fuente]);
}

/// Cómo terminó el turno.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Turno {
    pub respuesta: String,
    /// Si quedó un borrado esperando el "sí": la pregunta para los botones.
    pub confirmacion: Option<String>,
    /// Si lo que espera el "sí" es un cambio de archivo: su antes y después.
    pub cambio: Option<VistaCambio>,
}

fn guardar(db: &Db, mensaje: &Mensaje, ahora: &Ahora) -> Resultado<()> {
    mensajes::guardar(&db.conexion(), mensaje, ahora)
}

/// El contexto y el historial, leídos de una vez.
fn preparar(db: &Db, ahora: &Ahora) -> Resultado<(Contexto, Vec<Mensaje>)> {
    let conexion = db.conexion();
    let mut pendientes: Vec<_> = responsabilidades::listar(&conexion)?
        .into_iter()
        .filter(|r| r.estado == EstadoResponsabilidad::Pendiente)
        .collect();
    pendientes.sort_by(|a, b| (a.fecha_limite.is_none(), &a.fecha_limite).cmp(&(b.fecha_limite.is_none(), &b.fecha_limite)));
    let contexto = Contexto {
        materias: materias::listar(&conexion)?,
        eventos: eventos::ocurrencias(&conexion, ahora.hoy, ahora.hoy + Duration::days(DIAS_DE_AGENDA))?,
        pendientes,
        perfil: preferencias::perfil(&conexion)?,
    };
    let desde = (ahora.momento - Duration::hours(HORAS_DE_HISTORIAL)).to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    let historial = solo_lo_dicho(mensajes::recientes(&conexion, MENSAJES_DE_HISTORIAL, &desde)?);
    Ok((contexto, historial))
}

/// De los turnos anteriores alcanza con lo que se dijo: las llamadas a
/// herramientas y sus resultados (que pueden ser largos) ya no hacen falta y
/// se mandarían de nuevo en cada vuelta. Los ids siguen en la agenda.
fn solo_lo_dicho(historial: Vec<Mensaje>) -> Vec<Mensaje> {
    historial
        .into_iter()
        .filter(|m| match m {
            Mensaje::Usuario(_) => true,
            Mensaje::Asistente { texto, llamadas } => llamadas.is_empty() && !texto.is_empty(),
            _ => false,
        })
        .collect()
}

/// Ejecuta la acción pendiente (o la cancela) y lo deja en el historial.
pub fn resolver_pendiente(db: &Db, estado: &Asistente, aceptar: bool, avisos: &dyn Avisos, ahora: &Ahora) -> Resultado<Turno> {
    let Some(accion) = estado.tomar_pendiente() else {
        return invalido("No hay nada esperando confirmación.");
    };
    let respuesta = match (aceptar, &accion) {
        (false, _) => "Dale, no toco nada.".to_string(),
        // Un archivo que no se pudo cambiar no es un error del turno: Nova
        // lo dice ("El archivo cambió desde que te lo mostré…").
        (true, AccionPendiente::Archivo(cambio)) => {
            compu::herramientas::aplicar(&estado.compu, db, cambio, ahora).unwrap_or_else(|motivo| motivo)
        }
        (true, _) => {
            let (texto, cambios) = herramientas::ejecutar_pendiente(&mut db.conexion(), &accion)?;
            avisos.cambios(&cambios);
            texto
        }
    };
    guardar(db, &Mensaje::Asistente { texto: respuesta.clone(), llamadas: vec![] }, ahora)?;
    Ok(Turno { respuesta, confirmacion: None, cambio: None })
}

/// Un turno completo a partir de lo que escribió el usuario (los tests lo
/// usan así; la app pasa por `responder_con`).
#[cfg(test)]
pub async fn responder<P: ProveedorIa>(
    proveedor: &P,
    db: &Db,
    estado: &Asistente,
    texto: &str,
    avisos: &dyn Avisos,
    ahora: &Ahora,
) -> Resultado<Turno> {
    responder_con(proveedor, db, estado, texto, false, avisos, ahora).await
}

/// Igual que `responder`, pero `por_voz` pide una respuesta corta para leer
/// en voz alta (SPEC 5.2: "nada de párrafos largos por voz").
pub async fn responder_con<P: ProveedorIa>(
    proveedor: &P,
    db: &Db,
    estado: &Asistente,
    texto: &str,
    por_voz: bool,
    avisos: &dyn Avisos,
    ahora: &Ahora,
) -> Resultado<Turno> {
    let texto = texto.trim();
    if texto.is_empty() {
        return invalido("Escribime algo.");
    }
    if texto.chars().count() > LARGO_MAXIMO {
        return invalido("Es muy largo. Contámelo más corto.");
    }

    // ¿Es la respuesta a un "¿Borro…?"? Si no lo es, la pregunta se descarta
    // y se le avisa al modelo: si no, ve en el historial que pidió borrar y
    // contesta "listo, lo borré" sin haber borrado nada.
    let mut descartada = None;
    if let Some(accion) = estado.tomar_pendiente() {
        if let Some(aceptar) = accion.respuesta(texto) {
            guardar(db, &Mensaje::Usuario(texto.to_string()), ahora)?;
            estado.dejar_pendiente(accion);
            return resolver_pendiente(db, estado, aceptar, avisos, ahora);
        }
        descartada = Some(accion.descartada());
    }

    avisos.progreso(Progreso::Pensando);
    let (contexto, historial) = preparar(db, ahora)?;
    let ajustes_compu = preferencias::compu(&db.conexion())?;
    let usuario = Mensaje::Usuario(texto.to_string());
    guardar(db, &usuario, ahora)?;

    let mut conversacion = Vec::with_capacity(historial.len() + 4);
    let mut sistema = prompt::armar(ahora, &contexto);
    if por_voz {
        sistema.push_str(prompt::POR_VOZ);
    }
    let anterior = historial.iter().rev().find_map(|m| match m {
        Mensaje::Usuario(t) => Some(t.as_str()),
        _ => None,
    });
    let elegidas = seleccion::para_el_pedido(texto, anterior);
    let mut definiciones = herramientas::definiciones_de(elegidas.as_deref());
    // Las de la compu, solo si el tema las pide y el interruptor está prendido.
    let de_la_compu: Vec<&str> =
        elegidas.iter().flatten().copied().filter(|n| compu::herramientas::es_de_la_compu(n)).collect();
    if !de_la_compu.is_empty() && ajustes_compu.activa {
        if let Some(lista) = definiciones.as_array_mut() {
            lista.extend(compu::herramientas::definiciones(&de_la_compu, &ajustes_compu));
        }
        sistema.push_str(prompt::COMPU);
    }

    conversacion.push(Mensaje::Sistema(sistema));
    conversacion.extend(historial.iter().cloned());
    if let Some(aviso) = descartada {
        conversacion.push(Mensaje::Sistema(aviso));
    }
    conversacion.push(usuario);
    let mut confirmacion = None;
    let mut cambio = None;

    for _ in 0..MAXIMO_VUELTAS {
        let respuesta = proveedor.completar(&conversacion, &definiciones).await?;

        if respuesta.llamadas.is_empty() {
            let texto = if respuesta.texto.is_empty() { "Listo.".to_string() } else { sin_repetir(&respuesta.texto) };
            guardar(db, &Mensaje::Asistente { texto: texto.clone(), llamadas: vec![] }, ahora)?;
            return Ok(Turno { respuesta: texto, confirmacion, cambio });
        }

        let paso = Mensaje::Asistente { texto: respuesta.texto, llamadas: respuesta.llamadas.clone() };
        guardar(db, &paso, ahora)?;
        conversacion.push(paso);

        for llamada in respuesta.llamadas {
            avisos.progreso(Progreso::Herramienta { nombre: llamada.nombre.clone() });
            let salida = if compu::herramientas::es_de_la_compu(&llamada.nombre) {
                compu::herramientas::ejecutar(&estado.compu, db, &llamada.nombre, &llamada.argumentos, ahora).await
            } else {
                let argumentos = herramientas::con_horas_del_pedido(&llamada.nombre, &llamada.argumentos, texto);
                // El candado se suelta al terminar esta línea (antes del próximo await).
                herramientas::ejecutar(&mut db.conexion(), &llamada.nombre, &argumentos, ahora)
            };
            if !salida.cambios.is_empty() {
                avisos.cambios(&salida.cambios);
            }
            if let Some(accion) = salida.pendiente {
                confirmacion = Some(accion.pregunta().to_string());
                cambio = accion.vista().cloned();
                estado.dejar_pendiente(accion);
            }
            let resultado = Mensaje::Herramienta {
                llamada_id: llamada.id,
                nombre: llamada.nombre,
                contenido: salida.para_modelo,
            };
            guardar(db, &resultado, ahora)?;
            conversacion.push(resultado);
        }
        avisos.progreso(Progreso::Pensando);
    }

    let texto = "Me enredé un poco. ¿Me lo decís más simple?".to_string();
    guardar(db, &Mensaje::Asistente { texto: texto.clone(), llamadas: vec![] }, ahora)?;
    Ok(Turno { respuesta: texto, confirmacion, cambio })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde_json::{Value, json};

    use super::*;
    use crate::asistente::mensajes::{Llamada, RespuestaModelo};
    use crate::repos::materias::DatosMateria;

    /// Un "modelo" con respuestas escritas de antemano. Guarda lo que recibe.
    struct Guion {
        respuestas: Mutex<VecDeque<RespuestaModelo>>,
        recibido: Mutex<Vec<Vec<Mensaje>>>,
        /// Los nombres de las herramientas que recibió en cada vuelta.
        herramientas: Mutex<Vec<Vec<String>>>,
    }

    impl Guion {
        fn nuevo(respuestas: Vec<RespuestaModelo>) -> Self {
            Guion { respuestas: Mutex::new(respuestas.into()), recibido: Mutex::new(vec![]), herramientas: Mutex::new(vec![]) }
        }
    }

    impl ProveedorIa for Guion {
        async fn completar(&self, mensajes: &[Mensaje], herramientas: &Value) -> Resultado<RespuestaModelo> {
            self.recibido.lock().unwrap().push(mensajes.to_vec());
            let nombres = herramientas.as_array().into_iter().flatten();
            let nombres = nombres.filter_map(|h| h["function"]["name"].as_str().map(String::from)).collect();
            self.herramientas.lock().unwrap().push(nombres);
            Ok(self.respuestas.lock().unwrap().pop_front().expect("el guion se quedó sin respuestas"))
        }
    }

    #[derive(Default)]
    struct Registro {
        progreso: Mutex<Vec<Progreso>>,
        cambios: Mutex<Vec<Fuente>>,
    }

    impl Avisos for Registro {
        fn progreso(&self, p: Progreso) {
            self.progreso.lock().unwrap().push(p);
        }
        fn cambios(&self, f: &[Fuente]) {
            self.cambios.lock().unwrap().extend_from_slice(f);
        }
    }

    fn texto(t: &str) -> RespuestaModelo {
        RespuestaModelo { texto: t.into(), llamadas: vec![] }
    }

    fn llama(nombre: &str, args: Value) -> RespuestaModelo {
        RespuestaModelo {
            texto: String::new(),
            llamadas: vec![Llamada { id: format!("id_{nombre}"), nombre: nombre.into(), argumentos: args.to_string(), extra: None }],
        }
    }

    fn ahora() -> Ahora {
        Ahora::fijo_a_las("2026-11-17", "14:00")
    }

    fn base_con_analisis() -> Db {
        let db = Db::de_conexion(crate::db::en_memoria());
        materias::crear(&db.conexion(), DatosMateria { nombre: "Análisis".into(), color: "celeste".into() }, &ahora())
            .unwrap();
        db
    }

    #[tokio::test]
    async fn tengo_parcial_de_analisis_el_jueves_a_las_9() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let modelo = Guion::nuevo(vec![
            llama("crear_evento", json!({ "titulo": "Parcial de Análisis", "tipo": "examen", "fecha": "el jueves", "hora": "9", "materia": "Análisis" })),
            texto("Listo, parcial de Análisis el jueves 19 a las 9. Te anoté «Estudiar para Análisis» para el miércoles."),
        ]);

        let turno = responder(&modelo, &db, &estado, "tengo parcial de Análisis el jueves a las 9", &avisos, &ahora())
            .await
            .unwrap();

        assert!(turno.respuesta.starts_with("Listo, parcial"));
        assert_eq!(turno.confirmacion, None);
        let conexion = db.conexion();
        let evento = &eventos::listar_todos(&conexion).unwrap()[0];
        assert_eq!(evento.inicio, "2026-11-19T09:00:00-03:00");
        let resp = responsabilidades::listar(&conexion).unwrap();
        assert_eq!(resp[0].titulo, "Estudiar para Análisis");

        // Los widgets se enteran apenas se crea, y la esfera ve el progreso.
        assert_eq!(*avisos.cambios.lock().unwrap(), [Fuente::Agenda, Fuente::Responsabilidades]);
        assert_eq!(
            *avisos.progreso.lock().unwrap(),
            [Progreso::Pensando, Progreso::Herramienta { nombre: "crear_evento".into() }, Progreso::Pensando]
        );

        // El modelo recibió el resultado de la herramienta en la segunda vuelta.
        let recibido = modelo.recibido.lock().unwrap();
        let Mensaje::Sistema(sistema) = &recibido[0][0] else { panic!() };
        assert!(sistema.contains("martes 17 de noviembre"));
        let Some(Mensaje::Herramienta { contenido, .. }) = recibido[1].last() else { panic!() };
        assert!(contenido.contains("Estudiar para Análisis"));

        // Y todo quedó en el historial: usuario, llamada, resultado, respuesta.
        assert_eq!(mensajes::recientes(&conexion, 20, "2000").unwrap().len(), 4);
    }

    #[tokio::test]
    async fn manda_el_historial_reciente() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let modelo = Guion::nuevo(vec![texto("¡Hola!"), texto("Bien, ¿vos?")]);
        responder(&modelo, &db, &estado, "hola", &avisos, &ahora()).await.unwrap();
        responder(&modelo, &db, &estado, "¿cómo andás?", &avisos, &ahora()).await.unwrap();

        let recibido = modelo.recibido.lock().unwrap();
        assert_eq!(
            recibido[1][1..],
            [
                Mensaje::Usuario("hola".into()),
                Mensaje::Asistente { texto: "¡Hola!".into(), llamadas: vec![] },
                Mensaje::Usuario("¿cómo andás?".into()),
            ]
        );
    }

    #[tokio::test]
    async fn manda_solo_las_herramientas_del_tema_y_no_repite_resultados_viejos() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let modelo = Guion::nuevo(vec![
            llama("listar_materias", json!({})),
            texto("Tenés Análisis."),
            texto("¡Hola!"),
            texto("El domingo."),
        ]);
        responder(&modelo, &db, &estado, "¿qué materias tengo?", &avisos, &ahora()).await.unwrap();
        responder(&modelo, &db, &estado, "hola", &avisos, &ahora()).await.unwrap();
        responder(&modelo, &db, &estado, "¿cuándo juega Boca?", &avisos, &ahora()).await.unwrap();

        let herramientas = modelo.herramientas.lock().unwrap();
        assert_eq!(herramientas[0], ["crear_materia", "listar_materias"]);
        assert!(herramientas[2].is_empty(), "un saludo no lleva herramientas");
        assert_eq!(herramientas[3], ["consultar_partidos", "consultar_tabla", "consultar_jugador"]);

        // En el historial del último pedido no vuelven la llamada ni su resultado.
        let recibido = modelo.recibido.lock().unwrap();
        assert!(recibido[3].iter().all(|m| !matches!(m, Mensaje::Herramienta { .. })));
        assert!(recibido[3].contains(&Mensaje::Asistente { texto: "Tenés Análisis.".into(), llamadas: vec![] }));
    }

    #[tokio::test]
    async fn borrar_espera_el_si_y_el_modelo_no_puede_saltearlo() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let id = {
            let mut datos = eventos::tests::datos("Dentista", crate::dominio::texto::TipoEvento::Otro, "2026-11-19T16:00:00-03:00");
            datos.materia_id = None;
            eventos::crear(&mut db.conexion(), datos, &ahora()).unwrap().id
        };
        let modelo = Guion::nuevo(vec![llama("borrar_evento", json!({ "id": id })), texto("¿Lo borro?")]);

        let turno = responder(&modelo, &db, &estado, "borrá el dentista", &avisos, &ahora()).await.unwrap();
        assert_eq!(turno.confirmacion.as_deref(), Some("¿Borro «Dentista» del jueves 19 de noviembre a las 16:00?"));
        assert_eq!(eventos::listar_todos(&db.conexion()).unwrap().len(), 1);

        // "sí" lo resuelve Rust, sin pasar por el modelo (el guion está vacío).
        let turno = responder(&modelo, &db, &estado, "Sí", &avisos, &ahora()).await.unwrap();
        assert_eq!(turno.respuesta, "Listo, borré «Dentista».");
        assert!(eventos::listar_todos(&db.conexion()).unwrap().is_empty());
        assert!(avisos.cambios.lock().unwrap().contains(&Fuente::Agenda));

        // Ya no queda nada pendiente.
        assert!(resolver_pendiente(&db, &estado, true, &avisos, &ahora()).is_err());
    }

    #[tokio::test]
    async fn otro_mensaje_descarta_la_confirmacion() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        estado.dejar_pendiente(AccionPendiente::BorrarEvento { id: 1, titulo: "Dentista".into(), pregunta: "¿Borro?".into() });
        let modelo = Guion::nuevo(vec![texto("Dale.")]);
        responder(&modelo, &db, &estado, "mejor contame qué tengo mañana", &Registro::default(), &ahora())
            .await
            .unwrap();
        assert!(estado.tomar_pendiente().is_none());
        // Al modelo se le avisa que no se borró, justo antes del mensaje nuevo.
        let recibido = modelo.recibido.lock().unwrap();
        let conversacion = &recibido[0];
        let Mensaje::Sistema(aviso) = &conversacion[conversacion.len() - 2] else { panic!("falta el aviso") };
        assert!(aviso.contains("«Dentista»") && aviso.contains("NO está borrado"));
    }

    #[tokio::test]
    async fn volver_a_pedir_borrar_lo_mismo_lo_borra() {
        let db = base_con_analisis();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let mut datos = eventos::tests::datos("Pasear al perro", crate::dominio::texto::TipoEvento::Otro, "2026-11-19T18:00:00-03:00");
        datos.materia_id = None;
        let id = eventos::crear(&mut db.conexion(), datos, &ahora()).unwrap().id;
        let modelo = Guion::nuevo(vec![llama("borrar_evento", json!({ "id": id })), texto("¿Lo borro?")]);
        responder(&modelo, &db, &estado, "borrá lo de pasear al perro", &avisos, &ahora()).await.unwrap();

        // Lo pide otra vez en vez de decir "sí": se borra sin pasar por el modelo.
        assert!(estado.responde_al_pendiente("Eliminar la tarea, pasear al perro."));
        let turno = responder(&modelo, &db, &estado, "Eliminar la tarea, pasear al perro.", &avisos, &ahora()).await.unwrap();
        assert_eq!(turno.respuesta, "Listo, borré «Pasear al perro».");
        assert!(eventos::listar_todos(&db.conexion()).unwrap().is_empty());
    }

    #[tokio::test]
    async fn corta_si_el_modelo_no_para_de_llamar_herramientas() {
        let db = base_con_analisis();
        let vueltas = (0..MAXIMO_VUELTAS).map(|_| llama("listar_materias", json!({}))).collect();
        let modelo = Guion::nuevo(vueltas);
        let turno = responder(&modelo, &db, &Asistente::default(), "?", &Registro::default(), &ahora()).await.unwrap();
        assert!(turno.respuesta.contains("Me enredé"));
    }

    #[tokio::test]
    async fn rechaza_mensajes_vacios_o_enormes() {
        let db = base_con_analisis();
        let modelo = Guion::nuevo(vec![]);
        let avisos = Registro::default();
        assert!(responder(&modelo, &db, &Asistente::default(), "   ", &avisos, &ahora()).await.is_err());
        let largo = "a".repeat(LARGO_MAXIMO + 1);
        assert!(responder(&modelo, &db, &Asistente::default(), &largo, &avisos, &ahora()).await.is_err());
    }

    /// El criterio de v0.5 contra el modelo de verdad (Groq, con la key del
    /// llavero) y una base en memoria: "recordame comprar el regalo de Juan
    /// para el viernes" crea la responsabilidad y la confirma corto, como
    /// para leerla en voz alta. `cargo test regalo_de_juan -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn recordame_el_regalo_de_juan_por_voz() {
        let db = Db::de_conexion(crate::db::en_memoria());
        let ajustes = crate::asistente::proveedor::AjustesIa::default();
        let key = crate::llavero::leer("groq-api-key").unwrap();
        let cliente = ajustes.cliente_de(ajustes.proveedor, key).unwrap();
        let avisos = Registro::default();
        let turno = responder_con(
            &cliente,
            &db,
            &Asistente::default(),
            "Recordame comprar el regalo de Juan para el viernes.",
            true,
            &avisos,
            &ahora(),
        )
        .await
        .unwrap();
        let lista = crate::repos::responsabilidades::listar(&db.conexion()).unwrap();
        let creada = lista.iter().find(|r| r.titulo.to_lowercase().contains("regalo")).expect("no creó la responsabilidad");
        assert_eq!(creada.fecha_limite.as_deref(), Some("2026-11-20"));
        // Una oración corta, sin listas ni markdown.
        assert!(turno.respuesta.chars().count() < 160, "{}", turno.respuesta);
        assert!(!turno.respuesta.contains('*') && !turno.respuesta.contains("\n"), "{}", turno.respuesta);
        println!("Respuesta por voz: {}", turno.respuesta);
    }

    /// Lo mismo contra Gemini de verdad (v0.7.3), con la key del llavero:
    /// prueba que entiende las herramientas y devuelve bien su firma en la
    /// segunda vuelta. Gasta 2 o 3 de los 500 pedidos del día.
    /// `cargo test con_gemini -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn con_gemini_crea_el_recordatorio() {
        use crate::asistente::proveedor::{AjustesIa, TipoProveedor};
        let db = Db::de_conexion(crate::db::en_memoria());
        let key = crate::llavero::leer("gemini-api-key").unwrap().expect("falta la key de Gemini en el llavero");
        let cliente = AjustesIa::default().cliente_de(TipoProveedor::Gemini, Some(key)).unwrap();
        let turno = responder_con(
            &cliente,
            &db,
            &Asistente::default(),
            "Recordame comprar el regalo de Juan para el viernes.",
            true,
            &Registro::default(),
            &ahora(),
        )
        .await
        .unwrap();
        let lista = crate::repos::responsabilidades::listar(&db.conexion()).unwrap();
        assert!(lista.iter().any(|r| r.titulo.to_lowercase().contains("regalo")), "no creó la responsabilidad");
        println!("Gemini: {}", turno.respuesta);
    }

    /// Cuántos tokens gasta cada pedido típico contra Groq de verdad (el plan
    /// gratis cuenta tokens por minuto). Espera entre pedidos para no chocar
    /// con el límite. `cargo test tokens_por_pedido -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn tokens_por_pedido() {
        let db = base_con_analisis();
        let ajustes = crate::asistente::proveedor::AjustesIa::default();
        let key = crate::llavero::leer("groq-api-key").unwrap();
        let cliente = ajustes.cliente_de(ajustes.proveedor, key).unwrap();
        let estado = Asistente::default();
        let avisos = Registro::default();
        let pedidos = [
            "hola, ¿cómo andás?",
            "tengo parcial de Análisis el jueves a las 9",
            "¿qué tengo mañana?",
            "¿cuándo juega Boca?",
            "recordame comprar el regalo de Juan para el viernes",
        ];
        let mut total = 0;
        for (i, pedido) in pedidos.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            }
            let antes = cliente.tokens_usados();
            let turno = responder(&cliente, &db, &estado, pedido, &avisos, &ahora()).await;
            let gasto = cliente.tokens_usados() - antes;
            total += gasto;
            println!("{gasto:>6} tokens  «{pedido}» → {:?}", turno.map(|t| t.respuesta));
        }
        println!("{total:>6} tokens en total ({} por pedido)", total / pedidos.len() as u64);
    }

    /// Lo mismo con los pedidos de la compu (v0.7.1). La compu es de prueba:
    /// no abre nada, y el archivo está en una carpeta temporal.
    /// `cargo test tokens_de_la_compu -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn tokens_de_la_compu() {
        let db = base_con_analisis();
        let prueba = crate::compu::archivos::tests::Prueba::nueva("tokens");
        let ajustes_compu = crate::compu::AjustesCompu {
            carpetas: prueba.permitidas.iter().map(|c| c.display().to_string()).collect(),
            ..Default::default()
        };
        preferencias::guardar(&db.conexion(), preferencias::COMPU, &ajustes_compu).unwrap();
        let programas = vec![crate::compu::programas::Programa {
            nombre: "Visual Studio Code".into(),
            destino: "Visual Studio Code.lnk".into(),
        }];
        let (compu, manos) = Compu::de_prueba(programas, prueba.ruta("copias"));
        let estado = Asistente::new(compu);
        let cliente = crate::asistente::proveedor::AjustesIa::default()
            .cliente_de(crate::asistente::proveedor::TipoProveedor::Groq, crate::llavero::leer("groq-api-key").unwrap())
            .unwrap();
        let avisos = Registro::default();
        let pedidos = [
            "buscame en Google recetas de pizza",
            "abrí Visual Studio Code",
            "en notas.txt cambiá martes por jueves",
        ];
        let mut total = 0;
        for (i, pedido) in pedidos.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            }
            let antes = cliente.tokens_usados();
            let turno = responder(&cliente, &db, &estado, pedido, &avisos, &ahora()).await;
            let gasto = cliente.tokens_usados() - antes;
            total += gasto;
            println!("{gasto:>6} tokens  «{pedido}» → {:?}", turno.map(|t| (t.respuesta, t.cambio.is_some())));
        }
        println!("{total:>6} tokens en total ({} por pedido)", total / pedidos.len() as u64);
        println!("La compu de prueba hizo: {:?}", manos.0.lock().unwrap());
    }
}
