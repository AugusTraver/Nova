//! Archivos (v0.7.1): encontrarlos dentro de las carpetas permitidas,
//! leerlos y preparar los cambios que esperan el "sí" (con su antes y
//! después y una copia para deshacer).
//!
//! Regla de oro: toda ruta se resuelve con `canonicalize` (que sigue `..`,
//! enlaces y uniones de carpetas hasta la ruta real) y DESPUÉS se pregunta si
//! está adentro de una carpeta permitida.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use super::sistema;
use crate::dominio::compu::{
    self, AntesYDespues, LineaCambio, Reemplazo, dentro_de_permitidas, se_puede_abrir_con_programa, se_puede_editar,
    sin_prefijo_largo,
};
use crate::dominio::lenguaje::normalizar;
use crate::repos::Ahora;
use crate::repos::cambios_archivos::{self, AccionArchivo};

/// Archivos de texto más grandes que esto no se leen ni se modifican.
const TAMANO_MAXIMO_TEXTO: u64 = 1024 * 1024;
/// Para borrar se guarda una copia: más grande que esto, no.
const TAMANO_MAXIMO_COPIA: u64 = 50 * 1024 * 1024;
/// Lo que le llega al modelo de un archivo (por los tokens).
pub const CARACTERES_PARA_EL_MODELO: usize = 3000;
const PROFUNDIDAD_DE_BUSQUEDA: u8 = 5;
const ENTRADAS_MAXIMAS: usize = 30_000;

/// Las carpetas permitidas, resueltas. Las que no existen se saltean.
pub fn resolver_carpetas(carpetas: &[String]) -> Vec<PathBuf> {
    carpetas.iter().filter_map(|c| Path::new(c).canonicalize().ok()).filter(|c| c.is_dir()).collect()
}

/// La ruta para mostrar: relativa a su carpeta permitida, con el nombre en
/// castellano ("Descargas\Facu\tp.md").
pub fn para_mostrar(ruta: &Path, permitidas: &[PathBuf]) -> String {
    let ruta = sin_prefijo_largo(ruta);
    for carpeta in permitidas {
        let carpeta = sin_prefijo_largo(carpeta);
        if let Ok(resto) = ruta.strip_prefix(&carpeta) {
            let base = sistema::nombre_para_mostrar(&carpeta);
            return if resto.as_os_str().is_empty() { base } else { format!("{base}\\{}", resto.display()) };
        }
    }
    ruta.display().to_string()
}

fn nombres(permitidas: &[PathBuf]) -> String {
    permitidas.iter().map(|c| sistema::nombre_para_mostrar(c)).collect::<Vec<_>>().join(", ")
}

