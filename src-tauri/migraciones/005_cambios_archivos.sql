-- v0.7.1: cambios que Nova hizo en archivos, para poder deshacerlos.
-- La copia de antes va en %LOCALAPPDATA%\com.augustraver.nova\deshacer\
-- (nunca en el repo). Se borran a los 7 días, y quedan como mucho 50.
CREATE TABLE cambios_archivos (
    id INTEGER PRIMARY KEY,
    ruta TEXT NOT NULL,               -- el archivo que se tocó (ruta completa)
    accion TEXT NOT NULL CHECK (accion IN ('cambiar', 'crear', 'borrar', 'deshacer')),
    copia TEXT,                       -- la copia de antes; NULL si el archivo no existía (crear)
    huella_despues TEXT,              -- cómo quedó, para saber si lo tocaste después; NULL si se borró
    hecho_en TEXT NOT NULL,           -- ISO 8601
    deshecho INTEGER NOT NULL DEFAULT 0
);
