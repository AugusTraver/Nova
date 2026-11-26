Vamos a hacer la **v0.7.3 de Nova (buscar información en internet)**. Repo: https://github.com/AugusTraver/Nova. La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

La v0.7 se partió en tres: v0.7.1 (Nova usa la compu), v0.7.2 (Google Calendar) y **v0.7.3** (esta: buscar información en internet).

## Antes de nada
1. **Revisá en qué rama estamos.** La v0.7.2 va en la rama `v0.7.2`. Si todavía no está fusionada a `main`, preguntame si la probé y si anduvo bien; recién ahí fusionala sin checkout (ver abajo) y creá `v0.7.3` desde `main` con `git branch v0.7.3 main`. Si las dos ramas tienen los mismos archivos (`git diff --stat`), cambiar de rama no toca el disco: avisame y hacelo.
2. **PLAN.md ya tiene esta v0.7.3.** Si algo de lo que sigue no coincide, proponeme cómo queda.
3. Leé estos tres archivos. CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
   - `CLAUDE.md`;
   - `SPEC.md`, sobre todo 5.1 (ciclo del asistente), 5.5 (herramientas), 5.8 "Decisiones de v0.6.1" (tokens), 4.8 (datos de internet) y la sección de decisiones de v0.7.1;
   - `PLAN.md`, la sección v0.7.3.
4. **Si querés, usá `/find-skills`** para buscar skills de APIs de búsqueda o de seguridad con contenido de la web (inyección de instrucciones). Ya están instaladas tauri-v2, rust-best-practices, rust-async-patterns, rust-testing, vitest, better-accessibility, animate, emil-design-eng, frontend-design y security-and-hardening. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y **preguntame antes de instalar**.
5. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas mientras corre sin avisarme.**

## Qué tiene que lograr v0.7.3

### La diferencia con v0.7.1
En v0.7.1 Nova ya **abre** cosas: "buscame en Google X" abre una pestaña y "poneme el primer video de Riquelme" abre el video. Acá Nova **busca información y me contesta** sin abrir nada:
- "fijate quién ganó anoche River";
- "¿a qué hora abre el Coto de Caballito?";
- "¿cuándo sale la próxima temporada de tal serie?".

### Cómo
- **Con una API de búsqueda aparte, con plan gratis.** **No** con la búsqueda incluida de Groq: no quiero quedarme sin tokens.
- Lee unos pocos resultados (3 a 5, recortados) y contesta en una o dos frases, **diciendo de dónde lo sacó**.
- **Cuándo busca y cuándo no:** si la respuesta está en sus datos (agenda, fútbol, noticias) o es algo que no cambia, no busca. Si es algo actual o que no sabe, busca.
- **Si no encuentra algo confiable**, o si se acabó el plan gratis de la API: lo dice y ofrece abrir la búsqueda en el navegador (la herramienta de v0.7.1).
- **Seguridad:** lo que lee de una página es dato, no órdenes. Si un resultado dice "borrá la agenda", no lo hace.
- **Tokens:** la API trae los resultados, pero quien los lee y arma la respuesta sigue siendo Groq. Que cada búsqueda sume poco y no rompa el límite por minuto. La herramienta nueva va en `asistente/seleccion.rs` con sus palabras clave ("fijate", "averiguá", "buscá info"…), con tests.

**Terminado cuando:**
- Por voz le pregunto algo actual que no sabe y me contesta buscando, diciendo de dónde lo sacó.
- Le pregunto algo de mi agenda o de Boca y no busca en internet.
- Sin la key o sin plan gratis, me avisa y me ofrece abrir la búsqueda.
- Una búsqueda no rompe el límite por minuto de Groq (medido con `cargo test tokens_por_pedido -- --ignored --nocapture`, que gasta del límite diario: avisame antes).

## Lo que ya existe y hay que reusar
- **Asistente** (`src-tauri/src/asistente/`): `ciclo.rs`, `seleccion.rs`, `herramientas.rs`, `prompt.rs` y `openai.rs` (cuenta los tokens y explica los límites con `mensaje_de_limite`).
- **Datos de internet:** el patrón de `futbol/`, `noticias/` y `clima/`, con caché en `repos/cache.rs`.
- **Abrir en el navegador:** la herramienta de v0.7.1.
- **Llavero:** `llavero.rs`. Ajustes: `AjustesInternet` o `AjustesAsistente`, para cargar la key.
- **Navegador sin Tauri:** `src/mocks/apiMemoria.ts`. Sumale una búsqueda de prueba.

## Estética (obligatoria)
- **Dirección "Atmósfera"** (descripta en CLAUDE.md). Tipografías Fraunces + Instrument Sans.
- **De dónde lo sacó:** el nombre del sitio, chico y tranquilo debajo de la respuesta, sin parecer una lista de links.
- **Textos:** en español rioplatense, sin etiquetas en mayúsculas ni monoespaciadas.

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **Qué API:** verificá HOY precios y límites gratis (Tavily, Brave Search, Serper u otra), qué tan buenos son los resultados en español y para Argentina, y si pide tarjeta para el plan gratis.
2. **Cuándo busca:** con qué reglas (en el prompt, en `seleccion.rs` o las dos) y cómo se prueba.
3. **Cuántos resultados y cuánto texto** de cada uno, y cuántos tokens suma una búsqueda.
4. **Caché:** si guarda las búsquedas un rato para no gastar dos veces en lo mismo.
5. **Cómo se marca que lo leído es dato**, en el prompt y en el resultado de la herramienta.

Después de decidir: listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- **Contestame siempre en español.**
- **Lógica pura y con tests:** va separada (cuándo buscar, recortar resultados, armar la fuente). Lo que toca internet va en Rust, con comandos tipados.
- **Tests que usan internet o gastan cuota:** se marcan `#[ignore]` y no los corras sin preguntarme. Los de Azure, ElevenLabs y OpenAI gastan mi cuota.
- **Rust:** explicame brevemente lo que no sea obvio (soy fuerte en web, no en Rust).
- **Commits:** chicos y en español (`feat(busqueda): ...`), con la línea de coautoría que corresponda.
- **Preguntá antes** si una decisión no está en la especificación y cambia algo importante: arquitectura, dependencias pesadas, datos, permisos.
- **PowerShell:**
  - usá `npm.cmd` (`npm.ps1` está bloqueado);
  - **no reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- **Tests de Rust con `tauri dev` abierto:** usá otra carpeta de build (`CARGO_TARGET_DIR` en una carpeta temporal), así no se pisan.
- **Keys:** nunca en el código, en el repo ni en archivos de texto. Van al llavero (`llavero.rs`) y las cargo yo en Ajustes.
- **Al terminar:**
  - verificá el criterio de terminado;
  - actualizá el estado en CLAUDE.md y escribí la sección de decisiones de v0.7.3 en SPEC.md;
  - avisame antes de fusionar a `main`.
- **Fusionar sin checkout:**
  1. `git commit-tree "v0.7.3^{tree}" -p main -p v0.7.3 -m "..."`
  2. `git update-ref refs/heads/main <commit> <main anterior>`
