//! Los programas instalados (v0.7.1): los accesos directos del menú Inicio
//! más unos pocos de Windows que no tienen (la calculadora, el bloc de
//! notas…). Nova solo abre programas de esta lista: el modelo dice un nombre,
//! nunca una ruta.

use std::path::{Path, PathBuf};

use crate::dominio::lenguaje::normalizar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programa {
    pub nombre: String,
    /// El acceso directo (`.lnk`) o, para los de Windows, el nombre fijo.
    pub destino: PathBuf,
}

/// Los de Windows que no tienen acceso directo en el menú Inicio. El
/// destino es fijo (no viene del modelo).
const DE_WINDOWS: &[(&str, &str)] = &[
    ("Calculadora", "calc.exe"),
    ("Bloc de notas", "notepad.exe"),
    ("Explorador de archivos", "explorer.exe"),
    ("Paint", "mspaint.exe"),
    ("Recortes", "SnippingTool.exe"),
    ("Configuración", "ms-settings:"),
];

/// Otras formas de llamarlos: (cómo lo dicen, cómo se llama).
const APODOS: &[(&str, &str)] = &[
    ("vs code", "visual studio code"),
    ("vscode", "visual studio code"),
    ("code", "visual studio code"),
    ("visual code", "visual studio code"),
    ("calcu", "calculadora"),
    ("calculator", "calculadora"),
    ("notepad", "bloc de notas"),
    ("explorador", "explorador de archivos"),
    ("explorer", "explorador de archivos"),
    ("ajustes", "configuracion"),
    ("chrome", "google chrome"),
    ("edge", "microsoft edge"),
    ("word", "word"),
];

/// Las carpetas del menú Inicio: la tuya y la de todos los usuarios.
pub fn carpetas_del_menu_inicio() -> Vec<PathBuf> {
    ["APPDATA", "PROGRAMDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|base| PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"))
        .collect()
}

/// ¿Es un acceso directo que no es un programa ("Desinstalar X", la ayuda)?
fn es_de_relleno(nombre: &str) -> bool {
    let n = normalizar(nombre);
    ["desinstal", "uninstall", "readme", "leame", "release notes", "notas de la version", "website", "sitio web", "help", "ayuda de"]
        .iter()
        .any(|r| n.contains(r))
}

fn recorrer(carpeta: &Path, profundidad: u8, salida: &mut Vec<Programa>) {
    let Ok(entradas) = std::fs::read_dir(carpeta) else { return };
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        let Ok(tipo) = entrada.file_type() else { continue };
        if tipo.is_dir() {
            if profundidad < 4 {
                recorrer(&ruta, profundidad + 1, salida);
            }
        } else if ruta.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) {
            let nombre = ruta.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if !nombre.is_empty() && !es_de_relleno(&nombre) {
                salida.push(Programa { nombre, destino: ruta });
            }
        }
    }
}

/// Arma la lista recorriendo las carpetas. Si un programa aparece en las dos
/// (tuya y de todos), queda uno.
pub fn indexar(carpetas: &[PathBuf]) -> Vec<Programa> {
    let mut lista = Vec::new();
    for c in carpetas {
        recorrer(c, 0, &mut lista);
    }
    lista.extend(DE_WINDOWS.iter().map(|(n, d)| Programa { nombre: (*n).into(), destino: PathBuf::from(d) }));
    let mut vistos = std::collections::HashSet::new();
    lista.retain(|p| vistos.insert(normalizar(&p.nombre)));
    lista
}

/// Qué tan bien coincide lo pedido con un nombre (0 = nada).
fn puntaje(pedido: &str, nombre: &str) -> u8 {
    if pedido == nombre {
        return 100;
    }
    let palabras: Vec<&str> = nombre.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()).collect();
    if nombre.starts_with(pedido) && nombre[pedido.len()..].starts_with(|c: char| !c.is_alphanumeric()) {
        return 80;
    }
    if nombre.starts_with(pedido) {
        return 70;
    }
    let buscadas: Vec<&str> = pedido.split(' ').collect();
    if buscadas.iter().all(|b| palabras.iter().any(|p| p.starts_with(b))) {
        return 60;
    }
    // Whisper a veces escribe "spotifai" o "bisual studio": parecido alcanza
    // si es largo y le erra por poco.
    if pedido.chars().count() >= 5 && distancia(pedido, nombre) <= 2 {
        return 40;
    }
    0
}

