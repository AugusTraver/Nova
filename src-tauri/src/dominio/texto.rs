//! Enums que se guardan como texto en SQLite y viajan como texto al frontend.

/// Macro para no repetir lo mismo en cada enum. Una macro de Rust
/// (`macro_rules!`) es una plantilla de código: por cada enum que le pasamos,
/// escribe el `enum`, su conversión a texto (`como_texto`) y desde texto
/// (`desde_texto`). Con `serde(rename_all = "snake_case")` el JSON usa los
/// mismos textos que la base ("cumpleanos", "pendiente", etc.).
macro_rules! enum_texto {
    ($nombre:ident { $($variante:ident => $texto:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $nombre { $($variante),+ }

        impl $nombre {
            pub fn como_texto(self) -> &'static str {
                match self { $(Self::$variante => $texto),+ }
            }

            pub fn desde_texto(texto: &str) -> Option<Self> {
                match texto { $($texto => Some(Self::$variante),)+ _ => None }
            }
        }

        // Para poder pasar el enum directo como parámetro de una consulta SQL.
        impl rusqlite::ToSql for $nombre {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                Ok(self.como_texto().into())
            }
        }

        // Para poder leerlo directo de una fila (`fila.get::<_, TipoEvento>(2)`).
        impl rusqlite::types::FromSql for $nombre {
            fn column_result(valor: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                let texto = valor.as_str()?;
                Self::desde_texto(texto).ok_or_else(|| {
                    rusqlite::types::FromSqlError::Other(format!("valor inválido: {texto}").into())
                })
            }
        }
    };
}

enum_texto!(TipoEvento {
    Clase => "clase",
    Examen => "examen",
    Entrega => "entrega",
    Cumpleanos => "cumpleanos",
    Partido => "partido",
    Otro => "otro",
});

enum_texto!(Recurrencia {
    Ninguna => "ninguna",
    Semanal => "semanal",
    Anual => "anual",
});

enum_texto!(Prioridad {
    Baja => "baja",
    Media => "media",
    Alta => "alta",
});

enum_texto!(EstadoResponsabilidad {
    Pendiente => "pendiente",
    Hecha => "hecha",
});

enum_texto!(OrigenResponsabilidad {
    Manual => "manual",
    Auto => "auto",
    Asistente => "asistente",
});

enum_texto!(TipoRegla {
    Examen => "examen",
    Entrega => "entrega",
    Cumpleanos => "cumpleanos",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_con_texto_y_json() {
        assert_eq!(TipoEvento::Cumpleanos.como_texto(), "cumpleanos");
        assert_eq!(TipoEvento::desde_texto("examen"), Some(TipoEvento::Examen));
        assert_eq!(TipoEvento::desde_texto("fiesta"), None);
        assert_eq!(serde_json::to_string(&Prioridad::Alta).unwrap(), "\"alta\"");
        let leido: Recurrencia = serde_json::from_str("\"semanal\"").unwrap();
        assert_eq!(leido, Recurrencia::Semanal);
    }
}
