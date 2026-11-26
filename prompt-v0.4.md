Vamos a hacer la **v0.4 de Nova (Partidos, noticias y resumen del día)**. Repo: https://github.com/AugusTraver/Nova (rama `main`, al día con v0.3 fusionada). La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

## Antes de nada
1. Leé `CLAUDE.md`, `SPEC.md` (sobre todo la sección 4.4 "Fútbol", 4.5 "Noticias", 4.6 "Resumen del día", 4.7 "Clima" y 5.5 "Decisiones de v0.3") y `PLAN.md` (la sección v0.4). CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
2. Usá `/find-skills` para buscar skills que sirvan para esta versión: la API de fútbol que elijamos (API-Football u otra), parseo de RSS/Atom en Rust, caché y tareas periódicas en Rust/Tokio, filtrado o ranking con LLMs, Open-Meteo. Ya están instaladas tauri-v2, rust-best-practices, rust-testing, vitest, better-accessibility, animate, emil-design-eng y frontend-design. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y preguntame antes de instalar.
3. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas mientras corre sin avisarme.** Para crear la rama sin cambiarte podés usar `git branch v0.4 main`. Si `main` y la rama actual tienen los mismos archivos (`git diff --stat`), cambiar de rama no toca el disco: avisame y hacelo.

## Qué pide v0.4 (PLAN.md)
- **Fútbol:** integración con una API de fútbol y caché. Boca (próximo partido, último resultado, tabla, modo en vivo), Selección y jugadores seguidos.
- **Noticias:** feeds RSS y filtrado y ordenamiento con IA según mis intereses (5 a 8 titulares con un resumen de una línea; clic abre la nota en el navegador).
- **Resumen del día:** tarjeta y herramienta `resumen_del_dia`.
- Herramientas del asistente `consultar_partidos`, `consultar_tabla`, `consultar_jugador` y `consultar_noticias`.
- Manejo de errores y datos cacheados sin internet: cada widget muestra su último dato y un aviso, sin romper el panel.
- (Opcional) Widget de clima chico con Open-Meteo.
- **Terminado cuando:** abro el panel a la mañana y de un vistazo veo mi día, cuándo juega Boca y las noticias que me importan.

## Lo que ya existe (v0.1 a v0.3) y hay que reusar
- **Rust** (`src-tauri/src/`):
  - `dominio/`: lógica pura con tests (fechas como `Momento`, recurrencias, reglas automáticas, `fechas_relativas.rs` para "el jueves"/"a las 9", `lenguaje.rs` para normalizar texto sin tildes).
  - `repos/`: consultas, testeadas con base en memoria (`db::en_memoria()`). Todas reciben `Ahora` (reloj inyectado; `Ahora::fijo_a_las("2026-11-17", "14:00")` en tests). `preferencias.rs` guarda pares clave → JSON (ahí van equipos, jugadores seguidos, intereses, feeds).
  - `db/`: migraciones versionadas en `src-tauri/migraciones/` (`PRAGMA user_version`). Para tablas nuevas (caché, por ejemplo) se AGREGA un archivo `003_...sql`, nunca se edita uno viejo.
  - `asistente/`: el ciclo del asistente en Rust.
    - `herramientas.rs`: definiciones (JSON Schema) y `ejecutar`. Las herramientas nuevas de v0.4 se suman acá y en su test `las_definiciones_tienen_...`. Los errores vuelven al modelo como texto, no cortan el turno.
    - `prompt.rs`: el prompt de sistema con contexto. Ojo con el largo, porque el plan gratis de Groq tiene 8K tokens por minuto.
    - `proveedor.rs` / `openai.rs`: el trait `ProveedorIa` y el cliente compatible con OpenAI (Groq u Ollama). **Reusalo para filtrar y ordenar noticias con IA**, no armes otro cliente.
    - `ciclo.rs`: un turno completo; avisa cambios con el trait `Avisos`.
  - `llavero.rs`: secretos en el llavero de Windows (`keyring-core`). **La key de la API de fútbol va acá**, igual que la de Groq, configurable desde Ajustes y nunca en el webview ni en el repo.
  - `comandos.rs`: comandos de Tauri tipados. La base es `State<Db>` con un `Mutex<Connection>`; no retener el candado durante un `await`. Rust emite el evento `datos-cambiados` con la lista de fuentes que cambiaron (`asistente::herramientas::Fuente`: agenda, responsabilidades, materias). Si sumás fuentes nuevas (partidos, noticias), extendé ese enum y el tipo `FuenteDatos` de TS.
  - Ya hay `reqwest` (con TLS nativo de Windows, `native-tls`) y `serde_json`. Cualquier dependencia nueva (parser de RSS, etc.) se consulta antes.
