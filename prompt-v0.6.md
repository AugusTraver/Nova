Vamos a hacer la **v0.6 de Nova (Palabra clave y proactividad)**. Repo: https://github.com/AugusTraver/Nova. La carpeta local es `C:\Users\Augusto\Downloads\Nova`.

## Antes de nada
1. **Revisá en qué rama estamos.** La v0.5 (voz local, más las voces de Azure, ElevenLabs y OpenAI con su comparador) está en la rama `v0.5` y puede que todavía no esté fusionada a `main`. Si no lo está:
   - avisame;
   - después de que yo diga que sí, fusionala sin checkout (ver abajo);
   - recién ahí creá `v0.6` desde `main` con `git branch v0.6 main`.
2. Leé estos tres archivos. CLAUDE.md tiene las reglas del proyecto y hay que seguirlas.
   - `CLAUDE.md`;
   - `SPEC.md`, sobre todo 5.1 "Flujo", 5.4 "Proactividad", 5.6 "Decisiones de v0.5 (voz)", 4.8 (datos de internet y tarea de fondo) y 7 "Configuración";
   - `PLAN.md`, la sección v0.6.
3. **Si querés, usá `/find-skills`** para buscar skills que sirvan para esta versión. Temas:
   - detección de palabra clave (wake word) en Rust o con un proceso aparte;
   - notificaciones de Windows con Tauri 2 (`tauri-plugin-notification`);
   - procesamiento de audio y ONNX Runtime en Rust;
   - tareas en segundo plano con Tokio;
   - consumo de CPU y energía.

   Ya están instaladas tauri-v2, rust-best-practices, rust-async-patterns, rust-testing, vitest, better-accessibility, animate, emil-design-eng y frontend-design. Proponeme solo las que valgan la pena (más de 1K instalaciones o de una fuente confiable) y **preguntame antes de instalar**.
4. Nova suele estar corriendo con `npm.cmd run tauri dev`. **No hagas checkout de ramas mientras corre sin avisarme.** Si `main` y la rama actual tienen los mismos archivos (`git diff --stat`), cambiar de rama no toca el disco: avisame y hacelo.

## Qué pide v0.6 (PLAN.md)
- **Palabra clave local ("Oye Nova"):**
  - abre el panel y empieza a escuchar;
  - corre en segundo plano con el panel cerrado;
  - el audio nunca sale de la compu.
- **Notificaciones del sistema:**
  - eventos próximos ("En 30 minutos tenés clase");
  - responsabilidades que vencen ("Vence hoy: estudiar para Análisis");
  - partido que arranca ("Arranca Boca").
- **Ajustes:** activar o desactivar cada cosa y un horario de "no molestar".
- **Cuidar el consumo de CPU** de la escucha en segundo plano.
- **Terminado cuando:** uso Nova un día entero solo con la voz y las notificaciones.

## Lo que ya existe y hay que reusar
- **Voz** (`src-tauri/src/voz/`, detalle en SPEC 5.6):
  - `motor.rs` es un hilo dueño del micrófono, el detector de voz (VAD), Whisper y el parlante. Recibe todo por un canal de `Orden`: `Activar` / `Desactivar` al abrir y cerrar el panel, `Hablar`, `Callar`, etc.
  - El micrófono (cpal, en `dispositivos.rs`) **hoy solo se abre con el panel abierto**. Con la palabra clave eso cambia; pensá cómo convive con el motor.
  - `audio.rs` ya trae remuestreo a 16 kHz con filtro pasabajos, RMS y nivel visual. `vad.rs` ya detecta voz por energía con un piso de ruido que se adapta.
  - `sintetizador.rs` convierte las frases en audio en otro hilo. Las voces están en `nube.rs`, `azure.rs`, `elevenlabs.rs` y `openai.rs`, con Piper de respaldo.
  - `modelos.rs` baja programas y modelos a `%LOCALAPPDATA%\com.augustraver.nova\voz` (nunca al repo), con progreso en el evento `voz:descarga`. Si la palabra clave necesita un modelo, se baja igual.
  - Los programas aparte (whisper-server, piper) anotan su PID y se cierran si quedaron huérfanos.