/// Resuelve y verifica. Es la última puerta: todo lo que se lee, se abre o
/// se cambia pasa por acá.
pub fn verificar(ruta: &Path, permitidas: &[PathBuf]) -> Result<PathBuf, String> {
    let real = ruta.canonicalize().map_err(|_| "No encontré ese archivo.".to_string())?;
    if dentro_de_permitidas(&real, permitidas) {
        Ok(real)
    } else {
        Err("Eso está fuera de las carpetas que me dejaste usar, así que no lo toco.".into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buscado {
    Archivo,
    Carpeta,
    Cualquiera,
}

impl Buscado {
    fn acepta(self, es_carpeta: bool) -> bool {
        match self {
            Buscado::Archivo => !es_carpeta,
            Buscado::Carpeta => es_carpeta,
            Buscado::Cualquiera => true,
        }
    }
}

/// "la carpeta de descargas", "Downloads", "descargas" → la carpeta permitida.
fn carpeta_permitida_por_nombre(pedido: &str, permitidas: &[PathBuf]) -> Option<PathBuf> {
    let mut p = normalizar(pedido);
    for relleno in ["la carpeta de ", "carpeta de ", "la carpeta ", "carpeta ", "mis ", "el ", "la "] {
        if let Some(resto) = p.strip_prefix(relleno) {
            p = resto.to_string();
        }
    }
    permitidas
        .iter()
        .find(|c| {
            let disco = c.file_name().map(|n| normalizar(&n.to_string_lossy())).unwrap_or_default();
            p == disco || p == normalizar(&sistema::nombre_para_mostrar(c))
        })
        .cloned()
}

fn recorrer(carpeta: &Path, objetivo: &str, buscado: Buscado, profundidad: u8, vistas: &mut usize, salida: &mut Vec<PathBuf>) {
    let Ok(entradas) = std::fs::read_dir(carpeta) else { return };
    for entrada in entradas.flatten() {
        *vistas += 1;
        if *vistas > ENTRADAS_MAXIMAS || salida.len() >= 10 {
            return;
        }
        let Ok(tipo) = entrada.file_type() else { continue };
        // No se siguen enlaces ni uniones de carpetas al buscar (podrían
        // llevar afuera; igual se verificaría al final).
        if tipo.is_symlink() {
            continue;
        }
        let nombre = entrada.file_name().to_string_lossy().to_string();
        if nombre.starts_with('.') || nombre.eq_ignore_ascii_case("node_modules") {
            continue;
        }
        let ruta = entrada.path();
        let n = normalizar(&nombre);
        let sin_extension = Path::new(&nombre).file_stem().map(|s| normalizar(&s.to_string_lossy())).unwrap_or_default();
        let coincide = n == objetivo || (!objetivo.contains('.') && !tipo.is_dir() && sin_extension == objetivo);
        if coincide && buscado.acepta(tipo.is_dir()) {
            salida.push(ruta.clone());
        }
        if tipo.is_dir() && profundidad < PROFUNDIDAD_DE_BUSQUEDA {
            recorrer(&ruta, objetivo, buscado, profundidad + 1, vistas, salida);
        }
    }
}

/// Encuentra un archivo o carpeta por lo que dijo el usuario: un nombre
/// ("notas.txt", "notas"), una ruta dentro de una carpeta ("Facu\tp.md") o
/// una ruta completa. Si hay varios con ese nombre, pide que elija.
pub fn encontrar(pedido: &str, permitidas: &[PathBuf], buscado: Buscado) -> Result<PathBuf, String> {
    let pedido = pedido.trim().trim_matches(|c: char| c == '"' || c == '«' || c == '»' || c == '\'');
    if pedido.is_empty() {
        return Err("¿Qué archivo?".into());
    }
    if permitidas.is_empty() {
        return Err("No tengo ninguna carpeta permitida. Agregá una en Ajustes.".into());
    }
    if buscado != Buscado::Archivo {
        if let Some(c) = carpeta_permitida_por_nombre(pedido, permitidas) {
            return Ok(c);
        }
    }
    let como_ruta = Path::new(pedido);
    if como_ruta.is_absolute() {
        return verificar(como_ruta, permitidas);
    }
    if pedido.contains(['\\', '/']) {
        let encontrados: Vec<PathBuf> =
            permitidas.iter().map(|c| c.join(como_ruta)).filter_map(|r| verificar(&r, permitidas).ok()).collect();
        return elegir(pedido, encontrados, permitidas);
    }
    let objetivo = normalizar(pedido);
    let mut encontrados = Vec::new();
    let mut vistas = 0;
    for c in permitidas {
        recorrer(c, &objetivo, buscado, 0, &mut vistas, &mut encontrados);
    }
    let verificados = encontrados.iter().filter_map(|r| verificar(r, permitidas).ok()).collect();
    elegir(pedido, verificados, permitidas)
}

fn elegir(pedido: &str, mut encontrados: Vec<PathBuf>, permitidas: &[PathBuf]) -> Result<PathBuf, String> {
    encontrados.dedup();
    match encontrados.len() {
        0 => Err(format!("No encontré «{pedido}» en tus carpetas ({}).", nombres(permitidas))),
        1 => Ok(encontrados.remove(0)),
        _ => {
            let lugares: Vec<String> = encontrados.iter().take(4).map(|r| para_mostrar(r, permitidas)).collect();
            Err(format!("Encontré varios «{pedido}»: {}. ¿Cuál?", lugares.join(", ")))
        }
    }
}

// ---------- Texto ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texto {
    pub contenido: String,
    /// Si el archivo empezaba con la marca de UTF-8 (BOM): se respeta.
    pub bom: bool,
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn leer_texto(ruta: &Path) -> Result<Texto, String> {
    if !se_puede_editar(ruta) {
        return Err("Solo leo archivos de texto (.txt, .md, .csv…).".into());
    }
    let datos = std::fs::metadata(ruta).map_err(|_| "No encontré ese archivo.".to_string())?;
    if datos.len() > TAMANO_MAXIMO_TEXTO {
        return Err("Ese archivo es muy grande para leerlo (más de 1 MB).".into());
    }
    let bytes = std::fs::read(ruta).map_err(|e| format!("No lo pude leer: {e}"))?;
    let (bom, cuerpo) = match bytes.strip_prefix(BOM) {
        Some(resto) => (true, resto.to_vec()),
        None => (false, bytes),
    };
    let contenido = String::from_utf8(cuerpo).map_err(|_| "No es un texto que pueda leer (no está en UTF-8).".to_string())?;
    Ok(Texto { contenido, bom })
}

fn bytes_de(texto: &Texto) -> Vec<u8> {
    let mut bytes = if texto.bom { BOM.to_vec() } else { Vec::new() };
    bytes.extend_from_slice(texto.contenido.as_bytes());
    bytes
}

/// Una "huella" del contenido (FNV-1a de 64 bits y el largo): sirve para
/// saber si el archivo cambió desde que se mostró el antes y después. No es
/// criptográfica; acá alcanza.
pub fn huella(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}-{}", bytes.len())
}

fn huella_del_archivo(ruta: &Path) -> Option<String> {
    std::fs::read(ruta).ok().map(|b| huella(&b))
}

/// Escribe en un archivo temporal al lado y después lo pone en su lugar: si
/// algo falla en el medio, el original queda entero.
fn escribir(ruta: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporal = ruta.with_extension(format!("{}.nova-tmp", ruta.extension().map(|e| e.to_string_lossy()).unwrap_or_default()));
    std::fs::write(&temporal, bytes).map_err(|e| format!("No lo pude guardar: {e}"))?;
    std::fs::rename(&temporal, ruta).map_err(|e| {
        let _ = std::fs::remove_file(&temporal);
        format!("No lo pude guardar: {e}")
    })
}

/// Lo que se le pasa al modelo de un archivo: recortado y marcado como dato.
pub fn para_el_modelo(texto: &str) -> (String, bool) {
    let recortado = texto.chars().count() > CARACTERES_PARA_EL_MODELO;
    (texto.chars().take(CARACTERES_PARA_EL_MODELO).collect(), recortado)
}

// ---------- Cambios con confirmación ----------

/// Cómo se ve un cambio en pantalla, para confirmarlo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VistaCambio {
    pub archivo: String,
    pub carpeta: String,
    pub accion: &'static str,
    pub lineas: Vec<LineaCambio>,
    pub mas: usize,
    pub nota: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contenido {
    Texto(Texto),
    Borrar,
    /// Volver a poner la copia de antes, byte por byte.
    Copia(PathBuf),
}

/// Un cambio de archivo esperando el "sí" (va adentro de `AccionPendiente`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cambio {
    pub accion: AccionArchivo,
    pub ruta: PathBuf,
    pub contenido: Contenido,
    /// Cómo estaba al mostrar el cambio (`None` = no existía). Si cambió
    /// antes del "sí", no se toca.
    pub huella_antes: Option<String>,
    /// El cambio que se deshace, si es un "deshacé eso".
    pub deshace: Option<i64>,
    pub vista: VistaCambio,
    pub pregunta: String,
    /// Lo que dice Nova cuando lo hace.
    pub hecho: String,
}

fn vista(ruta: &Path, permitidas: &[PathBuf], accion: &'static str, ayd: AntesYDespues, nota: Option<String>) -> VistaCambio {
    let archivo = ruta.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let carpeta = ruta.parent().map(|c| para_mostrar(c, permitidas)).unwrap_or_default();
    VistaCambio { archivo, carpeta, accion, lineas: ayd.lineas, mas: ayd.mas, nota }
}

fn nombre(ruta: &Path) -> String {
    ruta.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// "en notas.txt cambiá martes por jueves" / "agregale una línea".
pub fn preparar_cambio(
    ruta: &Path,
    reemplazos: &[Reemplazo],
    agregar: Option<&str>,
    permitidas: &[PathBuf],
) -> Result<Cambio, String> {
    let ruta = verificar(ruta, permitidas)?;
    let actual = leer_texto(&ruta)?;
    let (nuevo, resumen) = match (reemplazos.is_empty(), agregar) {
        (false, _) => {
            let (mut texto, n) = compu::reemplazar(&actual.contenido, reemplazos)?;
            if let Some(linea) = agregar {
                texto = compu::agregar_linea(&texto, linea)?;
            }
            let primero = &reemplazos[0];
            let veces = if n > 1 { format!(" ({n} veces)") } else { String::new() };
            (texto, format!("cambié «{}» por «{}»{veces}", primero.buscar.trim(), primero.poner.trim()))
        }
        (true, Some(linea)) => (compu::agregar_linea(&actual.contenido, linea)?, "agregué la línea".to_string()),
        (true, None) => return Err("¿Qué le cambio?".into()),
    };
    let ayd = compu::antes_y_despues(&actual.contenido, &nuevo);
    let n = nombre(&ruta);
    Ok(Cambio {
        accion: AccionArchivo::Cambiar,
        huella_antes: Some(huella(&bytes_de(&actual))),
        contenido: Contenido::Texto(Texto { contenido: nuevo, bom: actual.bom }),
        deshace: None,
        vista: vista(&ruta, permitidas, "cambiar", ayd, None),
        pregunta: format!("¿Cambio {n} así?"),
        hecho: format!("Listo, en {n} {resumen}. Si querés, lo deshago."),
        ruta,
    })
}

/// "creá un archivo compras.txt con…". Va a la carpeta pedida o a Documentos
/// (la primera permitida). Nunca pisa uno que ya existe.
pub fn preparar_creacion(nombre_pedido: &str, contenido: &str, carpeta: Option<&str>, permitidas: &[PathBuf]) -> Result<Cambio, String> {
    let nombre_valido = compu::validar_nombre_nuevo(nombre_pedido)?;
    let destino = match carpeta.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => encontrar(c, permitidas, Buscado::Carpeta)?,
        None => permitidas.first().cloned().ok_or("No tengo ninguna carpeta permitida. Agregá una en Ajustes.")?,
    };
    let destino = verificar(&destino, permitidas)?;
    let ruta = destino.join(&nombre_valido);
    if ruta.exists() {
        return Err(format!("Ya hay un {nombre_valido} en {}. ¿Le cambio algo a ese o uso otro nombre?", para_mostrar(&destino, permitidas)));
    }
    let contenido = contenido.replace("\r\n", "\n");
    let contenido = if contenido.is_empty() || contenido.ends_with('\n') { contenido } else { format!("{contenido}\n") };
    if contenido.len() as u64 > TAMANO_MAXIMO_TEXTO {
        return Err("Es demasiado texto para un archivo nuevo.".into());
    }
    let ayd = compu::antes_y_despues("", &contenido);
    let lugar = para_mostrar(&destino, permitidas);
    Ok(Cambio {
        accion: AccionArchivo::Crear,
        huella_antes: None,
        contenido: Contenido::Texto(Texto { contenido, bom: false }),
        deshace: None,
        vista: vista(&ruta, permitidas, "crear", ayd, None),
        pregunta: format!("¿Creo {nombre_valido} en {lugar}?"),
        hecho: format!("Listo, creé {nombre_valido} en {lugar}."),
        ruta,
    })
}

/// "borrá notas-viejas.txt": va a la Papelera, y además queda una copia
/// para deshacer.
pub fn preparar_borrado(ruta: &Path, permitidas: &[PathBuf]) -> Result<Cambio, String> {
    let ruta = verificar(ruta, permitidas)?;
    if ruta.is_dir() {
        return Err("Carpetas enteras no borro: hacelo vos desde el Explorador.".into());
    }
    if !se_puede_abrir_con_programa(&ruta) {
        return Err("Ese tipo de archivo no lo borro.".into());
    }
    let tamano = std::fs::metadata(&ruta).map(|m| m.len()).unwrap_or(0);
    if tamano > TAMANO_MAXIMO_COPIA {
        return Err("Es muy grande para guardarle una copia: si lo querés borrar, hacelo vos.".into());
    }
    let ayd = match leer_texto(&ruta) {
        Ok(t) => compu::antes_y_despues(&t.contenido, ""),
        Err(_) => AntesYDespues { lineas: vec![], mas: 0 },
    };
    let n = nombre(&ruta);
    Ok(Cambio {
        accion: AccionArchivo::Borrar,
        huella_antes: huella_del_archivo(&ruta),
        contenido: Contenido::Borrar,
        deshace: None,
        vista: vista(&ruta, permitidas, "borrar", ayd, None),
        pregunta: format!("¿Mando {n} a la Papelera?"),
        hecho: format!("Listo, mandé {n} a la Papelera. Si querés, lo deshago."),
        ruta,
    })
}

/// "deshacé eso": arma el cambio inverso del último. También se confirma.
pub fn preparar_deshacer(conexion: &Connection, permitidas: &[PathBuf], ahora: &Ahora) -> Result<Cambio, String> {
    let ultimo = cambios_archivos::ultimo(conexion, ahora)
        .map_err(|e| e.to_string())?
        .ok_or("No tengo nada para deshacer (guardo los cambios de los últimos 7 días).")?;
    let ruta = PathBuf::from(&ultimo.ruta);
    let existe = ruta.exists();
    // La carpeta tiene que seguir permitida (pudiste sacarla en Ajustes).
    let carpeta = ruta.parent().ok_or("No encontré ese archivo.")?;
    verificar(carpeta, permitidas)?;
    if existe {
        verificar(&ruta, permitidas)?;
    }
    let actual = if existe { leer_texto(&ruta).ok() } else { None };
    let tocado_despues = existe && huella_del_archivo(&ruta) != ultimo.huella_despues;
    let nota = tocado_despues.then(|| "Ojo: el archivo cambió después de que lo toqué.".to_string());
    let n = nombre(&ruta);
    let (contenido, ayd, pregunta, hecho) = match ultimo.accion {
        AccionArchivo::Crear => {
            if !existe {
                return Err(format!("{n} ya no está."));
            }
            let ayd = compu::antes_y_despues(actual.as_ref().map_or("", |t| t.contenido.as_str()), "");
            (Contenido::Borrar, ayd, format!("¿Mando {n} a la Papelera, como antes de crearlo?"), format!("Listo, {n} fue a la Papelera."))
        }
        AccionArchivo::Cambiar | AccionArchivo::Borrar => {
            let copia = PathBuf::from(ultimo.copia.as_deref().ok_or("No tengo la copia de antes.")?);
            if !copia.exists() {
                return Err("La copia de antes ya no está.".into());
            }
            if ultimo.accion == AccionArchivo::Borrar && existe {
                return Err(format!("Ya hay un {n} de nuevo en esa carpeta: no lo piso."));
            }
            let anterior = std::fs::read(&copia).ok().and_then(|b| String::from_utf8(b.strip_prefix(BOM).unwrap_or(&b).to_vec()).ok());
            let ayd = match &anterior {
                Some(a) => compu::antes_y_despues(actual.as_ref().map_or("", |t| t.contenido.as_str()), a),
                None => AntesYDespues { lineas: vec![], mas: 0 },
            };
            let pregunta = if ultimo.accion == AccionArchivo::Borrar {
                format!("¿Vuelvo a poner {n}?")
            } else {
                format!("¿Dejo {n} como estaba?")
            };
            (Contenido::Copia(copia), ayd, pregunta, format!("Listo, {n} quedó como estaba."))
        }
        AccionArchivo::Deshacer => return Err("No tengo nada para deshacer.".into()),
    };
    Ok(Cambio {
        accion: AccionArchivo::Deshacer,
        huella_antes: if existe { huella_del_archivo(&ruta) } else { None },
        contenido,
        deshace: Some(ultimo.id),
        vista: vista(&ruta, permitidas, "deshacer", ayd, nota),
        pregunta,
        hecho,
        ruta,
    })
}

/// Un nombre de copia que no pise otra: dos cambios en el mismo milisegundo
/// (deshacer justo después de cambiar) no pueden compartir copia.
fn lugar_para_la_copia(carpeta_copias: &Path, ahora: &Ahora, nombre: &str) -> PathBuf {
    let base = ahora.momento.timestamp_millis();
    (0..)
        .map(|n| carpeta_copias.join(if n == 0 { format!("{base}-{nombre}") } else { format!("{base}-{n}-{nombre}") }))
        .find(|ruta| !ruta.exists())
        .unwrap_or_else(|| carpeta_copias.join(nombre))
}

/// Hace el cambio ya confirmado: primero la copia, después el cambio, y lo
/// anota para poder deshacerlo.
pub fn aplicar(cambio: &Cambio, conexion: &Connection, carpeta_copias: &Path, permitidas: &[PathBuf], ahora: &Ahora) -> Result<String, String> {
    // Se verifica de nuevo: entre la pregunta y el "sí" pudo cambiar algo.
    let carpeta = cambio.ruta.parent().ok_or("No encontré ese archivo.")?;
    verificar(carpeta, permitidas)?;
    let existe = cambio.ruta.exists();
    if existe {
        verificar(&cambio.ruta, permitidas)?;
    }
    if huella_del_archivo(&cambio.ruta) != cambio.huella_antes {
        return Err("El archivo cambió desde que te lo mostré, así que no lo toqué. Pedímelo de nuevo.".into());
    }
    let copia = if existe {
        std::fs::create_dir_all(carpeta_copias).map_err(|e| format!("No pude guardar la copia: {e}"))?;
        let destino = lugar_para_la_copia(carpeta_copias, ahora, &nombre(&cambio.ruta));
        std::fs::copy(&cambio.ruta, &destino).map_err(|e| format!("No pude guardar la copia, así que no lo toqué: {e}"))?;
        Some(destino)
    } else {
        None
    };
    match &cambio.contenido {
        Contenido::Texto(t) => escribir(&cambio.ruta, &bytes_de(t))?,
        Contenido::Copia(origen) => {
            let bytes = std::fs::read(origen).map_err(|e| format!("No pude leer la copia: {e}"))?;
            escribir(&cambio.ruta, &bytes)?;
        }
        Contenido::Borrar => sistema::a_la_papelera(&cambio.ruta)?,
    }
    let ruta = sin_prefijo_largo(&cambio.ruta).display().to_string();
    let copia_texto = copia.as_ref().map(|c| c.display().to_string());
    let despues = huella_del_archivo(&cambio.ruta);
    cambios_archivos::anotar(conexion, &ruta, cambio.accion, copia_texto.as_deref(), despues.as_deref(), ahora).map_err(|e| e.to_string())?;
    if let Some(id) = cambio.deshace {
        cambios_archivos::marcar_deshecho(conexion, id).map_err(|e| e.to_string())?;
    }
    if let Ok(viejas) = cambios_archivos::limpiar(conexion, ahora) {
        for v in viejas {
            let _ = std::fs::remove_file(v);
        }
    }
    Ok(cambio.hecho.clone())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db;

    /// Una carpeta de prueba con `Permitida\` (con archivos) y `Afuera\`.
    pub struct Prueba {
        pub raiz: PathBuf,
        pub permitidas: Vec<PathBuf>,
    }

    impl Prueba {
        pub fn nueva(nombre: &str) -> Self {
            let raiz = std::env::temp_dir().join(format!("nova-{nombre}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&raiz);
            std::fs::create_dir_all(raiz.join(r"Permitida\Facu")).unwrap();
            std::fs::create_dir_all(raiz.join("Afuera")).unwrap();
            std::fs::write(raiz.join(r"Permitida\notas.txt"), "Reunión el martes\r\nComprar pan\r\n").unwrap();
            std::fs::write(raiz.join(r"Permitida\Facu\tp.md"), "# TP\n").unwrap();
            std::fs::write(raiz.join(r"Afuera\secreto.txt"), "no").unwrap();
            let permitidas = resolver_carpetas(&[raiz.join("Permitida").display().to_string()]);
            Prueba { raiz, permitidas }
        }

        pub fn ruta(&self, relativa: &str) -> PathBuf {
            self.raiz.join(relativa)
        }
    }

    impl Drop for Prueba {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.raiz);
        }
    }

    #[test]
    fn encuentra_por_nombre_ruta_o_carpeta() {
        let p = Prueba::nueva("encontrar");
        let real = |r: &str| p.ruta(r).canonicalize().unwrap();
        assert_eq!(encontrar("notas.txt", &p.permitidas, Buscado::Archivo).unwrap(), real(r"Permitida\notas.txt"));
        assert_eq!(encontrar("Notas", &p.permitidas, Buscado::Archivo).unwrap(), real(r"Permitida\notas.txt"));
        assert_eq!(encontrar(r"Facu\tp.md", &p.permitidas, Buscado::Archivo).unwrap(), real(r"Permitida\Facu\tp.md"));
        assert_eq!(encontrar("facu", &p.permitidas, Buscado::Carpeta).unwrap(), real(r"Permitida\Facu"));
        assert_eq!(encontrar("la carpeta permitida", &p.permitidas, Buscado::Carpeta).unwrap(), real("Permitida"));
        assert!(encontrar("nada.txt", &p.permitidas, Buscado::Archivo).unwrap_err().contains("No encontré"));
    }

    #[test]
    fn no_sale_de_las_carpetas_permitidas() {
        let p = Prueba::nueva("afuera");
        // Por nombre no lo encuentra (no busca afuera)…
        assert!(encontrar("secreto.txt", &p.permitidas, Buscado::Archivo).is_err());
        // …ni con la ruta completa, ni con `..`.
        let completa = p.ruta(r"Afuera\secreto.txt").display().to_string();
        assert!(encontrar(&completa, &p.permitidas, Buscado::Archivo).unwrap_err().contains("fuera"));
        assert!(encontrar(r"..\Afuera\secreto.txt", &p.permitidas, Buscado::Archivo).is_err());
        let con_puntos = p.ruta(r"Permitida\..\Afuera\secreto.txt");
        assert!(verificar(&con_puntos, &p.permitidas).is_err());
        assert!(preparar_cambio(&con_puntos, &[Reemplazo { buscar: "no".into(), poner: "si".into() }], None, &p.permitidas).is_err());
        assert!(preparar_creacion("x.txt", "hola", Some(r"..\Afuera"), &p.permitidas).is_err());
        assert!(preparar_borrado(&p.ruta(r"Afuera\secreto.txt"), &p.permitidas).is_err());
        // Sin carpetas permitidas, nada.
        assert!(encontrar("notas.txt", &[], Buscado::Archivo).is_err());
    }

    /// Una unión de carpetas (lo que crea `mklink /J`, no pide permisos de
    /// administrador) adentro de la permitida que apunta afuera.
    #[test]
    #[cfg(windows)]
    fn un_enlace_no_saca_afuera() {
        let p = Prueba::nueva("enlace");
        let enlace = p.ruta(r"Permitida\atajo");
        let creado = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&enlace)
            .arg(p.ruta("Afuera"))
            .output()
            .unwrap();
        assert!(creado.status.success(), "{creado:?}");
        assert!(verificar(&enlace.join("secreto.txt"), &p.permitidas).is_err());
        assert!(encontrar(r"atajo\secreto.txt", &p.permitidas, Buscado::Archivo).is_err());
        assert!(encontrar("secreto.txt", &p.permitidas, Buscado::Archivo).is_err());
    }

    #[test]
    fn si_hay_dos_con_el_mismo_nombre_pregunta() {
        let p = Prueba::nueva("dos");
        std::fs::write(p.ruta(r"Permitida\Facu\notas.txt"), "otra").unwrap();
        let error = encontrar("notas.txt", &p.permitidas, Buscado::Archivo).unwrap_err();
        assert!(error.contains("varios") && error.contains("¿Cuál?"), "{error}");
    }

    #[test]
    fn lee_solo_texto_y_respeta_el_bom() {
        let p = Prueba::nueva("leer");
        std::fs::write(p.ruta(r"Permitida\bom.txt"), b"\xEF\xBB\xBFhola").unwrap();
        let t = leer_texto(&p.ruta(r"Permitida\bom.txt")).unwrap();
        assert_eq!((t.contenido.as_str(), t.bom), ("hola", true));
        std::fs::write(p.ruta(r"Permitida\foto.png"), b"\x89PNG").unwrap();
        assert!(leer_texto(&p.ruta(r"Permitida\foto.png")).is_err());
        std::fs::write(p.ruta(r"Permitida\latin1.txt"), b"caf\xe9").unwrap();
        assert!(leer_texto(&p.ruta(r"Permitida\latin1.txt")).is_err());
        let (texto, recortado) = para_el_modelo(&"a".repeat(5000));
        assert_eq!((texto.len(), recortado), (CARACTERES_PARA_EL_MODELO, true));
    }

    #[test]
    fn cambia_con_copia_y_se_puede_deshacer() {
        let p = Prueba::nueva("cambiar");
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-20");
        let copias = p.ruta("copias");
        let notas = p.ruta(r"Permitida\notas.txt");

        let cambio = preparar_cambio(&notas, &[Reemplazo { buscar: "martes".into(), poner: "jueves".into() }], None, &p.permitidas).unwrap();
        assert_eq!(cambio.pregunta, "¿Cambio notas.txt así?");
        assert_eq!(cambio.vista.lineas[0].marca, "martes");
        assert_eq!(cambio.vista.lineas[1].marca, "jueves");
        // Todavía no cambió nada.
        assert!(std::fs::read_to_string(&notas).unwrap().contains("martes"));

        let dicho = aplicar(&cambio, &base, &copias, &p.permitidas, &ahora).unwrap();
        assert!(dicho.contains("cambié «martes» por «jueves»"), "{dicho}");
        assert_eq!(std::fs::read_to_string(&notas).unwrap(), "Reunión el jueves\r\nComprar pan\r\n");
        assert_eq!(std::fs::read_dir(&copias).unwrap().count(), 1);

        let deshacer = preparar_deshacer(&base, &p.permitidas, &ahora).unwrap();
        assert_eq!(deshacer.pregunta, "¿Dejo notas.txt como estaba?");
        assert!(deshacer.vista.nota.is_none());
        aplicar(&deshacer, &base, &copias, &p.permitidas, &ahora).unwrap();
        assert_eq!(std::fs::read_to_string(&notas).unwrap(), "Reunión el martes\r\nComprar pan\r\n");
        // Ya se deshizo: no queda nada para deshacer.
        assert!(preparar_deshacer(&base, &p.permitidas, &ahora).is_err());
    }

    #[test]
    fn no_aplica_si_el_archivo_cambio_mientras_tanto() {
        let p = Prueba::nueva("carrera");
        let base = db::en_memoria();
        let notas = p.ruta(r"Permitida\notas.txt");
        let cambio = preparar_cambio(&notas, &[], Some("Llamar a mamá"), &p.permitidas).unwrap();
        std::fs::write(&notas, "lo cambié yo").unwrap();
        assert!(aplicar(&cambio, &base, &p.ruta("copias"), &p.permitidas, &Ahora::fijo("2026-11-20")).unwrap_err().contains("cambió"));
        assert_eq!(std::fs::read_to_string(&notas).unwrap(), "lo cambié yo");
    }

    #[test]
    fn crea_sin_pisar_y_deshacer_lo_saca() {
        let p = Prueba::nueva("crear");
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-20");
        assert!(preparar_creacion("notas.txt", "x", None, &p.permitidas).unwrap_err().contains("Ya hay"));
        let cambio = preparar_creacion("compras", "leche\nhuevos", Some("Facu"), &p.permitidas).unwrap();
        assert_eq!(cambio.vista.archivo, "compras.txt");
        assert_eq!(cambio.vista.carpeta, "Permitida\\Facu");
        aplicar(&cambio, &base, &p.ruta("copias"), &p.permitidas, &ahora).unwrap();
        let creado = p.ruta(r"Permitida\Facu\compras.txt");
        assert_eq!(std::fs::read_to_string(&creado).unwrap(), "leche\nhuevos\n");
        let deshacer = preparar_deshacer(&base, &p.permitidas, &ahora).unwrap();
        assert!(matches!(deshacer.contenido, Contenido::Borrar));
    }

    /// Manda a la Papelera de verdad (un archivo de prueba).
    #[test]
    #[ignore]
    fn borra_a_la_papelera_y_lo_vuelve_a_poner() {
        let p = Prueba::nueva("borrar");
        let base = db::en_memoria();
        let ahora = Ahora::fijo("2026-11-20");
        let notas = p.ruta(r"Permitida\notas.txt");
        let borrar = preparar_borrado(&notas, &p.permitidas).unwrap();
        aplicar(&borrar, &base, &p.ruta("copias"), &p.permitidas, &ahora).unwrap();
        assert!(!notas.exists());
        let volver = preparar_deshacer(&base, &p.permitidas, &ahora).unwrap();
        aplicar(&volver, &base, &p.ruta("copias"), &p.permitidas, &ahora).unwrap();
        assert!(std::fs::read_to_string(&notas).unwrap().contains("martes"));
    }
}
