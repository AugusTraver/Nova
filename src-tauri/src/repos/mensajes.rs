//! Historial de la conversación con el asistente (tabla `mensajes`).

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::Ahora;
use crate::asistente::mensajes::{Llamada, Mensaje};
use crate::error::Resultado;

/// Lo que va en `datos_json` de un mensaje de herramienta.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DatosHerramienta {
    llamada_id: String,
    nombre: String,
}

/// Los resultados de herramientas se guardan recortados: el modelo solo
/// necesita acordarse de qué pasó, no del listado entero.
const LARGO_MAXIMO_HERRAMIENTA: usize = 600;

fn recortar(texto: &str, largo: usize) -> String {
    if texto.chars().count() <= largo {
        return texto.to_string();
    }
    let mut corto: String = texto.chars().take(largo).collect();
    corto.push('…');
    corto
}

/// Guarda un mensaje. Los de sistema no se guardan (se arman en cada turno).
pub fn guardar(conexion: &Connection, mensaje: &Mensaje, ahora: &Ahora) -> Resultado<()> {
    let (rol, contenido, datos) = match mensaje {
        Mensaje::Sistema(_) => return Ok(()),
        Mensaje::Usuario(texto) => ("usuario", texto.clone(), None),
        Mensaje::Asistente { texto, llamadas } => {
            let datos = if llamadas.is_empty() { None } else { Some(serde_json::to_string(llamadas)?) };
            ("asistente", texto.clone(), datos)
        }
        Mensaje::Herramienta { llamada_id, nombre, contenido } => {
            let datos = DatosHerramienta { llamada_id: llamada_id.clone(), nombre: nombre.clone() };
            ("herramienta", recortar(contenido, LARGO_MAXIMO_HERRAMIENTA), Some(serde_json::to_string(&datos)?))
        }
    };
    conexion.execute(
        "INSERT INTO mensajes (rol, contenido, datos_json, creado_en) VALUES (?1, ?2, ?3, ?4)",
        params![rol, contenido, datos, ahora.marca],
    )?;
    Ok(())
}

/// Los últimos `limite` mensajes desde `desde` (marca ISO), en orden.
///
/// Siempre arranca en un mensaje del usuario: si el corte cae en medio de un
/// turno (una llamada a herramienta sin su pedido), esa parte se descarta,
/// porque los proveedores rechazan respuestas de herramientas sueltas.
pub fn recientes(conexion: &Connection, limite: usize, desde: &str) -> Resultado<Vec<Mensaje>> {
    let mut consulta = conexion.prepare(
        "SELECT rol, contenido, datos_json FROM mensajes WHERE creado_en >= ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let filas = consulta.query_map(params![desde, limite as i64], |f| {
        Ok((f.get::<_, String>(0)?, f.get::<_, String>(1)?, f.get::<_, Option<String>>(2)?))
    })?;

    let mut mensajes = Vec::new();
    for fila in filas {
        let (rol, contenido, datos) = fila?;
        let mensaje = match rol.as_str() {
            "usuario" => Mensaje::Usuario(contenido),
            "asistente" => {
                let llamadas: Vec<Llamada> = match datos {
                    Some(json) => serde_json::from_str(&json)?,
                    None => Vec::new(),
                };
                Mensaje::Asistente { texto: contenido, llamadas }
            }
            _ => {
                let Some(json) = datos else { continue };
                let d: DatosHerramienta = serde_json::from_str(&json)?;
                Mensaje::Herramienta { llamada_id: d.llamada_id, nombre: d.nombre, contenido }
            }
        };
        mensajes.push(mensaje);
    }
    mensajes.reverse();

    let inicio = mensajes.iter().position(|m| matches!(m, Mensaje::Usuario(_))).unwrap_or(mensajes.len());
    mensajes.drain(..inicio);
    Ok(mensajes)
}

pub fn borrar_todo(conexion: &Connection) -> Resultado<()> {
    conexion.execute("DELETE FROM mensajes", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn llamada(id: &str) -> Llamada {
        Llamada { id: id.into(), nombre: "listar_materias".into(), argumentos: "{}".into(), extra: None }
    }

    #[test]
    fn guarda_y_recupera_un_turno_con_herramientas() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        let turno = vec![
            Mensaje::Usuario("¿qué materias tengo?".into()),
            Mensaje::Asistente { texto: String::new(), llamadas: vec![llamada("c1")] },
            Mensaje::Herramienta { llamada_id: "c1".into(), nombre: "listar_materias".into(), contenido: "[]".into() },
            Mensaje::Asistente { texto: "No tenés ninguna.".into(), llamadas: vec![] },
        ];
        guardar(&base, &Mensaje::Sistema("no se guarda".into()), &ahora).unwrap();
        for m in &turno {
            guardar(&base, m, &ahora).unwrap();
        }
        assert_eq!(recientes(&base, 20, "2026-11-17T00:00:00-03:00").unwrap(), turno);
    }

    #[test]
    fn el_corte_empieza_en_un_mensaje_del_usuario() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        guardar(&base, &Mensaje::Usuario("hola".into()), &ahora).unwrap();
        guardar(&base, &Mensaje::Asistente { texto: String::new(), llamadas: vec![llamada("c1")] }, &ahora).unwrap();
        guardar(
            &base,
            &Mensaje::Herramienta { llamada_id: "c1".into(), nombre: "listar_materias".into(), contenido: "[]".into() },
            &ahora,
        )
        .unwrap();
        guardar(&base, &Mensaje::Asistente { texto: "Nada.".into(), llamadas: vec![] }, &ahora).unwrap();
        guardar(&base, &Mensaje::Usuario("gracias".into()), &ahora).unwrap();

        // Con 3 se cortaría en la herramienta: se descarta hasta "gracias".
        let ultimos = recientes(&base, 3, "2026-11-17T00:00:00-03:00").unwrap();
        assert_eq!(ultimos, vec![Mensaje::Usuario("gracias".into())]);
    }

    #[test]
    fn solo_los_recientes_y_se_puede_borrar() {
        let base = db::en_memoria();
        guardar(&base, &Mensaje::Usuario("viejo".into()), &Ahora::fijo_a_las("2026-11-17", "08:00")).unwrap();
        guardar(&base, &Mensaje::Usuario("nuevo".into()), &Ahora::fijo_a_las("2026-11-17", "11:00")).unwrap();
        let desde = "2026-11-17T10:00:00-03:00";
        assert_eq!(recientes(&base, 20, desde).unwrap(), vec![Mensaje::Usuario("nuevo".into())]);

        borrar_todo(&base).unwrap();
        assert!(recientes(&base, 20, "2000-01-01").unwrap().is_empty());
    }

    #[test]
    fn recorta_resultados_largos() {
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-17");
        guardar(&base, &Mensaje::Usuario("x".into()), &ahora).unwrap();
        let largo = "a".repeat(2000);
        guardar(
            &base,
            &Mensaje::Herramienta { llamada_id: "c".into(), nombre: "n".into(), contenido: largo },
            &ahora,
        )
        .unwrap();
        let Mensaje::Herramienta { contenido, .. } = &recientes(&base, 5, "2000").unwrap()[1] else { panic!() };
        assert_eq!(contenido.chars().count(), LARGO_MAXIMO_HERRAMIENTA + 1);
    }
}