- **Tarea de fondo** (`src-tauri/src/fondo.rs`):
  - Ya corre siempre, aunque el panel esté cerrado. Duerme 1 minuto, o 30 s con un partido en vivo, y el panel la despierta al abrirse.
  - Su comentario dice que existe "así en v0.6 puede avisar «arranca el partido»": **las notificaciones encajan ahí**.
  - Regla de oro: el candado de la base (`db.conexion()`) nunca se retiene durante un `await`.
- **Datos:**
  - `repos/` tiene eventos con recurrencia, responsabilidades y la caché de fútbol, noticias y clima.
  - `dominio/` tiene la lógica pura con tests. El reloj se inyecta con `Ahora`; en los tests se usa `Ahora::fijo_a_las("2026-11-17", "14:00")`.
  - `dominio/futbol.rs` ya sabe cuál es el próximo partido y si está en vivo.
- **Ventana y sistema:**
  - `ventana.rs`: `mostrar`, `alternar`, `abierto`. La ventana se crea una vez y se muestra u oculta; el frontend nunca se desmonta.
  - `bandeja.rs` arma el ícono de la bandeja.
  - El atajo global usa `tauri-plugin-global-shortcut`.
- **Preferencias:** `repos/preferencias.rs` guarda pares clave → JSON (`voz`, `atajo`, `clima`, …). Lo nuevo (palabra clave, notificaciones, no molestar) va ahí, validado en Rust.
- **Migraciones:** van en `src-tauri/migraciones/` (hay hasta `003_...`). Si hace falta una tabla nueva (por ejemplo, "qué notificación ya mandé" para no repetir), **se agrega `004_...sql`**; nunca se edita una vieja.
- **Frontend:**
  - `src/lib/api.ts` espeja los comandos. En el navegador sin Tauri usa `src/mocks/apiMemoria.ts`, y la voz de prueba está en `src/mocks/vozDePrueba.ts`. Hay que sumar ahí versiones de prueba de lo nuevo.
  - Ajustes está en `src/app/Ajustes.tsx`, y la voz en `AjustesVoz.tsx`. Usan las piezas de `src/components/Formulario.tsx` (`Interruptor`, `Opciones`, `Campo`, `BotonTexto`).
  - Hay un test de contrato en `comandos.rs` que verifica que los JSON de Rust y los tipos de `src/lib/tipos.ts` tengan los mismos nombres de campos. Si agregás comandos o tipos, sumalos ahí.

## Estética (obligatoria)
- **Dirección "Atmósfera"** (descripta en CLAUDE.md):
  - cielo translúcido según la hora;
  - esfera de luz en el centro;
  - widgets sin tarjetas que la orbitan.
- **Tokens:** los de `src/theme/tokens.css`, incluido `--color-aviso`.
- **Tipografías:** Fraunces + Instrument Sans.
- **Textos:** en español rioplatense, sin etiquetas en mayúsculas ni monoespaciadas.
- **Esfera al despertar con "Oye Nova":** tiene que dar una señal sutil de que te escuchó. Usá los estados que ya existen (`src/store/panel.ts`, `src/esfera/`) antes de inventar otros.
- **Notificaciones:** cortas, en el mismo tono que Nova.
- **Para probar la interfaz sin compilar Rust:** `http://localhost:1420/?cielo=noche`. Emulá una pantalla de 1600×900.

## Decisiones para resolver conmigo ANTES de escribir código (con tu recomendación para cada una)
1. **Detector de palabra clave.** Verificá HOY, sin confiar en lo que sabías antes, al menos estas tres opciones:
   - openWakeWord (modelos ONNX, se pueden entrenar palabras a medida);
   - Porcupine de Picovoice (palabras a medida, pero con licencia y key);
   - una frase corta transcripta con Whisper `tiny` como segunda confirmación.

   Para cada una quiero:
   - licencia y si necesita cuenta o key;
   - si se puede tener "Oye Nova" (o "Nova") en español, y cómo se entrena;
   - CPU y memoria en reposo en una compu normal;
   - falsos positivos (que se active sola con la tele o charlas) y falsos negativos;
   - cómo se corre desde Rust (crate, ONNX Runtime o un proceso aparte como whisper-server) y cuánto pesa.
