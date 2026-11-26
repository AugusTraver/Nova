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
- **Escucha automática al abrir el panel**, sin botón: detección de actividad de voz (VAD) local para saber cuándo empieza y termina el pedido. Escribir pausa la escucha.
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

## v0.6.1 — Gastar menos del plan gratis
**Objetivo:** que Groq no se quede sin cuota a los 3 o 4 mensajes.
- Medir cuántos tokens gasta cada pedido.
- Mandar solo las herramientas del tema del pedido.
- No repetir en el historial las llamadas a herramientas viejas.
- Arreglar las llamadas rechazadas por campos en `null`.
- Avisar claro qué límite se alcanzó y cuánto esperar.

**Terminado cuando:** un pedido típico gasta bastante menos que antes, medido con Groq de verdad, y los errores "armó mal un pedido" dejan de aparecer.

## v0.7.1 — Nova usa la compu
**Objetivo:** que Nova haga cosas en la compu cuando se lo pido: abrir programas, páginas, videos y archivos.
- Abrir páginas en el navegador: una dirección ("abrí la página de la facultad") o una pestaña con una búsqueda de Google ("buscame en Google recetas de pizza").
- YouTube: abrir un video ("poneme el primer video que sale cuando buscás Riquelme", "un video de lo-fi").
- Abrir programas instalados (Visual Studio Code, Spotify, la calculadora).
- Abrir carpetas, leer archivos y modificarlos con confirmación, mostrando el antes y después, con opción de deshacer.
- Solo dentro de las carpetas permitidas en Ajustes, y con un interruptor para apagarlo todo.
- Extras a elegir: controlar la música, temporizadores, portapapeles, ventanas, horas ambiguas.

Acá Nova solo **abre** cosas: no lee la página ni contesta con lo que dice. Eso es v0.7.3.

**Terminado cuando:** por voz abro el primer video de YouTube de una búsqueda, una pestaña de Google con lo que pedí y Visual Studio Code; cambio una palabra de un archivo confirmándolo y lo deshago; Nova no toca nada fuera de las carpetas permitidas, y un pedido típico no gasta más tokens que en v0.6.1.

## v0.7.2 — Google Calendar
**Objetivo:** que la agenda de Nova y mi Google Calendar sean la misma.
- Sincronización en los dos sentidos: lo que agendo en Nova (a mano o con el asistente) aparece en Calendar, y lo que agendo en Calendar aparece en Nova. Editar o borrar de un lado se refleja en el otro, sin duplicados.
- Las responsabilidades automáticas salen de los eventos, vengan de donde vengan.
- Sin internet, Nova funciona igual con lo local y sincroniza cuando vuelve.
- El permiso de Google (OAuth) guardado en el llavero. Conectar y desconectar desde Ajustes (desconectar no borra eventos).
- Cuando llega algo de Calendar, el widget se actualiza solo; si Google falla, muestra un aviso chico sin romper el panel.
- Las herramientas del asistente no cambian y no suman tokens.

**Terminado cuando:** un evento creado en Nova aparece en Google Calendar, uno creado en Calendar aparece en Nova con sus responsabilidades automáticas, y editar o borrar de un lado se ve en el otro.

## v0.7.3 — Buscar información en internet
**Objetivo:** que Nova conteste cosas que no sabe buscándolas, sin abrir el navegador.
- "Fijate quién ganó anoche River", "¿a qué hora abre tal lugar?": busca con una API de búsqueda aparte, con plan gratis (no la búsqueda incluida de Groq, para no gastar sus tokens), lee unos pocos resultados y contesta en una o dos frases, diciendo de dónde lo sacó.
- Nova decide sola cuándo buscar: si la respuesta está en sus datos (agenda, fútbol, noticias) o es algo que no cambia, no busca; si es algo actual o que no sabe, busca.
- Si no encuentra algo confiable o se acaba el plan gratis, lo dice y ofrece abrir la búsqueda en el navegador (lo de v0.7.1).

**Terminado cuando:** le pregunto por voz algo actual que no sabe y me contesta buscando, diciendo de dónde lo sacó, sin pasarse del límite por minuto de Groq.

## v0.8 — Instalable y de todos los días
**Objetivo:** que Nova se instale y aguante el uso diario sin abrir la terminal.
- Instalador para Windows, que conserve los datos y las keys al actualizar.
- Arranque con el sistema operativo, oculto en la bandeja (configurable).
- El modelo de "Oye Nova" llega solo con una instalación nueva.
- Notificaciones con el nombre "Nova".
- Revisión de rendimiento en la versión instalada (apertura en menos de 300 ms, 60 fps, consumo en reposo).
- Pendientes chicos de versiones anteriores.

**Terminado cuando:** instalo Nova, reinicio Windows, arranca sola, "Oye Nova" la abre, me llega un aviso que dice "Nova" y la uso una semana sin abrir la terminal.

## v1.0 — Pulido y conexiones
**Objetivo:** que se sienta un producto terminado y que Nova esté conectada a mis cosas.
- Definir la estética final (partiendo de "Atmósfera"): microanimaciones y sonidos sutiles.
- README con capturas y GIFs para mostrarlo.
- Conexiones (evaluar MCP, Model Context Protocol, como forma común de conectarlas en vez de programar cada una a mano; cualquier acción que mande o cambie algo afuera, siempre con confirmación):
  - **Celular:** hablarle a Nova desde el teléfono (agendar, preguntar, "¿qué tengo hoy?") y que los avisos lleguen también ahí. Cómo (un bot de Telegram, una app web o algo propio) se decide al empezar la versión.
  - **Gmail:** "¿me llegó algo de la facu?", los mails importantes en el resumen del día, mandar un mail.
  - **Spotify:** "poné algo para estudiar", "¿qué está sonando?", pasar de tema, elegir playlists.
  - **Google Drive:** buscar y abrir mis apuntes y documentos ("abrí el resumen de Análisis").
  - **GitHub:** cómo vienen mis repos, avisos de issues y pull requests.
  - **Discord:** avisos de mis servidores y mensajes directos.
  - **WhatsApp (opcional):** no tiene API oficial para cuentas personales; entra solo si aparece una forma segura de hacerlo.
- (A evaluar) Actualizaciones automáticas.

**Terminado cuando:** desde el celular le pido algo a Nova y lo hace, me resume los mails del día, pone música en Spotify, encuentra un apunte en Drive y me avisa de algo de GitHub o Discord.

## Ideas para después de v1.0
- Widgets redimensionables (arrastrarlos ya está, desde v0.7.1).
- Memoria a largo plazo del asistente (recordar preferencias y datos que le cuento).
- Más herramientas: modo foco con temporizador.
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
