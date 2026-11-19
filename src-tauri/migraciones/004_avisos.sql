-- v0.6: avisos del sistema ya mandados, para no repetirlos.
-- La clave dice qué fue: 'evento:12:2026-11-18T14:00:00-03:00', 'vence:2026-11-18',
-- 'partido:401', 'salir:2026-11-18'.
CREATE TABLE avisos_enviados (
    clave TEXT PRIMARY KEY,
    enviado_en TEXT NOT NULL          -- ISO 8601; los de hace más de una semana se borran
);
