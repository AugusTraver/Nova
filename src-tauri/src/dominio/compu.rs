//! Reglas puras del acceso a la compu (v0.7.1, SPEC 5.9): qué direcciones se
//! abren, qué rutas están dentro de las carpetas permitidas, qué archivos se
//! pueden tocar, cómo se aplica un cambio de texto y cómo se arma el "antes y
//! después" que se muestra para confirmar.
//!
//! Nada de esto toca el disco ni abre nada: recibe rutas ya resueltas
//! (`canonicalize`) y textos, así se testea solo.

use std::path::{Path, PathBuf};

use url::{ParseError, Url};
use serde::Serialize;

// ---------- Direcciones ----------

const LARGO_MAXIMO_DIRECCION: usize = 2000;
const LARGO_MAXIMO_BUSQUEDA: usize = 300;

/// Valida una dirección para abrirla en el navegador. Solo `http` y `https`,
/// con un nombre de sitio y sin usuario ni contraseña adentro. Si no trae
/// esquema ("utn.edu.ar/campus"), se le pone `https://`.
pub fn validar_direccion(texto: &str) -> Result<String, String> {
    const NO_SE_PUEDE: &str = "Esa dirección no se puede abrir: solo abro páginas web (http o https).";
    let texto = texto.trim();
    if texto.is_empty() {
        return Err("Me falta la dirección.".into());
    }
    if texto.len() > LARGO_MAXIMO_DIRECCION || texto.chars().any(char::is_whitespace) {
        return Err(NO_SE_PUEDE.into());
    }
    let url = match Url::parse(texto) {
        Ok(url) if url.scheme() == "http" || url.scheme() == "https" => url,
        // "localhost:3000" o "www.sitio.com:8080" se leen como si
        // "localhost" fuera el esquema: si no hay "://", se prueba como sitio.
        Ok(url) if !texto.contains("://") && parece_sitio(url.scheme()) => {
            Url::parse(&format!("https://{texto}")).map_err(|_| NO_SE_PUEDE.to_string())?
        }
        Ok(_) => return Err(NO_SE_PUEDE.into()),
        Err(ParseError::RelativeUrlWithoutBase) => {
            Url::parse(&format!("https://{texto}")).map_err(|_| NO_SE_PUEDE.to_string())?
        }
        Err(_) => return Err(NO_SE_PUEDE.into()),
    };
    let sitio = url.host_str().unwrap_or("");
    if sitio.is_empty() || !(sitio.contains('.') || sitio == "localhost") {
        return Err(NO_SE_PUEDE.into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(NO_SE_PUEDE.into());
    }
    Ok(url.to_string())
}

/// ¿Lo que quedó como "esquema" es en realidad un sitio sin `https://`?
fn parece_sitio(esquema: &str) -> bool {
    esquema == "localhost" || esquema.contains('.')
}

fn consulta(texto: &str) -> Result<&str, String> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Err("¿Qué busco?".into());
    }
    if texto.chars().count() > LARGO_MAXIMO_BUSQUEDA {
        return Err("Es muy largo para una búsqueda.".into());
    }
    Ok(texto)
}

/// La dirección de una búsqueda de Google.
pub fn busqueda_google(texto: &str) -> Result<String, String> {
    let q = consulta(texto)?;
    Ok(Url::parse_with_params("https://www.google.com/search", &[("q", q)]).map_err(|e| e.to_string())?.to_string())
}

/// La lista de resultados de YouTube (cuando no se puede elegir el video).
pub fn busqueda_youtube(texto: &str) -> Result<String, String> {
    let q = consulta(texto)?;
    Ok(Url::parse_with_params("https://www.youtube.com/results", &[("search_query", q)])
        .map_err(|e| e.to_string())?
        .to_string())
}

/// La dirección de un video. El id viene de la API de YouTube: igual se
/// valida (11 letras, números, `-` o `_`), para no abrir cualquier cosa.
pub fn video_youtube(id: &str) -> Result<String, String> {
    let valido = id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valido {
        return Err("YouTube devolvió un video raro.".into());
    }
    Ok(format!("https://www.youtube.com/watch?v={id}"))
}

