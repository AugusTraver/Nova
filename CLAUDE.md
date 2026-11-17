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
- Versión en curso: **ninguna (arrancar por v0.1)**
- Completadas: —
- Notas:
