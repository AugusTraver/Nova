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
| Base de datos | **SQLite** local (plugin SQL de Tauri) | Todos los datos personales quedan en la máquina |
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

### 5.4 Proactividad (v0.6+)
- Notificaciones del sistema: "En 30 minutos tenés clase", "Vence hoy: estudiar para Análisis", "Arranca el partido".
- Configurables y silenciables.

## 6. Modelo de datos (SQLite)

```
materias(id, nombre, color, creado_en)

eventos(id, titulo, tipo, inicio, fin, todo_el_dia, materia_id?, persona?, notas,
        origen['local'|'google'], id_externo?, creado_en, actualizado_en)

responsabilidades(id, titulo, descripcion?, fecha_limite?, prioridad['baja'|'media'|'alta'],
                  estado['pendiente'|'hecha'], hecha_en?, evento_id?, materia_id?,
                  origen['manual'|'auto'|'asistente'], creado_en, actualizado_en)

mensajes(id, rol['usuario'|'asistente'|'herramienta'], contenido, creado_en)

preferencias(clave, valor_json)
  -- atajo, proveedor_ia, modelo, equipos (Boca, Selección), jugadores_seguidos,
  -- intereses_noticias, feeds_rss,
  -- reglas_auto_responsabilidades, leer_resumen_al_abrir, ciudad_clima, tema
```
- Fechas en ISO 8601 con zona horaria.
- Migraciones versionadas desde el primer día.

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
