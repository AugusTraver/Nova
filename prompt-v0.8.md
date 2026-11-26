Vamos a hacer la **v0.8 de Nova (Instalable y de todos los días)**. Repo: https://github.com/AugusTraver/Nova. La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

## Antes de nada
1. **Revisá en qué rama estamos.** Antes vinieron v0.7.1 (Nova usa la compu), v0.7.2 (Google Calendar) y v0.7.3 (buscar información en internet), cada una en su rama. Si la última no está fusionada a `main`, preguntame si la probé y si anduvo bien; recién ahí fusionala sin checkout (ver abajo) y creá `v0.8` desde `main` con `git branch v0.8 main`.
2. **PLAN.md:** después de v0.7.3 viene v1.0 "Pulido". La idea es partir v1.0 en dos:
   - **v0.8 (esta):** que Nova se instale, arranque sola con Windows y aguante el uso diario sin sorpresas.
   - **v1.0:** la estética final con sonidos, el README con capturas y las conexiones (celular, Gmail, Spotify, Google Drive, GitHub, Discord y WhatsApp opcional). Google Calendar ya entró en v0.7.2.

   Proponeme cómo queda la sección en PLAN.md (objetivo, lista y "terminado cuando") y escribila cuando la apruebe.
3. Leé estos tres archivos. CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
   - `CLAUDE.md`;
   - `SPEC.md`, sobre todo 3.1 "Comportamiento del panel" (arranque con el sistema), 5.6 "Decisiones de v0.5 (voz)", 5.7 "Decisiones de v0.6", las secciones de decisiones de v0.7.1, v0.7.2 y v0.7.3, 7 "Configuración" y 8 "Requisitos no funcionales";
   - `PLAN.md`, las secciones v0.7.1 a v0.7.3 y v1.0.
4. **Si querés, usá `/find-skills`** para buscar skills de:
   - instaladores de Tauri 2 en Windows (NSIS o MSI) y recursos que van dentro del instalador;
   - arranque con el sistema (`tauri-plugin-autostart`);
   - actualizaciones (`tauri-plugin-updater`), solo para evaluar si conviene ahora;
   - medir rendimiento en React y WebView2.

   Ya están instaladas tauri-v2, rust-best-practices, rust-async-patterns, rust-testing, vitest, better-accessibility, animate, emil-design-eng y frontend-design. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y **preguntame antes de instalar**.
5. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas mientras corre sin avisarme.** Si `main` y la rama actual tienen los mismos archivos (`git diff --stat`), cambiar de rama no toca el disco: avisame y hacelo.

