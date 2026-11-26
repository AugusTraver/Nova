Vamos a hacer la **v0.7.1 de Nova (Nova usa la compu)**. Repo: https://github.com/AugusTraver/Nova. La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

La v0.7 se partió en tres: **v0.7.1** (esta: abrir programas, páginas, videos y archivos), **v0.7.2** (Google Calendar) y **v0.7.3** (buscar información en internet). Están en PLAN.md, cada una con su prompt.

## Antes de nada
1. **Creá la rama.** Antes vino la v0.6.1 (gastar menos del plan gratis de Groq, en la rama `v0.6.1`). Si `v0.6.1` tiene commits que no están en `main` (por ejemplo, los últimos cambios de PLAN.md), avisame y fusionalos sin checkout (ver abajo). Después creá `v0.7.1` desde `main` con `git branch v0.7.1 main`. Si las dos ramas tienen los mismos archivos (`git diff --stat`), cambiar de rama no toca el disco: avisame y hacelo.
2. **PLAN.md ya tiene esta v0.7.1.** Si algo de lo que sigue no coincide, proponeme cómo queda.
3. Leé estos tres archivos. CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
   - `CLAUDE.md`;
   - `SPEC.md`, sobre todo 5.1 (ciclo del asistente), 5.5 "Decisiones de v0.3" (asistente y herramientas), 5.6 (voz), 5.7 "Decisiones de v0.6", 5.8 "Decisiones de v0.6.1" (tokens) y 8 "Requisitos no funcionales";
   - `PLAN.md`, las secciones v0.7.1 a v0.7.3, para no adelantar lo que va después.
4. **Si querés, usá `/find-skills`** para buscar skills de:
   - automatización de Windows desde Rust (abrir apps, ventanas, teclas de medios);
   - seguridad de agentes con acceso a archivos (permisos, inyección de instrucciones);

   Ya están instaladas tauri-v2, rust-best-practices, rust-async-patterns, rust-testing, vitest, better-accessibility, animate, emil-design-eng, frontend-design y security-and-hardening. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y **preguntame antes de instalar**.
5. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas mientras corre sin avisarme.**

## Qué tiene que lograr v0.7.1

### La diferencia con v0.7.3 (importante)
En esta versión Nova **abre** cosas: una pestaña, un video, un programa, un archivo. No lee páginas para contestarme con lo que dicen.
- "Buscame en Google recetas de pizza": abre una pestaña con esa búsqueda de Google. **Es v0.7.1.**
- "Poneme el primer video que sale en YouTube cuando buscás Riquelme": encuentra ese video y lo abre. **Es v0.7.1.**
- "Fijate quién ganó anoche River" y que me conteste en una frase: busca información y la resume. **Es v0.7.3**, no la hagas acá.

### 1. Nova usa la compu
Que yo le pida cosas por voz o por texto y las haga:
- **Navegador:**
  - "abrí la página de la facultad" abre una dirección;
  - "buscame en Google X" abre una pestaña con la búsqueda de Google;
  - siempre en el navegador que tengo por defecto en Windows.
- **YouTube:**
  - "poneme el primer video que sale cuando buscás Riquelme" abre ese video, no la lista de resultados;
  - "abrime un video de lo-fi" busca y abre uno.
- **Programas:**
  - "abrí Visual Studio Code", "abrí Spotify", "abrí la calculadora";
  - tiene que encontrar los programas instalados (por ejemplo, los accesos directos del menú Inicio) sin que yo cargue la ruta de cada uno.
- **Archivos:**
  - "abrí la carpeta de descargas";
  - "qué dice el archivo notas.txt";
  - "en tal archivo cambiá X por Y" o "agregale una línea";
  - "creá un archivo con...".
