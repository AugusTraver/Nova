# Nova — Plan de desarrollo por versiones

Cada versión termina en algo que **funciona y se puede usar**. No se pasa a la siguiente hasta que la actual cumple su criterio de "terminado".

---

## Paso 0 — Preparación (antes de abrir Claude Code)

### Decisiones tomadas
- **Repositorio:** GitHub, **privado**. Tenés copia de seguridad, historial de cambios y podés volver atrás si algo se rompe. Lo hacés público cuando quieras mostrarlo.
- **Estructura:** un solo repo con la app Tauri (frontend en `src/`, Rust en `src-tauri/`).
- **Flujo de trabajo:** una rama por versión (`v0.1`, `v0.2`…), commits chicos, y se fusiona a `main` cuando la versión está terminada.

### Instalar
1. **Git** y una cuenta de **GitHub**.
2. **Node.js** (versión LTS) y **pnpm** (o npm).
3. **Rust** (con `rustup`).
4. Los **requisitos de Tauri 2 para Windows**: Microsoft C++ Build Tools (con "Desarrollo para el escritorio con C++") y WebView2 (ya viene en Windows 10).
5. **Claude Code**.

### Crear el proyecto
1. Crear una carpeta, por ejemplo `nova/`.
2. Copiar adentro `SPEC.md`, `PLAN.md` y `CLAUDE.md`.
3. (Ya completado: Windows 10 y Boca.)
4. Abrir Claude Code en esa carpeta y pasarle el primer mensaje (al final de este archivo).

---

## v0.1 — Esqueleto y panel
**Objetivo:** que el atajo abra un panel lindo, aunque los datos sean de mentira.
- Proyecto Tauri 2 + React + TypeScript + Vite + Tailwind.
- Repo git inicializado, `.gitignore`, primer commit, subido a GitHub.
- Ventana sin bordes, transparente, a pantalla completa y siempre adelante.
- Atajo global para abrir y cerrar. `Esc` cierra.
- Ícono en la bandeja del sistema con "Abrir" y "Salir".
- Layout de la sección 3.2 de la especificación con widgets que muestran **datos de ejemplo**.
- Esfera central simple (Three.js) con la animación de "respirar".
- Animación de entrada y salida del panel (Framer Motion).
- Barra de texto abajo (todavía no hace nada).

**Terminado cuando:** aprieto el atajo, el panel aparece con animación y se cierra bien, y la app queda en la bandeja.

## v0.2 — Datos reales locales
**Objetivo:** que Calendario, Responsabilidades y Facu funcionen de verdad, a mano.
- SQLite con migraciones y el modelo de datos de la sección 6.
- **Calendario:** crear, editar y borrar eventos desde el widget. Vista de hoy y próximos días.
- **Responsabilidades:** crear, editar, marcar como hecha (con animación) y borrar. Orden según la especificación.
- **Responsabilidades automáticas** a partir de exámenes, entregas y cumpleaños.
- **Facu:** materias, cuenta regresiva a exámenes y entregas, progreso por materia.
- Pantalla de ajustes básica (atajo, reglas automáticas).

**Terminado cuando:** puedo cargar mi semana real de la facu y usarla un par de días.

## v0.3 — Asistente por texto
**Objetivo:** pedirle cosas escribiendo y que las haga.
- Capa `LLMProvider` con un proveedor con plan gratuito y otro con Ollama.
- API keys en el llavero del sistema, configurables desde ajustes.
- Herramientas de calendario, responsabilidades y materias (sección 5.2).
- Ciclo completo: mensaje → modelo → herramientas → respuesta final.
- Resolución de fechas relativas con la hora local.
- Historial de conversación guardado.
- La esfera cambia a "pensando" mientras responde.
- Los widgets se actualizan en vivo cuando el asistente cambia algo.

**Terminado cuando:** escribo "tengo parcial de Análisis el jueves a las 9" y aparecen el evento y la responsabilidad "Estudiar para Análisis".

## v0.4 — Partidos, noticias y resumen del día
**Objetivo:** que el panel me informe además de organizarme.
- **Fútbol:** integración con la API de fútbol y caché. Boca (próximo partido, último resultado, tabla, modo en vivo), Selección y jugadores seguidos.
- **Noticias:** feeds RSS y filtrado y ordenamiento con IA según mis intereses.
- **Resumen del día:** tarjeta y herramienta `resumen_del_dia`.
- Herramientas `consultar_partidos`, `consultar_tabla`, `consultar_jugador` y `consultar_noticias`.
- Manejo de errores y datos cacheados sin internet.
- (Opcional) Widget de clima chico.

**Terminado cuando:** abro el panel a la mañana y de un vistazo veo mi día, cuándo juega Boca y las noticias que me importan.

## v0.5 — Voz
**Objetivo:** hablarle y que me hable. Acá aparece el efecto "wow".
- Botón y atajo de **pulsar para hablar**.
- Captura de micrófono y **Whisper** local con transcripción en vivo en pantalla.
- **Piper** local para las respuestas, con voz en español.
- **Esfera audio-reactiva:** se deforma con mi voz al escuchar y con la del asistente al hablar. Todos los estados de la sección 3.3.
- Respuestas cortas cuando la entrada fue por voz.
- Lectura del resumen del día al abrir el panel por primera vez en el día (configurable).

**Terminado cuando:** digo "recordame comprar el regalo de Juan para el viernes" y lo crea, me lo confirma por voz y la esfera reacciona todo el tiempo.

## v0.6 — Palabra clave y proactividad
**Objetivo:** que funcione sin tocar el teclado y que me avise solo.
- Detección local de la palabra clave ("Oye Nova") que abre el panel y empieza a escuchar.
- Notificaciones del sistema: eventos próximos, responsabilidades que vencen, partido que arranca.
- Ajustes para activar y desactivar todo esto, y horario de "no molestar".
- Cuidar el consumo de CPU de la escucha en segundo plano.

**Terminado cuando:** uso Nova un día entero solo con la voz y las notificaciones.

## v1.0 — Pulido
**Objetivo:** que se sienta un producto terminado.
- Definir la estética final: tema, tipografía, colores, microanimaciones y sonidos sutiles.
- Arranque con el sistema operativo.
- Instalador para Windows (.msi o .exe).
- Revisión de rendimiento (apertura en menos de 300 ms, 60 fps, consumo en reposo).
- README con capturas y GIFs para mostrarlo.
- (Si hay tiempo) Sincronización con Google Calendar.

## Ideas para después de v1.0
- Widgets reordenables y redimensionables.
- Memoria a largo plazo del asistente (recordar preferencias y datos que le cuento).
- Más herramientas: Spotify, abrir apps, modo foco con temporizador.
- Sistema de plugins para agregar widgets y herramientas nuevas.
- Versión para compartir con otras personas.

---

## Primer mensaje para Claude Code

> Leé `SPEC.md`, `PLAN.md` y `CLAUDE.md`. Vamos a construir Nova siguiendo el plan versión por versión. Antes de escribir código:
> 1. Decime si ves algo ambiguo, contradictorio o riesgoso en la especificación, y qué me recomendás.
> 2. Revisá que tenga instalado todo lo del Paso 0 y decime qué falta.
> 3. Proponeme la estructura de carpetas del proyecto.
>
> Cuando lo aprobemos, arrancamos con **v0.1**: inicializá el repo git, creá la rama `v0.1` y empezá.