2. **Arquitectura de la escucha en segundo plano:**
   - si va dentro del motor de voz actual o en un hilo o proceso aparte;
   - cómo se pasa de "esperando la palabra" a "escuchando el pedido" sin perder lo que digas justo después ("Oye Nova, ¿qué tengo hoy?");
   - qué pasa con Whisper (hoy se cierra a los 10 minutos sin uso).
3. **Consumo:** cuánto gasta escuchar todo el día y cómo se mide.
   - Ideas: el VAD por energía como compuerta antes del detector, y pausar con la compu bloqueada o en batería.
   - Decime números concretos que vamos a verificar.
4. **Privacidad y control:**
   - cómo se sabe que el micrófono está abierto en segundo plano (ícono de la bandeja, por ejemplo);
   - cómo se apaga rápido;
   - que nada se grabe ni se guarde en disco.
5. **Notificaciones:**
   - plugin (`tauri-plugin-notification` u otro);
   - qué pasa al hacerles clic (¿abren el panel?);
   - cuándo se mandan:
     - cuánto antes de un evento, configurable;
     - a qué hora el "vence hoy";
     - "arranca el partido" según `dominio/futbol.rs`;
   - cómo se evita mandarlas dos veces, incluso si se reinicia Nova;
   - cómo se testea la lógica de "qué toca avisar ahora" como función pura.
6. **"No molestar":**
   - horario configurable;
   - si silencia también la palabra clave;
   - qué hace con lo que tocaba avisar mientras tanto (se descarta o se junta en un resumen).
7. **Si Nova lee en voz alta algunos avisos** con el panel cerrado, o si solo manda notificaciones.

Después de decidir: creá la rama `v0.6`, listame las tareas y confirmá el plan conmigo.

## Cómo trabajar (recordatorios de CLAUDE.md y de lo aprendido)
- **Lógica pura y con tests:** va separada; por ejemplo, qué notificación toca ahora, el horario de "no molestar", la elección entre detección y confirmación, y no repetir avisos. Lo que toca el sistema (micrófono, modelos, notificaciones, SQLite) va en Rust, con comandos tipados.
- **Tests con audio:** los que usan el micrófono, la GPU o modelos bajados se marcan `#[ignore]` (se corren con `cargo test -- --ignored`). Para lo demás, usá audio sintético como en `voz/mod.rs` (`mod real`): Piper puede generar "Oye Nova" para probar el detector.
- **Rust:** explicame brevemente lo que no sea obvio (soy fuerte en web, no en Rust).
- **Commits:** chicos y en español (`feat(palabra-clave): ...`, `feat(avisos): ...`), con la línea de coautoría que corresponda.
- **Preguntá antes** si una decisión no está en la especificación y cambia algo importante: arquitectura, dependencias pesadas (ONNX Runtime lo es), datos.
- **PowerShell:**
  - usá `npm.cmd` (`npm.ps1` está bloqueado);
  - **no reescribas archivos con `Get-Content`/`Set-Content` de PowerShell 5.1: rompen los acentos.**
- **Tests de Rust con `tauri dev` abierto:** usá otra carpeta de build (`CARGO_TARGET_DIR` en una carpeta temporal), así no se pisan. `tauri dev` recompila solo cada vez que cambia el Rust.
- **Keys:** nunca en el código, en el repo ni en archivos de texto. Si Porcupine u otro servicio pide una, va al llavero (`llavero.rs`) y la carga el usuario en Ajustes; vos no la escribas en ningún campo.
- **Al terminar:**
  - verificá el criterio de terminado;
  - actualizá el estado en CLAUDE.md y la sección de decisiones en SPEC.md;
  - avisame antes de fusionar a `main`.
- **Fusionar sin checkout:**
  1. `git commit-tree "v0.6^{tree}" -p main -p v0.6 -m "..."`
  2. `git update-ref refs/heads/main <commit> <main anterior>`