/// La consulta para la API de YouTube. "el primer video que sale cuando
/// buscás Riquelme" → "Riquelme": las palabras de relleno ensucian la búsqueda.
pub fn limpiar_busqueda_de_video(texto: &str) -> String {
    let mut q = texto.trim().trim_matches(|c: char| c == '"' || c == '«' || c == '»' || c == '\'').to_string();
    for relleno in ["un video de ", "el video de ", "video de ", "videos de ", "en youtube", "de youtube"] {
        if let Some(pos) = q.to_lowercase().find(relleno) {
            // `to_lowercase` no cambia el largo de lo que puede aparecer acá
            // (letras sin tilde), así que la posición sirve en el original.
            if q.is_char_boundary(pos) && q.is_char_boundary(pos + relleno.len()) {
                q.replace_range(pos..pos + relleno.len(), " ");
            }
        }
    }
    q.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------- Rutas ----------

/// Una ruta como texto comparable: sin el `\\?\` que agrega `canonicalize`
/// en Windows, con `\` y en minúsculas (en Windows "Documentos" y
/// "documentos" son la misma carpeta).
fn partes(ruta: &Path) -> Vec<String> {
    let texto = sin_prefijo_largo(ruta).to_string_lossy().replace('/', "\\").to_lowercase();
    texto.split('\\').filter(|p| !p.is_empty()).map(String::from).collect()
}

/// `canonicalize` en Windows devuelve `\\?\C:\Users\…` (el formato "largo").
/// Para mostrarla o pasársela a un programa va sin eso.
pub fn sin_prefijo_largo(ruta: &Path) -> PathBuf {
    let texto = ruta.to_string_lossy();
    if let Some(resto) = texto.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{resto}"))
    } else if let Some(resto) = texto.strip_prefix(r"\\?\") {
        PathBuf::from(resto)
    } else {
        ruta.to_path_buf()
    }
}

/// ¿`ruta` es `carpeta` o está adentro? Compara carpeta por carpeta (no por
/// texto: `Documentos2` no está dentro de `Documentos`).
pub fn esta_dentro(ruta: &Path, carpeta: &Path) -> bool {
    let (r, c) = (partes(ruta), partes(carpeta));
    !c.is_empty() && r.len() >= c.len() && r[..c.len()] == c[..]
}

/// ¿La ruta (ya resuelta con `canonicalize`) está dentro de alguna carpeta
/// permitida (también resueltas)? Resolver antes es lo que hace que `..` o
/// un enlace no la saquen de ahí.
pub fn dentro_de_permitidas(ruta: &Path, permitidas: &[PathBuf]) -> bool {
    permitidas.iter().any(|c| esta_dentro(ruta, c))
}

/// Las carpetas del sistema que nunca se pueden permitir.
#[derive(Debug, Clone, Default)]
pub struct CarpetasDelSistema {
    /// `C:\Windows`, `Program Files`, `ProgramData`, `AppData`…: ni ellas ni
    /// nada adentro.
    pub prohibidas: Vec<PathBuf>,
    /// La carpeta del usuario (`C:\Users\vos`): entera no, pero sí sus
    /// carpetas (Documentos está adentro).
    pub perfil: Option<PathBuf>,
}

/// Por qué no se puede permitir esta carpeta (`None` = se puede).
pub fn carpeta_prohibida(carpeta: &Path, sistema: &CarpetasDelSistema) -> Option<&'static str> {
    let p = partes(carpeta);
    if p.len() <= 1 {
        return Some("No puedo usar un disco entero: elegí una carpeta.");
    }
    for prohibida in &sistema.prohibidas {
        if esta_dentro(carpeta, prohibida) || esta_dentro(prohibida, carpeta) {
            return Some("Esa es una carpeta del sistema: no la puedo usar.");
        }
    }
    if let Some(perfil) = &sistema.perfil {
        if esta_dentro(perfil, carpeta) {
            return Some("Esa carpeta es muy grande (tiene todo tu usuario): elegí una de adentro.");
        }
    }
    None
}

// ---------- Tipos de archivo ----------

/// Texto que Nova puede leer y modificar.
const EDITABLES: &[&str] = &[
    "txt", "md", "markdown", "csv", "tsv", "json", "log", "ini", "cfg", "conf", "yaml", "yml", "toml", "xml", "srt",
    "html", "htm", "css", "js", "ts", "tsx", "jsx", "py", "rs", "java", "c", "cpp", "h", "cs", "go", "sql", "sh",
];

