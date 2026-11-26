-- v0.7.1: "avisame en 10 minutos que saque la ropa". Quedan guardados para
-- que un reinicio de Nova no los pierda; se borran apenas avisan.
CREATE TABLE temporizadores (
    id INTEGER PRIMARY KEY,
    texto TEXT NOT NULL,
    vence_en TEXT NOT NULL,           -- ISO 8601 con zona
    creado_en TEXT NOT NULL
);
CREATE INDEX temporizadores_vence ON temporizadores (vence_en);
