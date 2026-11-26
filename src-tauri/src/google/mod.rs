//! Google Calendar (v0.7.2): el permiso (`oauth`), la API (`calendario`) y
//! la sincronización en los dos sentidos (`sincronizar`). La traducción de
//! eventos y las reglas puras están en `dominio::google`.

pub mod calendario;
pub mod oauth;
pub mod sincronizar;