/// Cuántas letras hay que cambiar, agregar o sacar para pasar de una
/// palabra a la otra (distancia de Levenshtein).
fn distancia(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut fila: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut anterior = fila[0];
        fila[0] = i + 1;
        for j in 0..b.len() {
            let arriba = fila[j + 1];
            fila[j + 1] = (anterior + usize::from(ca != b[j])).min(fila[j] + 1).min(arriba + 1);
            anterior = arriba;
        }
    }
    fila[b.len()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Busqueda<'a> {
    Uno(&'a Programa),
    /// Hay varios igual de parecidos: hay que preguntar cuál.
    Varios(Vec<&'a Programa>),
    Ninguno,
}

pub fn buscar<'a>(lista: &'a [Programa], pedido: &str) -> Busqueda<'a> {
    let mut pedido = normalizar(pedido);
    for prefijo in ["el ", "la ", "programa ", "app ", "aplicacion "] {
        if let Some(resto) = pedido.strip_prefix(prefijo) {
            pedido = resto.to_string();
        }
    }
    if pedido.is_empty() {
        return Busqueda::Ninguno;
    }
    if let Some((_, nombre)) = APODOS.iter().find(|(apodo, _)| *apodo == pedido) {
        pedido = (*nombre).to_string();
    }
    let puntajes: Vec<(u8, &Programa)> =
        lista.iter().map(|p| (puntaje(&pedido, &normalizar(&p.nombre)), p)).filter(|(s, _)| *s > 0).collect();
    let Some(mejor) = puntajes.iter().map(|(s, _)| *s).max() else { return Busqueda::Ninguno };
    let mut empatados: Vec<&Programa> = puntajes.iter().filter(|(s, _)| *s == mejor).map(|(_, p)| *p).collect();
    if empatados.len() == 1 {
        return Busqueda::Uno(empatados[0]);
    }
    empatados.sort_by_key(|p| p.nombre.len());
    empatados.truncate(5);
    Busqueda::Varios(empatados)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lista(nombres: &[&str]) -> Vec<Programa> {
        nombres.iter().map(|n| Programa { nombre: (*n).into(), destino: PathBuf::from(format!("{n}.lnk")) }).collect()
    }

    fn uno(l: &[Programa], pedido: &str) -> String {
        match buscar(l, pedido) {
            Busqueda::Uno(p) => p.nombre.clone(),
            otra => panic!("«{pedido}» dio {otra:?}"),
        }
    }

    #[test]
    fn encuentra_por_nombre_apodo_o_parecido() {
        let l = lista(&["Visual Studio Code", "Spotify", "Google Chrome", "Calculadora", "Word", "Discord"]);
        assert_eq!(uno(&l, "Visual Studio Code"), "Visual Studio Code");
        assert_eq!(uno(&l, "vs code"), "Visual Studio Code");
        assert_eq!(uno(&l, "VSCode"), "Visual Studio Code");
        assert_eq!(uno(&l, "visual"), "Visual Studio Code");
        assert_eq!(uno(&l, "spotify"), "Spotify");
        assert_eq!(uno(&l, "Spotifai"), "Spotify");
        assert_eq!(uno(&l, "chrome"), "Google Chrome");
        assert_eq!(uno(&l, "la calculadora"), "Calculadora");
        assert_eq!(buscar(&l, "photoshop"), Busqueda::Ninguno);
        assert_eq!(buscar(&l, ""), Busqueda::Ninguno);
    }

    #[test]
    fn si_hay_dos_parecidos_pregunta() {
        let l = lista(&["Visual Studio Code", "Visual Studio Code - Insiders", "Visual Studio 2022"]);
        // El nombre exacto gana.
        assert_eq!(uno(&l, "visual studio code"), "Visual Studio Code");
        match buscar(&l, "visual studio") {
            Busqueda::Varios(v) => assert_eq!(v.len(), 3, "{v:?}"),
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn indexa_los_accesos_directos_y_saltea_los_de_relleno() {
        let raiz = std::env::temp_dir().join(format!("nova-programas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(raiz.join("Visual Studio Code")).unwrap();
        for f in ["Visual Studio Code/Visual Studio Code.lnk", "Spotify.lnk", "Desinstalar Spotify.lnk", "Ayuda.txt", "Spotify.url"] {
            std::fs::write(raiz.join(f), b"").unwrap();
        }
        let l = indexar(&[raiz.clone(), raiz.clone()]);
        let nombres: Vec<&str> = l.iter().map(|p| p.nombre.as_str()).collect();
        assert!(nombres.contains(&"Visual Studio Code") && nombres.contains(&"Spotify"));
        assert!(!nombres.iter().any(|n| n.contains("Desinstalar") || n.contains("Ayuda")));
        assert_eq!(nombres.iter().filter(|n| **n == "Spotify").count(), 1);
        assert!(nombres.contains(&"Calculadora"));
        std::fs::remove_dir_all(&raiz).unwrap();
    }

    /// En esta compu: tienen que estar los programas del criterio de terminado.
    #[test]
    #[ignore]
    fn encuentra_los_de_esta_compu() {
        let l = indexar(&carpetas_del_menu_inicio());
        for pedido in ["Visual Studio Code", "Spotify", "calculadora"] {
            assert!(matches!(buscar(&l, pedido), Busqueda::Uno(_)), "{pedido}: {:?}", buscar(&l, pedido));
        }
    }
}