## Qué tiene que lograr v0.8
- **Instalador para Windows** (`npm.cmd run tauri build`): que se instale, que se desinstale limpio y que los datos (`%APPDATA%\com.augustraver.nova`, `%LOCALAPPDATA%\com.augustraver.nova\voz`) y las keys del llavero sobrevivan a una actualización.
- **Arranque con Windows:** opcional y configurable en Ajustes (SPEC 3.1). Tiene que arrancar oculto, solo en la bandeja.
- **El modelo de "Oye Nova"** (`oye_nova.onnx`, unos 7 MB) hoy NO está en el repo: solo vive en `%LOCALAPPDATA%\com.augustraver.nova\voz\modelos\`. Con una instalación nueva tiene que llegar solo.
- **Las notificaciones con nombre propio:** en desarrollo salen como "Windows PowerShell" porque Nova no tiene un AppUserModelID registrado. Instalada tiene que decir "Nova" y tocarlas tiene que abrir el panel, también desde el centro de notificaciones.
- **Rendimiento medido en la versión instalada** (SPEC 8):
  - el panel abre en menos de 300 ms;
  - la esfera y las animaciones van a 60 fps;
  - consumo en reposo con el panel cerrado y "Oye Nova" activa: CPU y RAM. Ojo: Whisper large queda cargado (~285 MB) para confirmar la palabra.
- **Pendientes que quedaron de versiones anteriores** (proponé cuáles entran):
  - **Horas ambiguas:** "de 6 a 7" se agendó a las 06:00 cuando era a la tarde. Que pregunte, o que use el contexto: si ya pasaron las 6 de la mañana de hoy, es a la tarde.
  - **Eventos que se pisan:** el asistente ya avisa, pero los formularios de los widgets no. Avisar también ahí.
  - **El micrófono elegido:** si se desconecta, usar el de Windows y mostrarlo; si vuelve, retomarlo. Ya pasó con el Razer inalámbrico (se corta unos segundos): con el panel abierto, `voz/motor.rs` muestra el error y no reintenta; con el panel cerrado, reintenta una sola vez, enseguida, y se rinde. Hay que reintentar cada pocos segundos sin llenar la consola, y que alcance con cerrar y abrir el panel.
  - **La consola:** el aviso "se perdió un pedacito de audio" sale muy seguido en desarrollo; juntarlo o bajarle el volumen.
  - **Clippy:** tres avisos de estilo (`futbol/vista.rs:55`, `voz/audio.rs:167` y `:195`).
  - **Un test mío sin commitear:** `lee_el_resumen_al_abrir`, en `voz/mod.rs`. Preguntame si va.
- **Terminado cuando:** instalo Nova desde el instalador en esta compu, reinicio Windows, arranca sola en la bandeja, "Oye Nova" la abre, me llega un aviso que dice "Nova" y la uso una semana sin abrir la terminal.

## Lo que ya existe y hay que reusar
- **Configuración de Tauri:** `src-tauri/tauri.conf.json` (identificador `com.augustraver.nova`, ventana oculta, sin bordes, transparente y siempre adelante).
- **Arranque de la app:** `src-tauri/src/lib.rs` (`setup`). Ahí se crean la base, la tarea de fondo, la voz, la bandeja y el atajo, y se arranca el motor si "Oye Nova" está activa.
- **Voz** (`src-tauri/src/voz/`):
  - `modelos.rs` baja Whisper y Piper con progreso (evento `voz:descarga`). Es el lugar natural para que también baje `oye_nova.onnx` (por ejemplo, desde un *release* de GitHub), si no va dentro del instalador.
  - `palabra.rs` es el detector. Tiene una prueba con grabación: `NOVA_GRABACION_PALABRA=archivo.wav NOVA_VECES_PALABRA=11 cargo test --release -- --ignored detecta_en_una_grabacion --nocapture`. También mide la CPU.
  - `sistema.rs` sabe si la sesión está bloqueada o la compu va a batería. `dispositivos.rs` tiene el micrófono elegido por nombre.
- **Avisos:** `dominio/avisos.rs` (puro), `repos/avisos.rs`, `notificaciones.rs` (`tauri-winrt-notification`; el AppUserModelID está ahí) y `fondo.rs`.
- **Asistente:** `asistente/` (ciclo, herramientas, proveedores Groq y Ollama). Los errores llegan al frontend con `mensajeDelAsistente` (`src/lib/api.ts`).
- **Ajustes:** `src/app/Ajustes.tsx` y sus secciones (`AjustesPerfil`, `AjustesAvisos`, `AjustesVoz`, `AjustesAsistente`, `AjustesInternet`). Las preferencias se validan en Rust (`repos/preferencias.rs`). Hay un test de contrato en `comandos.rs` para los nombres de los campos JSON contra `src/lib/tipos.ts`.
- **Navegador sin Tauri:** `src/mocks/apiMemoria.ts` y `vozDePrueba.ts`. Parámetros de prueba: `?cielo=noche`, `?vivo`, `?sinred`, `?gol`, `?despertar`, `?voz=no`.
- **Compilación rápida:** `src-tauri/.cargo/config.toml` usa `rust-lld` y el perfil de desarrollo guarda menos información de depuración. Si el instalador (release) falla al armar el `.exe`, revisá eso primero.

## Estética (obligatoria)
- **Dirección "Atmósfera"** (descripta en CLAUDE.md): cielo según la hora, esfera de luz, widgets sin tarjetas que la orbitan. Tokens en `src/theme/tokens.css`. Tipografías Fraunces + Instrument Sans.
- **Textos:** en español rioplatense, sin etiquetas en mayúsculas ni monoespaciadas.
- **Íconos nuevos** (el del instalador, el de las notificaciones y el de la bandeja con su puntito de "escuchando"): que sigan esa estética. Mostrame opciones antes de cambiar el ícono de la app.

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **Instalador:** NSIS o MSI. Si se instala para mi usuario o para toda la compu. Qué pasa con la firma (sin certificado, Windows SmartScreen avisa). Verificá HOY cómo lo hace Tauri 2.
2. **El modelo de "Oye Nova":** si va dentro del instalador (recurso de Tauri, +7 MB) o se baja con los de voz desde un release de GitHub. ¿Y si algún día lo reentreno?
3. **Arranque con Windows:** el plugin o una entrada del registro a mano, si viene activado por defecto, y cómo se ve en Ajustes.
4. **Actualizaciones:** si conviene `tauri-plugin-updater` ahora o lo dejamos para v1.0, y qué hace falta para firmarlas.
5. **Rendimiento:** cómo medimos cada número (herramienta, cuántas veces, en qué estado) y qué hacemos si Whisper large en memoria es mucho. Por ejemplo, un Whisper más chico solo para confirmar "nova", o cerrarlo y confiar más en el detector.
6. **Qué pendientes entran** y cuáles quedan para v1.0.

Después de decidir: creá la rama `v0.8`, listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- **Contestame siempre en español.**
- **Lógica pura y con tests:** va separada (por ejemplo, la hora ambigua, el reintento por cuota, qué micrófono usar). Lo que toca el sistema va en Rust, con comandos tipados.
- **Tests con audio, GPU o internet:** se marcan `#[ignore]` (`cargo test -- --ignored`). Los de Azure, ElevenLabs y OpenAI gastan mi cuota: no los corras sin preguntarme.
- **Rust:** explicame brevemente lo que no sea obvio (soy fuerte en web, no en Rust).
- **Commits:** chicos y en español (`feat(instalador): ...`, `fix(asistente): ...`), con la línea de coautoría que corresponda.
- **Preguntá antes** si una decisión no está en la especificación y cambia algo importante: arquitectura, dependencias pesadas, datos.
- **PowerShell:**
  - usá `npm.cmd` (`npm.ps1` está bloqueado);
  - **no reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- **Tests de Rust con `tauri dev` abierto:** usá otra carpeta de build (`CARGO_TARGET_DIR` en una carpeta temporal), así no se pisan. `tauri dev` recompila solo cada vez que cambia el Rust. Un `tauri build` (release, con LTO) tarda bastante más: corrélo en segundo plano.
- **Keys:** nunca en el código, en el repo ni en archivos de texto. Van al llavero (`llavero.rs`) y las cargo yo en Ajustes.
- **Al terminar:**
  - verificá el criterio de terminado;
  - actualizá el estado en CLAUDE.md y la sección de decisiones en SPEC.md;
  - avisame antes de fusionar a `main`.
- **Fusionar sin checkout:**
  1. `git commit-tree "v0.8^{tree}" -p main -p v0.8 -m "..."`
  2. `git update-ref refs/heads/main <commit> <main anterior>`
