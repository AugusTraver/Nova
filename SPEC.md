# Nova — Especificación de la aplicación

> "Nova" es un nombre provisorio. Se cambia cuando quieras (buscar y reemplazar).

## 1. Qué es

Nova es una app de escritorio personal. Con un atajo de teclado se abre un panel a pantalla completa, semitransparente y desenfocado, que se superpone a todo lo demás. En el centro hay un **asistente de IA con voz**, representado por una esfera animada. Alrededor, **widgets** con la información de mi día.

Le puedo hablar o escribir pedidos ("tengo clase a las 3", "recordame comprar el regalo de Juan", "¿cuándo juega mi equipo?"). El asistente los entiende, **ejecuta acciones** (crear eventos, crear o completar responsabilidades, consultar datos) y responde por voz. Los widgets se actualizan en vivo con animaciones.

Objetivos del proyecto, en orden:
1. Que lo use todos los días.
2. Que sea impresionante técnicamente.
3. Que sea impresionante estéticamente.

Sin objetivos comerciales. Es para un solo usuario (yo).

## 2. Plataforma y stack

| Área | Elección | Notas |
|---|---|---|
| App de escritorio | **Tauri 2** (Rust + webview) | Ventana transparente, atajos globales, bandeja del sistema |
| Frontend | **React + TypeScript + Vite** | |
| Estilos | **Tailwind CSS** | |
| Animaciones de interfaz | **Framer Motion** | Entrada y salida del panel, reordenamiento de widgets |
| Esfera central | **Three.js** vía `@react-three/fiber` + shaders | Reacciona al audio |
| Estado del frontend | **Zustand** | |
| Base de datos | **SQLite** local, manejada desde Rust (`rusqlite`) y expuesta con comandos de Tauri tipados | Todos los datos personales quedan en la máquina, en la carpeta de datos de la app |
| Secretos (API keys, tokens) | Llavero del sistema operativo (crate `keyring` o plugin de Tauri equivalente) | **Nunca** en el repo ni en texto plano |
| IA (cerebro) | Capa de proveedores intercambiables: **API con plan gratuito** (por defecto) u **Ollama local** | Tiene que soportar *tool calling* |
| Voz → texto | **Whisper** local (whisper.cpp / `whisper-rs`) | Español |
| Texto → voz | **Piper** local (binario sidecar) con voz en español | |
| Palabra clave ("Oye Nova") | **openWakeWord** o similar, local | Recién en v0.6 |
| Sistema operativo | **Windows 10 Home 22H2** (único objetivo) | Tauri usa WebView2 (ya viene en Windows 10). Desenfoque de fondo con efecto *acrylic* o *blur* (crate `window-vibrancy`). Los binarios sidecar (Piper, whisper.cpp) en versión Windows x64. |

Costo objetivo: **$0**. Todo lo pago se reemplaza por algo local o por un plan gratuito.

## 3. Interfaz

### 3.1 Comportamiento del panel
- **Atajo global** (por defecto `Ctrl+Shift+Space`, configurable) que abre y cierra el panel.
- `Esc` o el mismo atajo lo cierra. Clic fuera de los widgets también.
- La app vive en la **bandeja del sistema** y arranca con el sistema operativo (opcional, en configuración).
- El panel aparece con una animación: fondo que se oscurece con desenfoque, esfera que se expande desde el centro y widgets que entran en cascada.
- Al cerrarse, la app sigue corriendo en segundo plano (para notificaciones y la palabra clave).

### 3.2 Disposición
```
┌─────────────────────────────────────────────────────────┐
│  [Resumen del día]                     [Fútbol]         │
│                                                         │
│  [Calendario]          ( ESFERA )      [Responsabil.]   │
│                     transcripción /                     │
│                       respuesta                         │
│  [Facu]                                [Noticias]       │
│                                                         │
│                  [ barra de texto ]                     │
└─────────────────────────────────────────────────────────┘
```
- La esfera ocupa el centro. Debajo muestra lo que digo (transcripción en vivo) y la respuesta del asistente.
- Abajo hay una barra para escribir. **No hay botón de micrófono:** al abrir el panel, Nova ya está escuchando (ver 5.1). Se puede hablar o escribir indistintamente.
- Los widgets tienen una disposición fija en v1. Arrastrarlos y reordenarlos es una mejora futura.
- Estética a definir más adelante. Base: oscura, glassmorphism (paneles translúcidos), tipografía moderna, un color de acento. Todos los colores en variables o tokens para poder cambiar el tema fácil.