/// Lo que se puede abrir con su programa. Es una lista de los que sí (no de
/// los que no): un `.exe`, `.bat`, `.ps1`, `.js`, `.lnk` o `.html` bajado de
/// internet, abierto con doble clic, se ejecuta o corre con permisos de más.
const ABRIBLES: &[&str] = &[
    "txt", "md", "csv", "json", "log", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf",
    "epub", "png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "mp3", "wav", "flac", "ogg", "m4a", "aac", "mp4",
    "mkv", "avi", "mov", "webm", "zip", "rar", "7z",
];

fn extension(ruta: &Path) -> String {
    ruta.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn se_puede_editar(ruta: &Path) -> bool {
    EDITABLES.contains(&extension(ruta).as_str())
}

pub fn se_puede_abrir(ruta: &Path) -> bool {
    ABRIBLES.contains(&extension(ruta).as_str())
}

/// Abrirlo con un programa elegido ("abrí notas.txt en VS Code"): texto o
/// documentos, nunca algo que se ejecute.
pub fn se_puede_abrir_con_programa(ruta: &Path) -> bool {
    se_puede_editar(ruta) || se_puede_abrir(ruta)
}

/// Un nombre de archivo nuevo: sin carpetas, sin caracteres que Windows no
/// acepta y de un tipo editable.
pub fn validar_nombre_nuevo(nombre: &str) -> Result<String, String> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err("¿Cómo se llama el archivo?".into());
    }
    if nombre.chars().any(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control())
        || nombre.starts_with('.')
        || nombre.ends_with('.')
        || nombre.chars().count() > 120
    {
        return Err("Ese nombre de archivo no sirve.".into());
    }
    let reservado = nombre.split('.').next().unwrap_or("").to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "LPT1", "LPT2"].contains(&reservado.as_str()) {
        return Err("Ese nombre de archivo no sirve.".into());
    }
    let nombre = if Path::new(nombre).extension().is_none() { format!("{nombre}.txt") } else { nombre.to_string() };
    if !se_puede_editar(Path::new(&nombre)) {
        return Err("Solo creo archivos de texto (.txt, .md, .csv…).".into());
    }
    Ok(nombre)
}

// ---------- Cambios de texto ----------

/// Cómo termina cada línea del archivo: se respeta al modificarlo.
pub fn fin_de_linea(texto: &str) -> &'static str {
    if texto.contains("\r\n") { "\r\n" } else { "\n" }
}

fn con_fin_de_linea(texto: &str, fin: &str) -> String {
    let unix = texto.replace("\r\n", "\n");
    if fin == "\n" { unix } else { unix.replace('\n', fin) }
}

/// Las posiciones (en bytes) donde aparece `buscar`, sin importar mayúsculas.
fn buscar_sin_mayusculas(texto: &str, buscar: &str) -> Vec<(usize, usize)> {
    let objetivo: Vec<char> = buscar.chars().flat_map(char::to_lowercase).collect();
    let mut encontrados = Vec::new();
    let mut desde = 0;
    while desde < texto.len() {
        let mut fin = desde;
        let mut coincide = true;
        let mut letras = texto[desde..].char_indices().flat_map(|(i, c)| c.to_lowercase().map(move |l| (i, c, l)));
        let mut consumidas = 0;
        while consumidas < objetivo.len() {
            match letras.next() {
                Some((i, c, l)) if l == objetivo[consumidas] => {
                    fin = desde + i + c.len_utf8();
                    consumidas += 1;
                }
                _ => {
                    coincide = false;
                    break;
                }
            }
        }
        if coincide && consumidas == objetivo.len() && texto.is_char_boundary(fin) {
            encontrados.push((desde, fin));
            desde = fin;
        } else {
            desde += texto[desde..].chars().next().map_or(1, char::len_utf8);
        }
    }
    encontrados
}

