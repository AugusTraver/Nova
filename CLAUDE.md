# Instrucciones para Claude Code — proyecto Nova

## Contexto
- Nova es una app de escritorio personal: un panel que se abre con un atajo, con un asistente de IA con voz en el centro y widgets alrededor.
- La especificación completa está en `SPEC.md` y el plan por versiones en `PLAN.md`. **Leelos antes de cada tarea importante.**
- Stack: Tauri 2 (Rust) + React + TypeScript + Vite + Tailwind + Framer Motion + Three.js (@react-three/fiber) + Zustand + SQLite.
- El dueño del proyecto estudia informática y maneja bien web, pero no Rust. Cuando escribas Rust, explicá brevemente las partes no obvias.

## Cómo trabajar
- Trabajar **una versión a la vez**, en el orden de `PLAN.md`. No adelantar funcionalidades de versiones futuras salvo que te lo pida.
- Al empezar una versión: crear la rama `vX.Y`, listar las tareas y confirmar el plan conmigo.
- Commits chicos y descriptivos, en español (ej: `feat(responsabilidades): marcar como hecha con animación`).
- Al terminar una versión: verificar su criterio de "terminado", actualizar el estado abajo y avisarme antes de fusionar a `main`.
- Si una decisión no está en la especificación y cambia algo importante (arquitectura, dependencias pesadas, datos), **preguntá antes**.
- Si la especificación tiene algo que no conviene, decímelo y proponé una alternativa en vez de hacerlo igual.

## Reglas de código
- TypeScript estricto. Nada de `any` salvo justificado.
- Lógica de negocio (reglas de responsabilidades automáticas, resolución de fechas, herramientas del asistente) separada de los componentes y con **tests**.
- Todo lo que toca el sistema (atajos, ventanas, audio, SQLite, llavero, procesos sidecar) va en Rust, expuesto con comandos de Tauri tipados.
- Colores, espaciados y tipografía en tokens o variables de tema. Nada de colores sueltos en los componentes.
- Cada widget es independiente: si sus datos fallan, muestra un estado de error o caché sin romper el panel.
- Textos de la interfaz en español rioplatense.

## Seguridad
- **Nunca** guardar API keys, tokens o datos personales en el código ni en el repo.
- Secretos en el llavero del sistema. `.env`, bases de datos locales y modelos descargados en `.gitignore`.
- Los modelos de Whisper y Piper se descargan en una carpeta de datos de la app, no dentro del repo.