### 3.3 Estados de la esfera
| Estado | Comportamiento visual |
|---|---|
| Reposo | "Respira" lento, colores suaves |
| Escuchando | Se deforma según el volumen del micrófono |
| Pensando | Movimiento interno, gira o pulsa |
| Hablando | Se deforma según el audio de la respuesta |
| Error | Destello breve en un color de alerta |

## 4. Widgets

### 4.1 Calendario
- Muestra hoy y los próximos días con los eventos.
- Tipos de evento: `clase`, `examen`, `entrega`, `cumpleaños`, `partido`, `otro`. Cada uno con su color o ícono.
- Crear, editar y borrar eventos desde el widget o por voz.
- v1: calendario **local** (SQLite). Más adelante: sincronización con Google Calendar.

### 4.2 Responsabilidades
Lista de cosas que tengo que **hacer**, distinta del calendario (que dice lo que **pasa**).
- Cada responsabilidad tiene título, fecha límite opcional, prioridad, estado (pendiente o hecha) y, opcionalmente, un **evento vinculado**.
- Se marca como hecha con un clic o por voz ("ya compré el regalo"), con una animación de tachado.
- Orden: vencidas → hoy → próximas → sin fecha.
- **Responsabilidades automáticas** a partir de eventos (reglas configurables):
  - `examen` → "Estudiar para {materia}" con fecha límite el día anterior.
  - `entrega` → "Terminar {entrega}" con fecha límite el día anterior.
  - `cumpleaños` → "Comprar regalo para {persona}" con fecha límite 3 días antes.
  - Las automáticas llevan una etiqueta "auto". Se pueden editar o borrar como cualquier otra.
  - Siguen a su evento: si el evento cambia, las automáticas pendientes que no tocaste se actualizan; si el evento se borra, cambia de tipo o ya pasó, se borran. Las hechas y las que editaste a mano no se tocan nunca (al borrar el evento quedan, sin evento vinculado). Una automática que borrás no vuelve a aparecer.
  - En los eventos que se repiten (cumpleaños) existe solo la de la próxima repetición.
  - La fecha límite nunca queda antes del día en que se creó (si cargás un cumpleaños para pasado mañana, "comprar regalo" vence hoy).
- Se pueden crear a mano o por voz ("tengo que llamar al dentista").

### 4.3 Facu
- Lista de materias (nombre, color).
- Próximos exámenes y entregas con **cuenta regresiva** (se alimenta de los eventos tipo `examen` y `entrega`).
- Progreso de estudio: cuántas responsabilidades de cada materia están hechas o pendientes.

### 4.4 Fútbol
Solo fútbol. Prioridad: **Boca Juniors** > **Selección Argentina** > **jugadores argentinos** (en Boca y en el exterior).

- **Boca (protagonista del widget):**
  - Próximo partido (rival, fecha, hora local, competencia: Liga Profesional, Copa Argentina, Libertadores, Sudamericana, etc.) y último resultado.
  - Posición en la tabla del torneo local.
  - **Modo en vivo:** cuando juega Boca, el widget se agranda y muestra marcador, minuto y eventos (goles, expulsiones). La esfera puede reaccionar con un efecto especial en los goles de Boca.
- **Selección Argentina:** próximo partido y último resultado. Cuando juega, también entra en modo en vivo.
- **Jugadores:**
  - Plantel de Boca con estadísticas básicas (goles, asistencias).
  - Lista de **argentinos en el exterior** que sigo (configurable, empieza con algunos destacados). Muestra si jugaron en la última fecha y si hicieron goles o asistencias.
- **Herramientas del asistente:** "¿cuándo juega Boca?", "¿cómo salió Boca?", "¿cómo está Boca en la tabla?", "¿hizo un gol Julián?".
- **Proveedor:** una API de fútbol con plan gratuito que cubra la liga argentina, las copas de la Conmebol, la Selección y las ligas europeas (candidata: API-Football). Los planes gratuitos suelen tener un límite diario bajo de consultas, así que hace falta **caché agresiva**:
  - Fixture y tabla: una vez por día.
  - Datos de jugadores: una vez por día, después de la fecha.
  - En vivo: solo mientras juega Boca o la Selección, con un intervalo de consulta calculado para no pasarse del límite.
  - Verificar los límites reales del plan antes de implementar.