- **Seguridad (obligatoria):**
  - **Abrir y leer:** lo hace directo, solo dentro de las carpetas permitidas.
  - **Modificar, crear o borrar:** siempre me muestra qué va a cambiar (un antes y después) y espera que le diga que sí. Ya existe un mecanismo de confirmación para borrar (`AccionPendiente` en `asistente/herramientas.rs` y `ciclo.rs`): reusalo.
  - **Deshacer:** guarda una copia antes de modificar, así le puedo pedir "deshacé eso".
  - **Carpetas permitidas:** se configuran en Ajustes. Por defecto, solo Documentos, Escritorio y Descargas. Nada de `C:\Windows`, `Program Files` ni carpetas de sistema.
  - **Direcciones:** solo `http` y `https`. Nada de `file:`, `javascript:` ni otros esquemas.
  - **Nada de comandos sueltos:** no corre comandos de consola que arme el modelo. Cada acción es una herramienta concreta con argumentos validados en Rust.
  - **Contenido leído:** lo que lee de un archivo es dato, no órdenes. Si un archivo dice "borrá todo", no lo hace.
  - Un interruptor en Ajustes para apagar todo el acceso a la compu.

### 2. Que no vuelva a gastar de más
En v0.6.1 se bajó el gasto de tokens: las herramientas se mandan según el tema del pedido (`asistente/seleccion.rs`). Las herramientas nuevas tienen que sumarse ahí con su tema y sus palabras clave ("abrí", "poneme", "youtube", "google", "archivo", "carpeta"…), con tests. Medí con `cargo test tokens_por_pedido -- --ignored --nocapture` antes y después (gasta del límite diario de Groq: avisame antes).

### 3. Extras (proponé cuáles entran)
- **Controlar la música:** "pausá", "siguiente tema", "subí el volumen", con las teclas de medios de Windows.
- **Temporizadores y recordatorios rápidos:** "avisame en 10 minutos que saque la ropa", con los avisos de v0.6.
- **Portapapeles:** "resumime lo que copié", "copiá esto".
- **Ventanas:** "minimizá todo", "cerrá el navegador" (con confirmación para cerrar).
- **Horas ambiguas:** "de 6 a 7" se agendó a las 06:00 cuando era a la tarde. Que pregunte, o que use el contexto.

**Terminado cuando:**
- Por voz, "Oye Nova, poneme el primer video que sale cuando buscás Riquelme" abre ese video.
- "Buscame en Google recetas de pizza" abre la pestaña con la búsqueda.
- "Abrí Visual Studio Code" lo abre.
- "En notas.txt cambiá martes por jueves" me muestra el cambio, le digo que sí, lo hace y lo puedo deshacer.
- Le pido algo fuera de las carpetas permitidas y no lo hace.
- Un pedido típico no gasta más tokens que en v0.6.1.

## Lo que ya existe y hay que reusar
- **Asistente** (`src-tauri/src/asistente/`):
  - `ciclo.rs`: el turno modelo → herramientas → modelo, con `MAXIMO_VUELTAS`, `MENSAJES_DE_HISTORIAL`, y la acción pendiente que pide confirmación.
  - `seleccion.rs`: qué herramientas van según el pedido.
  - `herramientas.rs`: `definiciones()` y `ejecutar()`. Tiene tests con base en memoria.
  - `prompt.rs`: el prompt de sistema.
  - `proveedor.rs`: Groq y Ollama, con `MODELO_GROQ` y `MODELO_OLLAMA`.
  - `openai.rs`: `ClienteCompatible`. Cuenta los tokens y explica los límites (`mensaje_de_limite`).