/// Si lo que se reemplaza empezaba con mayúscula ("Martes"), lo nuevo
/// también ("Jueves"), aunque el pedido diga "jueves".
fn como_el_original(original: &str, nuevo: &str) -> String {
    let empieza_mayuscula = original.chars().next().is_some_and(char::is_uppercase);
    let mut letras = nuevo.chars();
    match letras.next() {
        Some(primera) if empieza_mayuscula && primera.is_lowercase() => primera.to_uppercase().chain(letras).collect(),
        _ => nuevo.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reemplazo {
    pub buscar: String,
    pub poner: String,
}

/// Aplica los reemplazos en orden. Todas las veces que aparece cada uno.
/// Primero busca exacto; si no está, sin importar mayúsculas. Devuelve el
/// texto nuevo y cuántos cambios hizo.
pub fn reemplazar(texto: &str, reemplazos: &[Reemplazo]) -> Result<(String, usize), String> {
    if reemplazos.is_empty() {
        return Err("¿Qué cambio?".into());
    }
    let fin = fin_de_linea(texto);
    let mut actual = texto.to_string();
    let mut total = 0;
    for r in reemplazos {
        let buscar = con_fin_de_linea(&r.buscar, fin);
        let poner = con_fin_de_linea(&r.poner, fin);
        if buscar.trim().is_empty() {
            return Err("¿Qué texto cambio?".into());
        }
        let exactos = actual.matches(buscar.as_str()).count();
        if exactos > 0 {
            actual = actual.replace(buscar.as_str(), &poner);
            total += exactos;
            continue;
        }
        let posiciones = buscar_sin_mayusculas(&actual, &buscar);
        if posiciones.is_empty() {
            return Err(format!("No encontré «{}» en el archivo.", r.buscar.trim()));
        }
        let mut nuevo = String::with_capacity(actual.len());
        let mut ultimo = 0;
        for &(inicio, final_) in &posiciones {
            nuevo.push_str(&actual[ultimo..inicio]);
            nuevo.push_str(&como_el_original(&actual[inicio..final_], &poner));
            ultimo = final_;
        }
        nuevo.push_str(&actual[ultimo..]);
        total += posiciones.len();
        actual = nuevo;
    }
    if actual == texto {
        return Err("Así como está, el archivo no cambia.".into());
    }
    Ok((actual, total))
}

/// Agrega una línea al final, con el mismo fin de línea que el archivo.
pub fn agregar_linea(texto: &str, linea: &str) -> Result<String, String> {
    let fin = fin_de_linea(texto);
    let linea = con_fin_de_linea(linea.trim_end_matches(['\r', '\n']), fin);
    if linea.trim().is_empty() {
        return Err("¿Qué línea agrego?".into());
    }
    let mut nuevo = texto.to_string();
    let terminaba_en_linea = texto.ends_with('\n');
    if !texto.is_empty() && !terminaba_en_linea {
        nuevo.push_str(fin);
    }
    nuevo.push_str(&linea);
    if terminaba_en_linea || texto.is_empty() {
        nuevo.push_str(fin);
    }
    Ok(nuevo)
}

// ---------- Antes y después ----------

/// Cómo se ve una línea en la confirmación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoLinea {
    /// De contexto, no cambia.
    Igual,
    Sale,
    Entra,
    /// Hay líneas sin cambios en el medio que no se muestran.
    Corte,
}

/// Una línea partida en tres: lo que queda igual antes, lo que cambia
/// (resaltado) y lo que queda igual después. "Reunión el martes" →
/// "Reunión el " + "martes" + "".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineaCambio {
    pub tipo: TipoLinea,
    pub inicio: String,
    pub marca: String,
    pub fin: String,
}

impl LineaCambio {
    fn igual(texto: &str) -> Self {
        LineaCambio { tipo: TipoLinea::Igual, inicio: recortar_final(texto, LARGO_CONTEXTO), marca: String::new(), fin: String::new() }
    }

    fn entera(tipo: TipoLinea, texto: &str) -> Self {
        LineaCambio { tipo, inicio: String::new(), marca: recortar_final(texto, LARGO_MARCA), fin: String::new() }
    }