- **Noticias:** el widget de noticias incluye Boca y la Selección entre los intereses por defecto.

### 4.5 Noticias
- Fuentes: feeds RSS configurables.
- Intereses configurables en texto libre (por ejemplo: "fútbol argentino, IA, desarrollo web").
- La IA filtra y ordena los titulares según mis intereses y muestra los 5–8 más relevantes con un resumen de una línea.
- Clic → abre la nota en el navegador.

### 4.6 Resumen del día
- Tarjeta con el resumen de hoy: eventos, responsabilidades urgentes, si juega mi equipo y una noticia destacada.
- La primera vez que abro el panel en el día, el asistente puede **leerlo en voz alta** (configurable).
- También se pide por voz: "¿qué tengo hoy?".

### 4.7 Clima (opcional, baja prioridad)
- Temperatura y estado del cielo con Open-Meteo (gratis, sin API key).
- Si se implementa, que sea chico y se pueda ocultar.

### 4.8 Decisiones de v0.4
- **Proveedor de fútbol: la API pública de ESPN** (sin key), detrás del trait `ProveedorFutbol` (`src-tauri/src/futbol/`). Se descartó API-Football porque su plan gratis no cubre la temporada actual. Tope propio de 500 pedidos por día.
- **Caché en SQLite** (migración 3, `cache_externa`): cada dato guarda cuándo vence y su último error. Si falla una descarga, queda el dato viejo y el widget muestra "Sin conexión · datos de hace 3 h". Los reintentos se espacian 5, 15, 30 y 60 minutos.
- **Tarea de fondo** (`fondo.rs`): corre aunque el panel esté cerrado, revisa cada minuto qué venció y avisa con `datos-cambiados`. Los widgets y las herramientas leen solo la caché, nunca la red.
  - Fixture, tabla y jugadores: a las 6 de la mañana, o 2 h 30 después del arranque del próximo partido.
  - En vivo: desde 5 minutos antes del arranque. Consulta cada 30 s con el panel abierto, cada 60 s cerrado y cada 120 s en el entretiempo. Los goles se detectan por diferencia de marcador, y la esfera festeja los de Boca en azul y oro.
- **Jugadores por defecto:** Messi, Tomás Aranda, Paredes, Leonel Flores, Lautaro Blanco, Valentín Barco, Milton Delgado y Neymar. El plantel completo de Boca con estadísticas quedó para más adelante.
- **Noticias:**
  - Los feeds se bajan cada 30 minutos.
  - De hasta 30 titulares recientes, mezclando fuentes, la IA del asistente elige de 5 a 8 y escribe una línea de cada uno. Se le pregunta como mucho cada 2 horas y solo si hay titulares nuevos.
  - La IA copia el título elegido y Nova busca la noticia por ese título, porque el número suele quedar corrido.
  - Sin IA (sin key o con error), se ordena por palabras clave de los intereses y por lo reciente, con hasta 3 notas por fuente.
- **Resumen del día:** una plantilla fija en `dominio/resumen.rs` (sin IA). La usan la herramienta `resumen_del_dia` y la lectura en voz alta de v0.5.
- **Clima:** Open-Meteo cada 30 minutos, al lado de la fecha del saludo. La ciudad se busca en Ajustes y se puede ocultar.

## 5. Asistente

### 5.1 Flujo
1. Entrada: texto o voz, **sin tocar ningún botón**. Al abrir el panel, Nova empieza a escuchar sola: detecta cuándo empezás y terminás de hablar (detección de actividad de voz, local) y procesa el pedido. Si escribís, la escucha se pausa. Desde v0.6 también se abre con la palabra clave ("Oye Nova"). El micrófono solo está activo con el panel abierto (o, desde v0.6, para la palabra clave).
2. Voz → texto con Whisper. La transcripción se muestra en vivo.
3. Se manda al modelo con: instrucciones de sistema, fecha y hora actuales, contexto (eventos y responsabilidades próximas), historial reciente y la lista de herramientas.
4. El modelo responde con texto o con llamadas a herramientas. Se ejecutan, se le devuelve el resultado y continúa hasta dar una respuesta final.
5. La respuesta se muestra y se lee con Piper. Los widgets afectados se actualizan con animación.

