-- v0.3: el historial del asistente guarda también las llamadas a herramientas.
-- En un mensaje del asistente: la lista de llamadas que pidió el modelo.
-- En uno de herramienta: a qué llamada responde y de qué herramienta es.
ALTER TABLE mensajes ADD COLUMN datos_json TEXT;
CREATE INDEX mensajes_por_fecha ON mensajes (creado_en);
