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
- Versión en curso: **v0.2** (por empezar)
- Completadas: v0.1 (esqueleto y panel), v0.1.1 (rediseño "Atmósfera" y escucha sin botón)
- Notas:
  - Dirección visual aprobada: **"Atmósfera"**. Cielo translúcido que cambia según la hora (`src/theme/cielo.ts` + bloques `data-cielo` en `tokens.css`), esfera de luz en el centro y widgets sin tarjetas que la orbitan (`src/app/Orbita.tsx`). Tipografías Fraunces + Instrument Sans. Nada de etiquetas en mayúsculas ni monoespaciadas. Los widgets nuevos tienen que seguir esta estética.
  - Se descartaron dos estéticas: vidrio con violeta (v0.1) y negro con ámbar tipo "instrumento".
  - Voz: sin botón de micrófono. Nova escucha sola al abrir el panel (SPEC 5.1, en v0.5).
  - Para ver otro momento del día en el navegador: `http://localhost:1420/?cielo=tarde`.
  - En PowerShell, `npm.ps1` está bloqueado por la política de ejecución: usar `npm.cmd` (ej. `npm.cmd run tauri dev`).
  - Decisiones pendientes para v0.2: SQLite desde Rust (recomendado) o con el plugin SQL desde el frontend; eventos recurrentes (clases semanales, cumpleaños anuales); qué pasa con las responsabilidades automáticas si se edita o borra el evento; guardar eventos de todo el día como fecha sola.
  - La ventana del panel se crea una vez y se muestra u oculta; el frontend nunca se desmonta (para abrir en menos de 300 ms).
