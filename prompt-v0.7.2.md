Vamos a hacer la **v0.7.2 de Nova (Google Calendar)**. Repo: https://github.com/AugusTraver/Nova. La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

La v0.7 se partió en tres: v0.7.1 (Nova usa la compu, ya terminada y fusionada a `main`), **v0.7.2** (esta: Google Calendar) y v0.7.3 (buscar información en internet).

## Antes de nada
1. **La rama.** `main` ya tiene todo v0.7.1 y es igual a la rama `v0.7.1`. Creá `v0.7.2` desde `main` con `git switch -c v0.7.2 main`: como los archivos son los mismos, no toca el disco aunque Nova esté corriendo. Revisá igual con `git diff --stat main v0.7.1` antes y avisame.
2. **PLAN.md ya tiene esta v0.7.2.** Si algo de lo que sigue no coincide, proponeme cómo queda.
3. Leé estos tres archivos. CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
   - `CLAUDE.md`;
   - `SPEC.md`, sobre todo 4.1 (Calendario), 4.2 y 6 (decisiones de v0.2: recurrencia, excepciones y responsabilidades automáticas), 4.8 (datos de internet y caché), 5.5 (herramientas del asistente), 5.8 (tokens), 5.9 (decisiones de v0.7.1) y 7 "Configuración";
   - `PLAN.md`, la sección v0.7.2.
4. **Si querés, usá `/find-skills`** para buscar skills de Google Calendar API, OAuth en apps de escritorio o sincronización de datos. Ya están instaladas tauri-v2, rust-best-practices, rust-async-patterns, rust-testing, vitest, better-accessibility, animate, emil-design-eng, frontend-design y security-and-hardening. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y **preguntame antes de instalar**.
5. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas con archivos distintos mientras corre sin avisarme.**

## Qué tiene que lograr v0.7.2
- **Los dos sentidos:**
  - lo que agendo en Nova (a mano o con el asistente) aparece en mi Google Calendar;
  - lo que agendo en Calendar (desde el celular, por ejemplo) aparece en Nova;
  - editar o borrar de un lado se refleja en el otro, sin duplicados.
- **Las responsabilidades automáticas** ("estudiar para el parcial") siguen saliendo de los eventos, vengan de donde vengan. No se suben a Calendar como eventos.
- **Sin internet:** Nova funciona igual con lo local y sincroniza cuando vuelve. Si falla, el widget del calendario muestra un aviso chico sin romper el panel.
- **En vivo:** cuando llega algo de Calendar, el widget se actualiza solo (ya existe `escucharCambios` en el frontend para cuando el asistente cambia algo).
- **Conectar y desconectar** desde Ajustes. El permiso de Google (OAuth) va al llavero, nunca al repo ni a archivos de texto. Desconectar no borra mis eventos de ningún lado.
- **El asistente:** las herramientas de la agenda no cambian; la sincronización pasa por debajo. Que no sume tokens a los pedidos.

**Terminado cuando:**
- Creo un evento en Nova y aparece en Google Calendar.
- Creo uno en Calendar desde el celular y aparece en Nova, con sus responsabilidades automáticas si es un examen o una entrega.
- Edito o borro de un lado y se ve en el otro.
- Sin internet, Nova sigue andando y sincroniza cuando vuelve.

## Lo que ya existe y hay que reusar
- **Agenda:** `repos/eventos.rs` (recurrencia semanal y anual con excepciones), las automáticas en `dominio/` (`reglas.rs`), y los eventos de todo el día como fecha sola.
- **Datos de internet:** el patrón de `futbol/`, `noticias/` y `clima/`, con caché en `repos/cache.rs` y la tarea de fondo en `fondo.rs` (que ya sabe despertarse antes con `Fondo::despertar`). La sincronización puede ir ahí.
- **Llavero:** `llavero.rs`. Como en v0.7.1 con la key de YouTube: Rust nunca le devuelve el secreto a la interfaz, solo si hay uno guardado.
- **Abrir el navegador:** desde Rust con `compu::sistema::abrir` (`ShellExecuteW`), que es lo que usa v0.7.1. El webview no tiene `opener`.
- **Ajustes:** `src/app/Ajustes.tsx` y sus secciones (la de la compu, `AjustesCompu.tsx`, es un buen modelo). Las preferencias se validan en Rust (`repos/preferencias.rs`). Hay un test de contrato en `comandos.rs` para los nombres de los campos contra `src/lib/tipos.ts`.
- **Migraciones:** van por la 6 (`006_temporizadores.sql`); la que sigue es la 7.
- **Widget del calendario:** `src/widgets/Calendario.tsx`. Desde v0.7.1 los widgets se arrastran (`Arrastrable` en `app/Orbita.tsx`): lo nuevo no tiene que romper eso.
- **Navegador sin Tauri:** `src/mocks/apiMemoria.ts`. Sumale un Calendar de prueba (conectar, un evento que "llega" de Google, un error de red).

## Estética (obligatoria)
- **Dirección "Atmósfera"** (descripta en CLAUDE.md). Tokens en `src/theme/tokens.css`. Tipografías Fraunces + Instrument Sans.
- **Textos:** en español rioplatense, sin etiquetas en mayúsculas ni monoespaciadas.
- Si un evento vino de Calendar, que se note apenas (un detalle sutil, no una etiqueta).

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **Cómo se conecta:** OAuth de Google con una app mía en Google Cloud. Verificá HOY qué pide Google para una app personal (modo de prueba, cuánto dura el permiso, si hay que verificar la app, qué alcance mínimo de Calendar alcanza) y cómo se hace el OAuth desde una app de escritorio (navegador, redirección a localhost y PKCE). Decime qué tengo que hacer yo en Google Cloud, paso a paso. El ID de cliente y el secreto de la app también van al llavero, cargados por mí en Ajustes.
2. **Qué calendario usa:** el principal o uno "Nova", y qué pasa con los eventos que ya tengo en cada lado la primera vez.
3. **Cada cuánto sincroniza** (o si usa los avisos de cambios de Google), cómo pide solo lo que cambió (`syncToken`) y **quién gana** si cambié lo mismo de los dos lados.
4. **Los datos:** cómo se guardan en SQLite los ids de Google y lo que falta subir cuando no hay internet. Cambia datos: proponeme la migración.
5. **La recurrencia:** cómo encajan la semanal y anual con excepciones de v0.2 con las de Google (RRULE), y qué hacer con las que Nova no sabe representar.
6. **Dependencias:** si hace falta un crate de OAuth o de Google, cuál y cuánto pesa. Preferí `reqwest` (ya está) si alcanza.

Después de decidir: listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- **Contestame siempre en español.**
- **Lógica pura y con tests:** va separada (qué cambió de cada lado, quién gana, cómo se traduce la recurrencia). Lo que toca el sistema va en Rust, con comandos tipados.
- **Tests que usan internet o mi cuenta de Google:** se marcan `#[ignore]` y no los corras sin preguntarme. Los de Groq gastan del límite diario: avisame antes.
- **Rust:** explicame brevemente lo que no sea obvio (soy fuerte en web, no en Rust).
- **Commits:** chicos y en español (`feat(calendar): ...`), con la línea de coautoría que corresponda.
- **Preguntá antes** si una decisión no está en la especificación y cambia algo importante: arquitectura, dependencias pesadas, datos, permisos.
- **PowerShell:**
  - usá `npm.cmd` (`npm.ps1` está bloqueado);
  - **no reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- **Editar con scripts:** los heredocs largos de bash se rompen. Para cambios grandes, usá la herramienta de editar o un `.py` en la carpeta temporal corrido con `python -I` (con `PYTHONIOENCODING=utf-8` si imprime acentos).
- **Chequear tipos:** `npx.cmd tsc -b` deja `vite.config.js`, `vite.config.d.ts` y `tsconfig.tsbuildinfo` sueltos, y Vite podría leer el `.js` en vez del `.ts`: borralos después.
- **Tests de Rust con `tauri dev` abierto:** usá otra carpeta de build (`CARGO_TARGET_DIR=%LOCALAPPDATA%\Temp\nova-tests`), así no se pisan.
- **Keys y permisos:** nunca en el código, en el repo ni en archivos de texto. Van al llavero (`llavero.rs`) y los cargo yo en Ajustes.
- **Al terminar:**
  - verificá el criterio de terminado;
  - actualizá el estado en CLAUDE.md y escribí la sección de decisiones de v0.7.2 en SPEC.md;
  - avisame antes de fusionar a `main`.
- **Fusionar sin checkout:**
  1. `git commit-tree "v0.7.2^{tree}" -p main -p v0.7.2 -m "..."`
  2. `git update-ref refs/heads/main <commit> <main anterior>`