    fn corte() -> Self {
        LineaCambio { tipo: TipoLinea::Corte, inicio: String::new(), marca: String::new(), fin: String::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AntesYDespues {
    pub lineas: Vec<LineaCambio>,
    /// Líneas cambiadas que no entraron en la vista.
    pub mas: usize,
}

/// Cuántas líneas como mucho (sin contar los cortes): es debajo de la
/// esfera, no un editor.
const LINEAS_VISIBLES: usize = 8;
const LARGO_CONTEXTO: usize = 90;
const LARGO_MARCA: usize = 120;
const LARGO_ALREDEDOR: usize = 40;
/// Arriba de esto (líneas distintas × líneas distintas) no se busca el
/// mejor emparejamiento: se muestra que sale todo y entra todo.
const LIMITE_COMPARACION: usize = 250_000;

fn recortar_final(texto: &str, largo: usize) -> String {
    if texto.chars().count() <= largo {
        texto.to_string()
    } else {
        format!("{}…", texto.chars().take(largo).collect::<String>())
    }
}

fn recortar_principio(texto: &str, largo: usize) -> String {
    let n = texto.chars().count();
    if n <= largo { texto.to_string() } else { format!("…{}", texto.chars().skip(n - largo).collect::<String>()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paso {
    Igual(usize),
    Sale(usize),
    Entra(usize),
}

/// Las diferencias línea por línea (la subsecuencia común más larga, después
/// de sacar lo igual del principio y del final).
fn diferencias(a: &[&str], b: &[&str]) -> Vec<Paso> {
    let mut inicio = 0;
    while inicio < a.len() && inicio < b.len() && a[inicio] == b[inicio] {
        inicio += 1;
    }
    let mut fin = 0;
    while fin < a.len() - inicio && fin < b.len() - inicio && a[a.len() - 1 - fin] == b[b.len() - 1 - fin] {
        fin += 1;
    }
    let (ma, mb) = (&a[inicio..a.len() - fin], &b[inicio..b.len() - fin]);
    let mut pasos: Vec<Paso> = (0..inicio).map(Paso::Igual).collect();
    let (n, m) = (ma.len(), mb.len());
    if n * m <= LIMITE_COMPARACION {
        // `tabla[i][j]`: cuántas líneas en común tienen ma[i..] y mb[j..].
        let ancho = m + 1;
        let mut tabla = vec![0u32; (n + 1) * ancho];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                tabla[i * ancho + j] = if ma[i] == mb[j] {
                    tabla[(i + 1) * ancho + j + 1] + 1
                } else {
                    tabla[(i + 1) * ancho + j].max(tabla[i * ancho + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && ma[i] == mb[j] {
                pasos.push(Paso::Igual(inicio + i));
                i += 1;
                j += 1;
            } else if i < n && (j == m || tabla[(i + 1) * ancho + j] >= tabla[i * ancho + j + 1]) {
                pasos.push(Paso::Sale(inicio + i));
                i += 1;
            } else {
                pasos.push(Paso::Entra(inicio + j));
                j += 1;
            }
        }
    } else {
        pasos.extend((0..n).map(|i| Paso::Sale(inicio + i)));
        pasos.extend((0..m).map(|j| Paso::Entra(inicio + j)));
    }
    pasos.extend((a.len() - fin..a.len()).map(Paso::Igual));
    pasos
}

/// Una línea que cambió, emparejada con la que la reemplaza: se resalta solo
/// la parte distinta.
fn par(viejo: &str, nuevo: &str) -> (LineaCambio, LineaCambio) {
    let v: Vec<char> = viejo.chars().collect();
    let n: Vec<char> = nuevo.chars().collect();
    let mut pre = 0;
    while pre < v.len() && pre < n.len() && v[pre] == n[pre] {
        pre += 1;
    }
    // Si corta en el medio de una palabra, se resalta la palabra entera
    // ("martes" → "jueves", no "mart" → "juev").
    while pre > 0 && v[pre - 1].is_alphanumeric() {
        pre -= 1;
    }
    let mut suf = 0;
    while suf < v.len() - pre && suf < n.len() - pre && v[v.len() - 1 - suf] == n[n.len() - 1 - suf] {
        suf += 1;
    }
    while suf > 0 && v[v.len() - suf].is_alphanumeric() {
        suf -= 1;
    }
    let armar = |tipo, letras: &[char]| {
        let texto = |r: std::ops::Range<usize>| letras[r].iter().collect::<String>();
        LineaCambio {
            tipo,
            inicio: recortar_principio(&texto(0..pre), LARGO_ALREDEDOR),
            marca: recortar_final(&texto(pre..letras.len() - suf), LARGO_MARCA),
            fin: recortar_final(&texto(letras.len() - suf..letras.len()), LARGO_ALREDEDOR),
        }
    };
    (armar(TipoLinea::Sale, &v), armar(TipoLinea::Entra, &n))
}

/// El antes y después de un archivo de texto: las líneas que cambian, con una
/// de contexto antes y después, y un corte donde se saltean líneas iguales.
pub fn antes_y_despues(viejo: &str, nuevo: &str) -> AntesYDespues {
    let a: Vec<&str> = viejo.lines().collect();
    let b: Vec<&str> = nuevo.lines().collect();
    let pasos = diferencias(&a, &b);
    let cambia = |k: usize| !matches!(pasos[k], Paso::Igual(_));
    // Se muestra cada cambio y la línea de antes y de después.
    let mostrar: Vec<bool> = (0..pasos.len())
        .map(|k| cambia(k) || (k > 0 && cambia(k - 1)) || (k + 1 < pasos.len() && cambia(k + 1)))
        .collect();

    let mut lineas: Vec<LineaCambio> = Vec::new();
    let mut visibles = 0;
    let mut mas = 0;
    let mut salteo = false;
    let mut k = 0;
    while k < pasos.len() {
        if !mostrar[k] {
            salteo = true;
            k += 1;
            continue;
        }
        if visibles >= LINEAS_VISIBLES {
            mas += pasos[k..].iter().filter(|p| !matches!(p, Paso::Igual(_))).count();
            break;
        }
        if salteo && !lineas.is_empty() {
            lineas.push(LineaCambio::corte());
        }
        salteo = false;
        match pasos[k] {
            Paso::Igual(i) => {
                lineas.push(LineaCambio::igual(a[i]));
                visibles += 1;
                k += 1;
            }
            _ => {
                // Un tramo seguido de cambios: lo que sale y lo que entra.
                let mut salen = Vec::new();
                let mut entran = Vec::new();
                while k < pasos.len() {
                    match pasos[k] {
                        Paso::Sale(i) => salen.push(a[i]),
                        Paso::Entra(j) => entran.push(b[j]),
                        Paso::Igual(_) => break,
                    }
                    k += 1;
                }
                let pares = salen.len().min(entran.len());
                let mut tramo = Vec::with_capacity(salen.len() + entran.len());
                for p in 0..pares {
                    let (s, e) = par(salen[p], entran[p]);
                    tramo.push(s);
                    tramo.push(e);
                }
                tramo.extend(salen[pares..].iter().map(|l| LineaCambio::entera(TipoLinea::Sale, l)));
                tramo.extend(entran[pares..].iter().map(|l| LineaCambio::entera(TipoLinea::Entra, l)));
                for linea in tramo {
                    if visibles < LINEAS_VISIBLES {
                        lineas.push(linea);
                        visibles += 1;
                    } else {
                        mas += 1;
                    }
                }
            }
        }
    }
    AntesYDespues { lineas, mas }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_abre_paginas_web() {
        assert_eq!(validar_direccion("https://www.frba.utn.edu.ar").unwrap(), "https://www.frba.utn.edu.ar/");
        assert_eq!(validar_direccion("utn.edu.ar/campus").unwrap(), "https://utn.edu.ar/campus");
        assert_eq!(validar_direccion("localhost:1420").unwrap(), "https://localhost:1420/");
        assert_eq!(validar_direccion("www.sitio.com:8080/x").unwrap(), "https://www.sitio.com:8080/x");
        for mala in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "C:\\Windows\\notepad.exe",
            "ms-settings:privacy",
            "ftp://sitio.com",
            "https://usuario:clave@sitio.com",
            "https://sitio.com/una cosa",
            "",
            "hola",
            "vbscript:msgbox",
        ] {
            assert!(validar_direccion(mala).is_err(), "no debería abrir {mala}");
        }
    }

    #[test]
    fn arma_las_busquedas() {
        assert_eq!(busqueda_google("recetas de pizza").unwrap(), "https://www.google.com/search?q=recetas+de+pizza");
        assert_eq!(busqueda_google("a&b=c #1").unwrap(), "https://www.google.com/search?q=a%26b%3Dc+%231");
        assert_eq!(busqueda_youtube("lo-fi").unwrap(), "https://www.youtube.com/results?search_query=lo-fi");
        assert!(busqueda_google("   ").is_err());
        assert_eq!(video_youtube("dQw4w9WgXcQ").unwrap(), "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert!(video_youtube("../../etc").is_err());
        assert!(video_youtube("abc").is_err());
    }

    #[test]
    fn limpia_la_busqueda_del_video() {
        assert_eq!(limpiar_busqueda_de_video("un video de lo-fi"), "lo-fi");
        assert_eq!(limpiar_busqueda_de_video("«Riquelme»"), "Riquelme");
        assert_eq!(limpiar_busqueda_de_video("goles de Riquelme en YouTube"), "goles de Riquelme");
    }

    fn p(texto: &str) -> PathBuf {
        PathBuf::from(texto)
    }

    #[test]
    fn una_ruta_esta_dentro_carpeta_por_carpeta() {
        let docs = p(r"C:\Users\Augusto\Documents");
        assert!(esta_dentro(&p(r"C:\Users\Augusto\Documents\notas.txt"), &docs));
        assert!(esta_dentro(&p(r"\\?\C:\Users\augusto\documents\facu\tp.md"), &docs));
        assert!(esta_dentro(&docs, &docs));
        assert!(!esta_dentro(&p(r"C:\Users\Augusto\Documents2\x.txt"), &docs));
        assert!(!esta_dentro(&p(r"C:\Users\Augusto\x.txt"), &docs));
        assert!(!esta_dentro(&p(r"D:\Users\Augusto\Documents\x.txt"), &docs));
        let permitidas = [docs.clone(), p(r"C:\Users\Augusto\Downloads")];
        assert!(dentro_de_permitidas(&p(r"C:\Users\Augusto\Downloads\a.pdf"), &permitidas));
        assert!(!dentro_de_permitidas(&p(r"C:\Windows\win.ini"), &permitidas));
        assert!(!dentro_de_permitidas(&p(r"C:\Users\Augusto\Downloads\..\x.txt"), &[]));
        assert_eq!(sin_prefijo_largo(&p(r"\\?\C:\a\b")), p(r"C:\a\b"));
        assert_eq!(sin_prefijo_largo(&p(r"\\?\UNC\servidor\c")), p(r"\\servidor\c"));
    }

    #[test]
    fn no_deja_permitir_carpetas_del_sistema() {
        let sistema = CarpetasDelSistema {
            prohibidas: vec![
                p(r"C:\Windows"),
                p(r"C:\Program Files"),
                p(r"C:\Program Files (x86)"),
                p(r"C:\ProgramData"),
                p(r"C:\Users\Augusto\AppData"),
            ],
            perfil: Some(p(r"C:\Users\Augusto")),
        };
        assert!(carpeta_prohibida(&p(r"C:\Users\Augusto\Documents"), &sistema).is_none());
        assert!(carpeta_prohibida(&p(r"D:\Facu"), &sistema).is_none());
        for mala in [
            r"C:\",
            r"D:\",
            r"C:\Windows",
            r"C:\Windows\System32",
            r"C:\Program Files\Algo",
            r"C:\ProgramData",
            r"C:\Users\Augusto\AppData\Local\Nova",
            r"C:\Users\Augusto",
            r"C:\Users",
        ] {
            assert!(carpeta_prohibida(&p(mala), &sistema).is_some(), "no debería permitir {mala}");
        }
    }

    #[test]
    fn distingue_los_tipos_de_archivo() {
        assert!(se_puede_editar(&p("notas.TXT")) && se_puede_editar(&p("lista.md")));
        assert!(!se_puede_editar(&p("tp.docx")) && !se_puede_editar(&p("foto.png")));
        assert!(se_puede_abrir(&p("tp.pdf")) && se_puede_abrir(&p("tema.mp3")));
        for peligroso in ["virus.exe", "a.bat", "a.cmd", "a.ps1", "a.vbs", "a.js", "a.lnk", "a.url", "a.msi", "a.html", "a.reg", "sin_extension"] {
            assert!(!se_puede_abrir(&p(peligroso)), "no debería abrir {peligroso}");
        }
        assert!(se_puede_abrir_con_programa(&p("script.js")));
        assert!(!se_puede_abrir_con_programa(&p("virus.exe")));
    }

    #[test]
    fn valida_los_nombres_nuevos() {
        assert_eq!(validar_nombre_nuevo("compras").unwrap(), "compras.txt");
        assert_eq!(validar_nombre_nuevo(" ideas.md ").unwrap(), "ideas.md");
        for malo in ["", "..\\x.txt", "a/b.txt", "C:x.txt", "con.txt", "a.exe", ".oculto", "a?.txt"] {
            assert!(validar_nombre_nuevo(malo).is_err(), "no debería aceptar {malo}");
        }
    }

    fn r(buscar: &str, poner: &str) -> Reemplazo {
        Reemplazo { buscar: buscar.into(), poner: poner.into() }
    }

    #[test]
    fn reemplaza_respetando_mayusculas_y_fines_de_linea() {
        let (nuevo, n) = reemplazar("Reunión el martes\r\nMartes: llevar apuntes\r\n", &[r("martes", "jueves")]).unwrap();
        // Exacto primero: solo el que está en minúscula.
        assert_eq!((nuevo.as_str(), n), ("Reunión el jueves\r\nMartes: llevar apuntes\r\n", 1));
        let (nuevo, n) = reemplazar("Martes: llevar apuntes\n", &[r("martes", "jueves")]).unwrap();
        assert_eq!((nuevo.as_str(), n), ("Jueves: llevar apuntes\n", 1));
        let (nuevo, _) = reemplazar("a\r\nb\r\n", &[r("a\nb", "c\nd")]).unwrap();
        assert_eq!(nuevo, "c\r\nd\r\n");
        let (nuevo, _) = reemplazar("ÁRBOL y árbol", &[r("Árbol", "pino")]).unwrap();
        assert_eq!(nuevo, "Pino y pino");
        assert!(reemplazar("hola", &[r("chau", "x")]).unwrap_err().contains("No encontré"));
        assert!(reemplazar("hola", &[r("hola", "hola")]).is_err());
        assert!(reemplazar("hola", &[]).is_err());
    }

    #[test]
    fn agrega_lineas_al_final() {
        assert_eq!(agregar_linea("a\nb\n", "c").unwrap(), "a\nb\nc\n");
        assert_eq!(agregar_linea("a\r\nb", "c").unwrap(), "a\r\nb\r\nc");
        assert_eq!(agregar_linea("", "primera").unwrap(), "primera\n");
        assert!(agregar_linea("a", "  ").is_err());
    }

    fn tipos(v: &AntesYDespues) -> Vec<TipoLinea> {
        v.lineas.iter().map(|l| l.tipo).collect()
    }

    #[test]
    fn resalta_solo_la_palabra_que_cambia() {
        let v = antes_y_despues("Compras\nReunión el martes a las 10\nFin\n", "Compras\nReunión el jueves a las 10\nFin\n");
        use TipoLinea::*;
        assert_eq!(tipos(&v), [Igual, Sale, Entra, Igual]);
        assert_eq!((v.lineas[1].inicio.as_str(), v.lineas[1].marca.as_str(), v.lineas[1].fin.as_str()), ("Reunión el ", "martes", " a las 10"));
        assert_eq!(v.lineas[2].marca, "jueves");
        assert_eq!(v.mas, 0);
    }

    #[test]
    fn corta_las_partes_que_no_cambian() {
        let viejo: String = (1..=20).map(|i| format!("línea {i}\n")).collect();
        let nuevo = viejo.replace("línea 2\n", "línea dos\n").replace("línea 18\n", "línea dieciocho\n");
        let v = antes_y_despues(&viejo, &nuevo);
        use TipoLinea::*;
        // El corte no cuenta entre las ocho líneas visibles.
        assert_eq!(tipos(&v), [Igual, Sale, Entra, Igual, Corte, Igual, Sale, Entra, Igual]);
        assert_eq!(v.lineas[0].inicio, "línea 1");
        assert_eq!((v.lineas[1].marca.as_str(), v.lineas[2].marca.as_str()), ("2", "dos"));
        assert_eq!(v.mas, 0);
    }

    #[test]
    fn un_archivo_nuevo_o_borrado_se_ve_entero_hasta_un_limite() {
        let v = antes_y_despues("", "uno\ndos\n");
        assert_eq!(tipos(&v), [TipoLinea::Entra, TipoLinea::Entra]);
        assert_eq!(v.lineas[0].marca, "uno");
        let largo: String = (1..=30).map(|i| format!("{i}\n")).collect();
        let v = antes_y_despues(&largo, "");
        assert_eq!(v.lineas.len(), 8);
        assert_eq!(v.mas, 22);
    }

    #[test]
    fn agrega_una_linea_con_contexto() {
        let v = antes_y_despues("a\nb\n", "a\nb\nc\n");
        assert_eq!(tipos(&v), [TipoLinea::Igual, TipoLinea::Entra]);
        assert_eq!(v.lineas[0].inicio, "b");
    }

    #[test]
    fn recorta_lineas_largas() {
        let viejo = format!("{} martes {}", "x".repeat(100), "y".repeat(100));
        let nuevo = viejo.replace("martes", "jueves");
        let v = antes_y_despues(&viejo, &nuevo);
        assert!(v.lineas[0].inicio.starts_with('…') && v.lineas[0].inicio.chars().count() <= 41);
        assert!(v.lineas[0].fin.ends_with('…'));
        assert_eq!(v.lineas[0].marca, "martes");
    }
}