## Estado actual
- Versión en curso: **v0.6** (hecha en la rama `v0.6`, falta probarla un día entero y fusionar a `main`)
- Completadas: v0.1 (esqueleto y panel), v0.1.1 (rediseño "Atmósfera" y escucha sin botón), v0.2 (datos reales locales), v0.3 (asistente por texto con Groq u Ollama), v0.4 (fútbol, noticias, resumen del día y clima), v0.5 (voz local con Whisper y Piper)
- Notas:
  - Dirección visual aprobada: **"Atmósfera"**. Cielo translúcido que cambia según la hora (`src/theme/cielo.ts` + bloques `data-cielo` en `tokens.css`), esfera de luz en el centro y widgets sin tarjetas que la orbitan (`src/app/Orbita.tsx`). Tipografías Fraunces + Instrument Sans. Nada de etiquetas en mayúsculas ni monoespaciadas. Los widgets nuevos tienen que seguir esta estética.
  - Se descartaron dos estéticas: vidrio con violeta (v0.1) y negro con ámbar tipo "instrumento".
  - Voz: sin botón de micrófono. Nova escucha sola al abrir el panel (SPEC 5.1, en v0.5).
  - Para ver otro momento del día en el navegador: `http://localhost:1420/?cielo=tarde`.
  - En PowerShell, `npm.ps1` está bloqueado por la política de ejecución: usar `npm.cmd` (ej. `npm.cmd run tauri dev`).
  - Decisiones de v0.2 (detalle en SPEC 4.2 y 6): SQLite desde Rust con `rusqlite`; recurrencia semanal/anual con excepciones; las automáticas siguen al evento salvo que estén hechas, editadas o descartadas; eventos de todo el día como fecha sola.
  - Estructura de Rust: `dominio/` (lógica pura con tests), `repos/` (consultas, testeadas con base en memoria), `comandos.rs` (capa de Tauri). En el frontend, `src/lib/api.ts` espeja los comandos y en el navegador usa `src/mocks/apiMemoria.ts`.
  - Los formularios se abren dentro de cada widget; mientras se edita, el resto del panel se apaga (`edicion` en `src/store/panel.ts`).
  - En PowerShell 5.1 no reescribir archivos con `Get-Content`/`Set-Content`: rompen los acentos (leen como ANSI).
  - La ventana del panel se crea una vez y se muestra u oculta; el frontend nunca se desmonta (para abrir en menos de 300 ms).
  - Asistente (v0.3, detalle en SPEC 5.5): el ciclo, las herramientas y el proveedor están en `src-tauri/src/asistente/`; el historial, en `repos/mensajes.rs`; las fechas relativas, en `dominio/fechas_relativas.rs`. En el frontend están `store/asistente.ts` y `app/Respuesta.tsx`, que va debajo de la esfera. En el navegador, `apiMemoria` trae un asistente de prueba que entiende "parcial" y "borrá".
  - Para correr los tests de Rust con `tauri dev` abierto: `CARGO_TARGET_DIR` en otra carpeta, así no se pisan los builds.
  - Datos de internet (v0.4, detalle en SPEC 4.8): `src-tauri/src/futbol/` (ESPN), `noticias/` (RSS), `clima/` (Open-Meteo); la caché en `repos/cache.rs` y la tarea de fondo en `fondo.rs`. En el navegador, `src/mocks/externos.ts` trae datos de ejemplo: `?vivo` pone a Boca jugando, `?sinred` muestra los avisos y `?gol` festeja un gol a los 3 segundos.
  - Voz (v0.5, detalle en SPEC 5.6): `src-tauri/src/voz/`. Lo puro (`audio`, `vad`, `texto`) tiene tests comunes. Los que usan el micrófono, la GPU o Groq están marcados `#[ignore]`: `cargo test -- --ignored`, con los modelos bajados. En el navegador, `src/mocks/vozDePrueba.ts` simula los estados (`?voz=no` arranca sin modelos).
  - Para hablar usa la voz que se elija en Ajustes, con un comparador: Azure (`voz/azure.rs`), ElevenLabs (`voz/elevenlabs.rs`) u OpenAI (`voz/openai.rs`), cada una con su key en el llavero. Sin eso o sin internet habla Piper. La síntesis corre en su propio hilo (`voz/sintetizador.rs`). Los tests reales son `cargo test -- --ignored azure|elevenlabs|openai`; el de Azure necesita `NOVA_AZURE_REGION`.
  - Palabra clave y avisos (v0.6, detalle en SPEC 5.7): "Oye Nova" en `voz/palabra.rs` (modelo `oye_nova.onnx` en la carpeta de modelos de voz, no en el repo), pausas en `voz/sistema.rs`, bandeja en `bandeja.rs`. Los avisos: lógica en `dominio/avisos.rs`, lo ya mandado en `repos/avisos.rs`, las notificaciones en `notificaciones.rs` y la revisión cada minuto en `fondo.rs`. En el navegador, `?despertar` hace el pulso de la esfera a los 2 segundos.
  - Para probar la palabra clave con una grabación: `NOVA_GRABACION_PALABRA=archivo.wav NOVA_VECES_PALABRA=11 cargo test --release -- --ignored detecta_en_una_grabacion --nocapture` (WAV mono de 16 kHz).
  - "Sobre vos" (perfil) en `dominio/perfil.rs`: cómo llamarte, tono y chistes, y un texto libre que va al prompt.
