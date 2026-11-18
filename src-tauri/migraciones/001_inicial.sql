-- Esquema inicial (SPEC sección 6, con los agregados decididos para v0.2:
-- recurrencia de eventos, excepciones y datos para las responsabilidades
-- automáticas).
--
-- Fechas:
--   * Eventos con hora: ISO 8601 con zona horaria ("2026-11-20T09:00:00-03:00").
--   * Eventos de todo el día y fechas límite: fecha sola ("2026-11-20").
--   * Marcas de tiempo (creado_en, etc.): ISO 8601 con zona horaria.

CREATE TABLE materias (
    id         INTEGER PRIMARY KEY,
    nombre     TEXT    NOT NULL,
    -- Nombre de un color de la paleta de tokens.css, no un hex.
    color      TEXT    NOT NULL,
    creado_en  TEXT    NOT NULL
);

CREATE TABLE eventos (
    id              INTEGER PRIMARY KEY,
    titulo          TEXT    NOT NULL,
    tipo            TEXT    NOT NULL CHECK (tipo IN ('clase','examen','entrega','cumpleanos','partido','otro')),
    inicio          TEXT    NOT NULL,
    fin             TEXT,
    todo_el_dia     INTEGER NOT NULL DEFAULT 0,
    materia_id      INTEGER REFERENCES materias(id) ON DELETE SET NULL,
    persona         TEXT,
    notas           TEXT,
    recurrencia     TEXT    NOT NULL DEFAULT 'ninguna' CHECK (recurrencia IN ('ninguna','semanal','anual')),
    -- Última fecha (incluida) en la que puede caer una repetición.
    repetir_hasta   TEXT,
    origen          TEXT    NOT NULL DEFAULT 'local' CHECK (origen IN ('local','google')),
    id_externo      TEXT,
    creado_en       TEXT    NOT NULL,
    actualizado_en  TEXT    NOT NULL
);

CREATE INDEX eventos_inicio ON eventos(inicio);

-- Fechas en las que un evento recurrente NO ocurre (feriados, clases suspendidas).
CREATE TABLE eventos_excepciones (
    evento_id  INTEGER NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
    fecha      TEXT    NOT NULL,
    PRIMARY KEY (evento_id, fecha)
);

CREATE TABLE responsabilidades (
    id              INTEGER PRIMARY KEY,
    titulo          TEXT    NOT NULL,
    descripcion     TEXT,
    fecha_limite    TEXT,
    prioridad       TEXT    NOT NULL DEFAULT 'media' CHECK (prioridad IN ('baja','media','alta')),
    estado          TEXT    NOT NULL DEFAULT 'pendiente' CHECK (estado IN ('pendiente','hecha')),
    hecha_en        TEXT,
    evento_id       INTEGER REFERENCES eventos(id) ON DELETE SET NULL,
    materia_id      INTEGER REFERENCES materias(id) ON DELETE SET NULL,
    origen          TEXT    NOT NULL DEFAULT 'manual' CHECK (origen IN ('manual','auto','asistente')),
    -- Solo para las automáticas: qué regla la creó y para qué ocurrencia del evento.
    regla           TEXT    CHECK (regla IN ('examen','entrega','cumpleanos')),
    evento_fecha    TEXT,
    -- Si la tocaste a mano, la sincronización ya no la cambia ni la borra.
    editada_a_mano  INTEGER NOT NULL DEFAULT 0,
    -- Automática que borraste: queda oculta para que no vuelva a aparecer.
    descartada      INTEGER NOT NULL DEFAULT 0,
    creado_en       TEXT    NOT NULL,
    actualizado_en  TEXT    NOT NULL
);

CREATE INDEX responsabilidades_evento ON responsabilidades(evento_id);

CREATE TABLE mensajes (
    id         INTEGER PRIMARY KEY,
    rol        TEXT    NOT NULL CHECK (rol IN ('usuario','asistente','herramienta')),
    contenido  TEXT    NOT NULL,
    creado_en  TEXT    NOT NULL
);

CREATE TABLE preferencias (
    clave       TEXT PRIMARY KEY,
    valor_json  TEXT NOT NULL
);