- **Frontend:**
  - `src/lib/api.ts` espeja los comandos. En el navegador sin Tauri usa `src/mocks/apiMemoria.ts`. Hay que sumar ahí versiones de prueba de lo nuevo.
  - `src/store/datos.ts` (Zustand) carga y recarga datos por fuente; `escucharCambios` en `src/app/puenteTauri.ts` recarga los widgets cuando Rust avisa.
  - **Los widgets `src/widgets/Futbol.tsx`, `Noticias.tsx` y `ResumenDelDia.tsx` hoy leen datos de ejemplo de `src/mocks/datos.ts`**: hay que pasarlos a datos reales. Los tipos `Partido`, `ResultadoPartido` y `Noticia` están en `src/lib/tipos.ts` (sección "Datos externos").
  - `src/app/Ajustes.tsx` + `AjustesAsistente.tsx`: Ajustes ya tiene atajo, reglas automáticas y asistente (proveedor, modelo, API key con "Probar conexión"). Lo nuevo (key de fútbol, equipos/jugadores, intereses y feeds) sigue ese mismo patrón.
  - La respuesta del asistente va debajo de la esfera (`src/app/Respuesta.tsx`); la esfera tiene estados en `src/store/panel.ts` (`estadoEsfera`) y uniforms en `src/esfera/` (ya hay `uPensando` y `uError`; un efecto de gol de Boca encajaría ahí).
- **Test de contrato:** `comandos.rs` tiene un test que verifica que los JSON de Rust y los tipos de TS (`src/lib/tipos.ts`) tengan los mismos nombres de campos. Si agregás comandos o tipos, sumalos a ese test.

## Estética (obligatoria)
Dirección "Atmósfera" (está descripta en CLAUDE.md): cielo translúcido según la hora, esfera de luz en el centro y widgets sin tarjetas que la orbitan. Usá los tokens de `src/theme/tokens.css` (incluido `--color-aviso` para errores y avisos), las tipografías Fraunces + Instrument Sans, textos en español rioplatense y nada de etiquetas en mayúsculas ni monoespaciadas. Los formularios se abren dentro del lugar donde se usan, sin modales. El modo en vivo de Boca agranda el widget (SPEC 4.4) sin romper la órbita. Para ver la interfaz sin compilar Rust: `http://localhost:1420/?cielo=noche` (o `manana`, `mediodia`, `tarde`, `anochecer`, `madrugada`). Probala emulando una pantalla de 1600×900: el panel del navegador interno es chico y no representa la pantalla completa.

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **API de fútbol:** cuál usar. Verificá HOY, sin confiar en lo que sabías antes: límites diarios del plan gratis, **si el plan gratis da acceso a la temporada actual** (algunos planes gratis solo dan temporadas viejas), si cubre Liga Profesional Argentina, Copa Argentina, Libertadores, Sudamericana, Selección y ligas europeas, y si tiene datos en vivo. Compará al menos API-Football con dos alternativas.
2. **Dónde corre la descarga y la caché:** en Rust o en el frontend. Tené en cuenta las keys, que en v0.6 habrá notificaciones con el panel cerrado ("arranca el partido") y el consumo en reposo.
3. **Caché:** dónde se guarda (tabla nueva en SQLite con migración, o archivos), cuándo se refresca (fixture/tabla una vez por día, jugadores después de la fecha, en vivo solo cuando juega Boca o la Selección) y cómo se reparte el límite diario de consultas. Mostrá la cuenta.
4. **Modo en vivo:** cómo se detecta que empezó el partido, cada cuánto se consulta para no pasarse del límite, qué muestra el widget y el efecto de la esfera en un gol.
5. **Noticias:** feeds por defecto (fútbol argentino, Boca, Selección, tecnología/IA, desarrollo web), crate para parsear RSS/Atom, y **cómo filtra la IA** sin pasarse del límite de tokens (cuándo corre, cuántos titulares le manda, qué pasa si falla o si no hay key: orden de respaldo sin IA).
6. **Jugadores seguidos:** cuáles por defecto, cómo se configuran y cómo se buscan en la API.
7. **Resumen del día:** armado con una plantilla local (determinística, testeable) o generado por la IA; y qué devuelve la herramienta `resumen_del_dia`.
8. **Clima:** si entra en esta versión o no.

Después de decidir: creá la rama `v0.4`, listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- La lógica de negocio va separada y con tests (por ejemplo: elegir el próximo partido y el último resultado, calcular cuándo refrescar la caché, el orden de respaldo de noticias, el armado del resumen). Lo que toca el sistema (red con keys, llavero, SQLite) va en Rust, con comandos tipados.
- Para la red, separá "armar el pedido" y "leer la respuesta" en funciones puras con tests (como `asistente/openai.rs`), así se testea sin internet.
- Explicame brevemente el Rust que no sea obvio (soy fuerte en web, no en Rust).
- Commits chicos y en español (`feat(futbol): ...`, `feat(noticias): ...`), con la línea de coautoría que corresponda.
- Si una decisión no está en la especificación y cambia algo importante (arquitectura, dependencias pesadas, datos), preguntá antes.
- En PowerShell usá `npm.cmd` (`npm.ps1` está bloqueado). **No reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- Para correr los tests de Rust con `tauri dev` abierto, usá otra carpeta de build (`CARGO_TARGET_DIR` en una carpeta temporal), así no se pisan. `tauri dev` recompila solo cada vez que cambia el Rust.
- Nunca pongas API keys en el código, en el repo ni en archivos de texto. Las keys las carga el usuario en Ajustes; vos no las escribas en ningún campo.
- Al terminar: verificá el criterio de terminado, actualizá el estado en CLAUDE.md y avisame antes de fusionar a `main`. La fusión se puede hacer sin checkout (como v0.3): `git commit-tree "v0.4^{tree}" -p main -p v0.4` y `git update-ref refs/heads/main <commit> <main anterior>`.
