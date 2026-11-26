-- v0.7.2: sincronización con Google Calendar en los dos sentidos.
--
-- `origen` e `id_externo` ya venían de la migración 1. Acá se suma lo que
-- hace falta para saber qué cambió de cada lado y qué falta subir.

-- Lo que dijo Google la última vez que lo vimos: su `etag` (para mandar
-- `If-Match` y que Google avise si alguien lo cambió en el medio) y su
-- `updated` (para decidir quién gana si cambió de los dos lados).
ALTER TABLE eventos ADD COLUMN etag_google TEXT;
ALTER TABLE eventos ADD COLUMN actualizado_google TEXT;

-- 1 = cambió en Nova y falta subirlo a Google. Los eventos que ya existían
-- quedan en 1: la primera vez que conectes Google, se suben.
ALTER TABLE eventos ADD COLUMN cambiado_local INTEGER NOT NULL DEFAULT 1;

-- Repeticiones de una serie de Google que Nova no sabe repetir (diaria,
-- mensual, "lunes y miércoles"…): se guardan sueltas, se editan desde
-- Calendar y `serie_externa` es el id de la serie en Google.
ALTER TABLE eventos ADD COLUMN solo_lectura INTEGER NOT NULL DEFAULT 0;
ALTER TABLE eventos ADD COLUMN serie_externa TEXT;

-- Una repetición movida de una serie que Nova sí sabe repetir: en la serie
-- queda salteada esa fecha y este evento suelto la reemplaza.
ALTER TABLE eventos ADD COLUMN serie_id INTEGER REFERENCES eventos(id) ON DELETE CASCADE;
ALTER TABLE eventos ADD COLUMN serie_fecha TEXT;

CREATE UNIQUE INDEX eventos_id_externo ON eventos(id_externo) WHERE id_externo IS NOT NULL;
CREATE INDEX eventos_pendientes ON eventos(cambiado_local) WHERE cambiado_local = 1;

-- Lo que borraste en Nova y todavía no se pudo borrar en Google (sin
-- internet). Se va de acá apenas Google lo confirma.
CREATE TABLE google_borrados (
    id_externo  TEXT PRIMARY KEY,
    borrado_en  TEXT NOT NULL
);
