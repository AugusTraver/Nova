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
use crate::db::Db;
use crate::dominio::lenguaje::interpretar_confirmacion;
use crate::dominio::texto::EstadoResponsabilidad;
use crate::error::{Resultado, invalido};
use crate::repos::{Ahora, eventos, materias, mensajes, responsabilidades};

/// Cuántas vueltas modelo ↔ herramientas como mucho por turno.
const MAXIMO_VUELTAS: usize = 6;
/// Historial que se manda: los últimos mensajes de las últimas horas.
const MENSAJES_DE_HISTORIAL: usize = 12;
const HORAS_DE_HISTORIAL: i64 = 2;
const LARGO_MAXIMO: usize = 2000;

/// Estado del asistente entre turnos (Tauri lo guarda con `app.manage`).
#[derive(Default)]
pub struct Asistente {
    /// Un borrado esperando el "sí". `Mutex` porque lo pueden tocar dos
    /// comandos a la vez.
    pendiente: Mutex<Option<AccionPendiente>>,
}

impl Asistente {
    pub fn hay_pendiente(&self) -> bool {
        self.pendiente.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    fn tomar_pendiente(&self) -> Option<AccionPendiente> {
        self.pendiente.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    fn dejar_pendiente(&self, accion: AccionPendiente) {
        *self.pendiente.lock().unwrap_or_else(|e| e.into_inner()) = Some(accion);
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
    };
    let desde = (ahora.momento - Duration::hours(HORAS_DE_HISTORIAL)).to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    let historial = mensajes::recientes(&conexion, MENSAJES_DE_HISTORIAL, &desde)?;
    Ok((contexto, historial))
}

/// Ejecuta la acción pendiente (o la cancela) y lo deja en el historial.
pub fn resolver_pendiente(db: &Db, estado: &Asistente, aceptar: bool, avisos: &dyn Avisos, ahora: &Ahora) -> Resultado<Turno> {
    let Some(accion) = estado.tomar_pendiente() else {
        return invalido("No hay nada esperando confirmación.");
    };
    let respuesta = if aceptar {
        let (texto, cambios) = herramientas::ejecutar_pendiente(&mut db.conexion(), &accion)?;
        avisos.cambios(&cambios);
        texto
    } else {
        "Dale, no toco nada.".to_string()
    };
    guardar(db, &Mensaje::Asistente { texto: respuesta.clone(), llamadas: vec![] }, ahora)?;
    Ok(Turno { respuesta, confirmacion: None })
}

/// Un turno completo a partir de lo que escribió el usuario.
pub async fn responder<P: ProveedorIa>(
    proveedor: &P,
    db: &Db,
    estado: &Asistente,
    texto: &str,
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

    // ¿Es la respuesta a un "¿Borro…?"? Si no lo es, la pregunta se descarta.
    if let Some(accion) = estado.tomar_pendiente()
        && let Some(aceptar) = interpretar_confirmacion(texto)
    {
        guardar(db, &Mensaje::Usuario(texto.to_string()), ahora)?;
        estado.dejar_pendiente(accion);
        return resolver_pendiente(db, estado, aceptar, avisos, ahora);
    }

    avisos.progreso(Progreso::Pensando);
    let (contexto, historial) = preparar(db, ahora)?;
    let usuario = Mensaje::Usuario(texto.to_string());
    guardar(db, &usuario, ahora)?;

    let mut conversacion = Vec::with_capacity(historial.len() + 4);
    conversacion.push(Mensaje::Sistema(prompt::armar(ahora, &contexto)));
    conversacion.extend(historial);
    conversacion.push(usuario);

    let definiciones = herramientas::definiciones();
    let mut confirmacion = None;

    for _ in 0..MAXIMO_VUELTAS {
        let respuesta = proveedor.completar(&conversacion, &definiciones).await?;

        if respuesta.llamadas.is_empty() {
            let texto = if respuesta.texto.is_empty() { "Listo.".to_string() } else { respuesta.texto };
            guardar(db, &Mensaje::Asistente { texto: texto.clone(), llamadas: vec![] }, ahora)?;
            return Ok(Turno { respuesta: texto, confirmacion });
        }

        let paso = Mensaje::Asistente { texto: respuesta.texto, llamadas: respuesta.llamadas.clone() };
        guardar(db, &paso, ahora)?;
        conversacion.push(paso);

        for llamada in respuesta.llamadas {
            avisos.progreso(Progreso::Herramienta { nombre: llamada.nombre.clone() });
            // El candado se suelta al terminar esta línea (antes del próximo await).
            let salida = herramientas::ejecutar(&mut db.conexion(), &llamada.nombre, &llamada.argumentos, ahora);
            if !salida.cambios.is_empty() {
                avisos.cambios(&salida.cambios);
            }
            if let Some(accion) = salida.pendiente {
                confirmacion = Some(accion.pregunta().to_string());
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
    Ok(Turno { respuesta: texto, confirmacion })
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
    }

    impl Guion {
        fn nuevo(respuestas: Vec<RespuestaModelo>) -> Self {
            Guion { respuestas: Mutex::new(respuestas.into()), recibido: Mutex::new(vec![]) }
        }
    }

    impl ProveedorIa for Guion {
        async fn completar(&self, mensajes: &[Mensaje], _: &Value) -> Resultado<RespuestaModelo> {
            self.recibido.lock().unwrap().push(mensajes.to_vec());
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
            llamadas: vec![Llamada { id: format!("id_{nombre}"), nombre: nombre.into(), argumentos: args.to_string() }],
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
        estado.dejar_pendiente(AccionPendiente::BorrarEvento { id: 1, pregunta: "¿Borro?".into() });
        let modelo = Guion::nuevo(vec![texto("Dale.")]);
        responder(&modelo, &db, &estado, "mejor contame qué tengo mañana", &Registro::default(), &ahora())
            .await
            .unwrap();
        assert!(estado.tomar_pendiente().is_none());
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
}
