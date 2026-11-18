-- v0.4: caché de datos de internet (fútbol, noticias, clima).
-- Cada fila es un pedido: guarda el último dato bueno aunque después falle,
-- así el widget lo sigue mostrando con un aviso cuando no hay internet.
CREATE TABLE cache_externa (
    clave TEXT PRIMARY KEY,           -- ej. 'futbol:equipo:5', 'noticias:feed:https://...'
    valor_json TEXT,                  -- último dato bueno; NULL si nunca se pudo obtener
    obtenido_en TEXT,                 -- cuándo se obtuvo ese dato (ISO 8601)
    vence_en TEXT NOT NULL,           -- desde cuándo hay que volver a pedirlo
    error TEXT,                       -- el error del último intento, NULL si salió bien
    fallas_seguidas INTEGER NOT NULL DEFAULT 0
);

-- Cuántos pedidos se hicieron por día y a qué servicio (tope propio).
CREATE TABLE pedidos_por_dia (
    servicio TEXT NOT NULL,
    fecha TEXT NOT NULL,              -- AAAA-MM-DD local
    cantidad INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (servicio, fecha)
);