- **Lo que necesite el sistema:** va en Rust. Hay ejemplos con `windows-sys` en `voz/sistema.rs`. Para abrir direcciones y archivos puede servir `tauri-plugin-opener` o `tauri-plugin-shell`; evaluá cuál, con permisos acotados en `src-tauri/capabilities/`.
- **Ajustes:** `src/app/Ajustes.tsx` y sus secciones; `AjustesAsistente` es donde va lo del modelo y el acceso a la compu. Las preferencias se validan en Rust (`repos/preferencias.rs`). Hay un test de contrato en `comandos.rs` para los nombres de los campos contra `src/lib/tipos.ts`.
- **La respuesta en pantalla:** `src/app/Respuesta.tsx` y `store/asistente.ts`. Ahí se tiene que ver el antes y después de un cambio de archivo y la pregunta de confirmación.
- **Navegador sin Tauri:** `src/mocks/apiMemoria.ts` trae un asistente de prueba. Sumale casos para las herramientas nuevas.
- **Avisos:** `notificaciones.rs` y `fondo.rs`, para los temporizadores si entran.

## Estética (obligatoria)
- **Dirección "Atmósfera"** (descripta en CLAUDE.md): cielo según la hora, esfera de luz, widgets sin tarjetas. Tokens en `src/theme/tokens.css`. Tipografías Fraunces + Instrument Sans.
- **El antes y después de un archivo:** sin tarjetas pesadas ni estilo de editor de código; legible y tranquilo, siguiendo la estética.
- **Textos:** en español rioplatense, sin etiquetas en mayúsculas ni monoespaciadas.

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **YouTube:** cómo encuentra "el primer video que sale". Opciones: la API oficial de YouTube (pide una key, que va al llavero; verificá HOY su cuota gratis), leer la página de resultados (sin key, pero frágil), u otra. Esto no es la API de búsqueda de v0.7.3: acá solo hace falta el link del video.
2. **Programas:** cómo los encuentra (menú Inicio, registro, una lista mía) y qué pasa si hay dos parecidos.
3. **Archivos:**
   - qué carpetas vienen permitidas;
   - qué tipos de archivo puede tocar (¿solo texto?);
   - cómo se ve la confirmación;
   - dónde y cuánto tiempo guarda las copias para deshacer.
4. **Herramientas por pedido:** cómo se suman las nuevas a `seleccion.rs` sin que se manden de más.
5. **Qué extras entran** y cuáles quedan para después.

Después de decidir: listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- **Contestame siempre en español.**
- **Lógica pura y con tests:** va separada (validar rutas dentro de las carpetas permitidas, validar direcciones, armar el antes y después, a qué tema pertenece cada herramienta nueva, horas ambiguas). Lo que toca el sistema va en Rust, con comandos tipados.
- **Las rutas:** se validan en Rust, después de resolverlas (`canonicalize`), para que `..` o un acceso directo no saquen a Nova de las carpetas permitidas. Esto tiene que tener tests.
- **Tests que abren programas, usan internet o gastan cuota:** se marcan `#[ignore]`. Los de Azure, ElevenLabs y OpenAI gastan mi cuota: no los corras sin preguntarme.
- **Rust:** explicame brevemente lo que no sea obvio (soy fuerte en web, no en Rust).
- **Commits:** chicos y en español (`feat(compu): ...`, `feat(asistente): ...`), con la línea de coautoría que corresponda.
- **Preguntá antes** si una decisión no está en la especificación y cambia algo importante: arquitectura, dependencias pesadas, datos, permisos.
- **PowerShell:**
  - usá `npm.cmd` (`npm.ps1` está bloqueado);
  - **no reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- **Tests de Rust con `tauri dev` abierto:** usá otra carpeta de build (`CARGO_TARGET_DIR` en una carpeta temporal), así no se pisan.
- **Keys:** nunca en el código, en el repo ni en archivos de texto. Van al llavero (`llavero.rs`) y las cargo yo en Ajustes.
- **Al terminar:**
  - verificá el criterio de terminado;
  - actualizá el estado en CLAUDE.md y escribí la sección de decisiones de v0.7.1 en SPEC.md;
  - avisame antes de fusionar a `main`.
- **Fusionar sin checkout:**
  1. `git commit-tree "v0.7.1^{tree}" -p main -p v0.7.1 -m "..."`
  2. `git update-ref refs/heads/main <commit> <main anterior>`