### 5.2 Herramientas (tool calling)
| Herramienta | Qué hace |
|---|---|
| `crear_evento` | título, inicio, fin o duración, tipo, materia opcional |
| `editar_evento` / `borrar_evento` | por id |
| `listar_eventos` | por rango de fechas |
| `crear_responsabilidad` | título, fecha límite, prioridad, evento vinculado |
| `completar_responsabilidad` | por id o buscando por texto |
| `listar_responsabilidades` | filtros: pendientes, hoy, vencidas |
| `crear_materia` / `listar_materias` | |
| `consultar_partidos` | próximo, último o en vivo de Boca o la Selección |
| `consultar_tabla` | posición de Boca en el torneo |
| `consultar_jugador` | estadísticas y últimos partidos de un jugador seguido |
| `consultar_noticias` | titulares relevantes |
| `resumen_del_dia` | genera el resumen |

Reglas:
- Fechas relativas ("el jueves", "a las 3", "mañana") se resuelven con la fecha y hora local actuales (zona horaria del sistema).
- Si falta un dato importante, el asistente **pregunta** en lugar de inventarlo.
- Las acciones destructivas (borrar) piden confirmación.
- Responde en español rioplatense, breve y con onda. Nada de párrafos largos por voz.

### 5.3 Capa de proveedores de IA
- Interfaz común (`LLMProvider`) con al menos dos implementaciones: una API con plan gratuito y Ollama local.
- Se elige en configuración. Cambiar de proveedor no debe tocar el resto del código.

### 5.5 Decisiones de v0.3
- **Proveedores:** Groq (plan gratis, por defecto `openai/gpt-oss-120b`) y Ollama local (`qwen3:8b`). Los dos hablan el formato de OpenAI, así que el trait `ProveedorIa` tiene una implementación compatible que sirve para ambos. La API key va en el llavero de Windows (`keyring-core`) y nunca llega al webview.
- **El ciclo corre en Rust** (`src-tauri/src/asistente/`): las herramientas llaman a los mismos `repos::*` que la interfaz. Como mucho son 6 vueltas modelo ↔ herramientas por turno.
- **Sin streaming token a token:** la respuesta llega entera y el progreso ("Anotando el evento…") llega por un `Channel` de Tauri.
- **Fechas:** el modelo copia lo que dijo el usuario ("el jueves", "a las 9") y Rust lo resuelve (`dominio/fechas_relativas.rs`). Los días de la semana van siempre para adelante. Una hora de 1 a 7 sin aclarar es de la tarde, y la hora de fin se lee según la de inicio.
- **Borrar:** `borrar_evento` deja la acción pendiente en Rust. Se confirma con los botones debajo de la esfera o escribiendo "sí", y nunca la confirma el modelo. Cerrar el panel la cancela.
- **Historial:** se mandan los últimos 12 mensajes de las últimas 2 horas, con los resultados de herramientas recortados. Se borra desde Ajustes. La migración 2 agrega `mensajes.datos_json` para guardar las llamadas a herramientas.
- **Widgets en vivo:** Rust emite `datos-cambiados` con las fuentes afectadas apenas termina cada herramienta.

### 5.6 Decisiones de v0.5 (voz)
- **Todo local, como programas aparte:** `whisper-server.exe` (whisper.cpp) y `piper.exe` quedan abiertos mientras se usan y Nova les habla por la red local (Whisper) y por la entrada estándar (Piper). No hace falta compilar C++ ni tener CMake.
  - Al salir de Nova se cierran.
  - Si Nova se cae, se cierran al volver a abrirla (se anota su número de proceso).
- **Modelos:** Whisper `large-v3-turbo` cuantizado y la voz `es_AR-daniela-high` de Piper.
  - En la placa de video (CUDA), Whisper transcribe un pedido en unos 170 ms. Si CUDA no arranca, se usa la versión de CPU.
  - Se bajan desde Ajustes (unos 1,4 GB, una vez) a `%LOCALAPPDATA%\com.augustraver.nova\voz`, nunca al repo.
- **Escucha:** el micrófono (cpal) se abre al abrir el panel y se suelta al cerrarlo.
  - La detección de voz es por energía, con un piso de ruido que se adapta: el ruido es lo más bajo de los últimos 5 segundos.
  - Arranca después de 120 ms de voz y termina después de 900 ms de silencio. Agrega los 300 ms de antes para no comerse la primera sílaba.
  - Antes de bajar de 48 a 16 kHz pasa un filtro pasabajos, para que no haya aliasing.
- **Transcripción en vivo:** con la placa de video, lo que vas diciendo se transcribe cada 0,7 s y se ve debajo de la esfera. Las frases que Whisper inventa sobre el ruido ("Subtítulos realizados por…", "Gracias por ver") se descartan.
- **Medio dúplex:** mientras Nova habla y hasta 350 ms después, el micrófono se ignora, para que no se escuche a sí misma. `Esc` la calla.
- **Respuestas:**
  - Lo que dijiste hablando se manda al asistente con `por_voz`, que pide una sola oración corta, sin símbolos y con las horas dichas.
  - Esas respuestas, y la pregunta de "¿Borro…?", se leen con Piper, frase por frase, así empieza a hablar antes de tener todo sintetizado.
  - Lo que escribís se contesta sin voz, y escribir pausa la escucha.
- **Resumen al abrir:** la primera vez en el día, si está activado, se lee el texto de `dominio/resumen.rs` sin pasar por la IA.
- **Esfera:** con la voz, la esfera recibe el volumen unas 30 veces por segundo.
  - Escuchando, se forman ondas finas y se aclara el borde.
  - Hablando, laten pulsos más grandes y se infla un poco con cada sílaba.

### 5.4 Proactividad (v0.6+)
- Notificaciones del sistema: "En 30 minutos tenés clase", "Vence hoy: estudiar para Análisis", "Arranca el partido".
- Configurables y silenciables.

## 6. Modelo de datos (SQLite)

```
materias(id, nombre, color, creado_en)

eventos(id, titulo, tipo, inicio, fin, todo_el_dia, materia_id?, persona?, notas,
        recurrencia['ninguna'|'semanal'|'anual'], repetir_hasta?,
        origen['local'|'google'], id_externo?, creado_en, actualizado_en)

eventos_excepciones(evento_id, fecha)   -- fechas salteadas de una serie (feriados)

responsabilidades(id, titulo, descripcion?, fecha_limite?, prioridad['baja'|'media'|'alta'],
                  estado['pendiente'|'hecha'], hecha_en?, evento_id?, materia_id?,
                  origen['manual'|'auto'|'asistente'],
                  regla?, evento_fecha?, editada_a_mano, descartada,  -- solo automáticas
                  creado_en, actualizado_en)

mensajes(id, rol['usuario'|'asistente'|'herramienta'], contenido, creado_en)

preferencias(clave, valor_json)
  -- atajo, proveedor_ia, modelo, equipos (Boca, Selección), jugadores_seguidos,
  -- intereses_noticias, feeds_rss,
  -- reglas_auto_responsabilidades, leer_resumen_al_abrir, ciudad_clima, tema
```
- Fechas en ISO 8601 con zona horaria. Los eventos de todo el día y las fechas límite se guardan como fecha sola (`2026-11-20`); el `fin` de un evento de todo el día es el último día incluido.
- Los eventos recurrentes se guardan como serie y las repeticiones se calculan al pedir un rango. Editar una repetición cambia toda la serie; para una fecha suelta existe "saltear".
- Los colores de las materias son nombres de una paleta de tokens (`celeste`, `coral`, `menta`, `lavanda`, `ambar`, `rosa`), no hex.
- Migraciones versionadas desde el primer día (`PRAGMA user_version`, archivos en `src-tauri/migraciones/`).

## 7. Configuración
Pantalla de ajustes (dentro del panel) para: atajo, proveedor y modelo de IA, API keys (guardadas en el llavero), equipos, intereses y feeds de noticias, reglas de responsabilidades automáticas, voz (activar/desactivar, velocidad), arranque con el sistema, clima on/off.

## 8. Requisitos no funcionales
- **Rápido:** el panel tiene que abrirse en menos de 300 ms desde el atajo. Animaciones a 60 fps.
- **Liviano en reposo:** poco CPU cuando el panel está cerrado.
- **Privado:** datos personales y audio se quedan en la máquina. Solo salen las consultas a la IA (si se usa una API) y a las APIs de datos.
- **Resiliente:** si una API falla o no hay internet, el widget muestra su último dato cacheado y un aviso, sin romper el panel.
- **Seguro:** ninguna clave en el código ni en el repo. `.env` y similares en `.gitignore`.

## 9. Fuera del alcance (por ahora)
- Multiusuario o cuentas.
- Mobile.
- Hardware externo.
- Widgets reordenables (mejora futura).
- Sincronización con Google Calendar (después de v1.0 o en v1.0 si hay tiempo).
