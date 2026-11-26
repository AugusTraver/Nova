//! El principal y su respaldo (v0.7.3, SPEC 5.10).
//!
//! Groq es rápido pero tiene poco cupo (8.000 tokens por minuto y 200.000 por
//! día en el plan gratis). La cadena le manda cada pedido a Groq si entra y,
//! si no entra o Groq dice que se pasó, se lo manda al respaldo (Gemini). Al
//! minuto siguiente vuelve a Groq solo, porque el cupo se renueva.
//!
//! Qué proveedor usar es una decisión pura (`elegir` y `pasar_al_respaldo`,
//! con tests); lo que se sabe del cupo de cada uno está en `cupo.rs`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde_json::Value;

use super::cupo::{self, Cupo};
use super::mensajes::{Mensaje, RespuestaModelo};
use super::openai::{ClienteCompatible, armar_pedido, estimar_tokens};
use super::proveedor::{ProveedorIa, TipoProveedor};
use crate::error::{Error, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eleccion {
    Principal,
    Respaldo,
}

/// ¿A quién se le manda un pedido de `estimado` tokens? Al principal si le
/// entra (o si no hay otro); si no, al respaldo, salvo que tampoco le entre.
pub fn elegir(principal: Cupo, respaldo: Option<Cupo>, estimado: u64, ahora: Instant) -> Eleccion {
    match respaldo {
        Some(r) if !principal.alcanza(estimado, ahora) && r.alcanza(estimado, ahora) => Eleccion::Respaldo,
        _ => Eleccion::Principal,
    }
}

/// El principal falló: ¿se prueba con el respaldo? Solo si el error fue de
/// cupo o del servidor (quedó en pausa), no si la key está mal o el pedido
/// era inválido: ahí el respaldo fallaría igual o escondería el problema.
pub fn pasar_al_respaldo(principal_despues: Cupo, hay_respaldo: bool, ahora: Instant) -> bool {
    hay_respaldo && principal_despues.en_pausa(ahora)
}

pub struct CadenaIa {
    principal: ClienteCompatible,
    respaldo: Option<ClienteCompatible>,
    /// Si en este turno ya se pasó al respaldo, se queda ahí hasta el final
    /// (no se mezclan proveedores en medio de un pedido con herramientas).
    en_respaldo: AtomicBool,
}

impl CadenaIa {
    pub fn nueva(principal: ClienteCompatible, respaldo: Option<ClienteCompatible>) -> Self {
        CadenaIa { principal, respaldo, en_respaldo: AtomicBool::new(false) }
    }

    #[cfg(test)]
    pub fn tipos(&self) -> (TipoProveedor, Option<TipoProveedor>) {
        (self.principal.tipo(), self.respaldo.as_ref().map(ClienteCompatible::tipo))
    }

    /// El que toca para un pedido de `estimado` tokens.
    fn cual(&self, estimado: u64) -> &ClienteCompatible {
        let Some(respaldo) = &self.respaldo else { return &self.principal };
        if self.en_respaldo.load(Ordering::Relaxed) {
            return respaldo;
        }
        let eleccion = elegir(cupo::de(self.principal.tipo()), Some(cupo::de(respaldo.tipo())), estimado, Instant::now());
        if eleccion == Eleccion::Respaldo {
            self.avisar_cambio(respaldo, "no le alcanza el cupo");
            return respaldo;
        }
        &self.principal
    }

    fn avisar_cambio(&self, respaldo: &ClienteCompatible, por_que: &str) {
        self.en_respaldo.store(true, Ordering::Relaxed);
        eprintln!("IA: a {} {por_que}; uso {}.", self.principal.tipo().nombre(), respaldo.tipo().nombre());
    }

    /// Si `resultado` es un error de cupo del principal, prueba con el
    /// respaldo. `otra_vez` repite el pedido con el cliente que recibe.
    async fn con_respaldo<'a, T, F, Fut>(&'a self, usado: &'a ClienteCompatible, resultado: Resultado<T>, otra_vez: F) -> Resultado<T>
    where
        F: FnOnce(&'a ClienteCompatible) -> Fut,
        Fut: std::future::Future<Output = Resultado<T>>,
    {
        match (resultado, &self.respaldo) {
            (Err(Error::Sistema(_)), Some(respaldo))
                if std::ptr::eq(usado, &self.principal)
                    && pasar_al_respaldo(cupo::de(self.principal.tipo()), true, Instant::now()) =>
            {
                self.avisar_cambio(respaldo, "se le acabó el cupo");
                otra_vez(respaldo).await
            }
            (otro, _) => otro,
        }
    }

    /// Un pedido sin herramientas (tareas internas). `de_fondo`: lo de la
    /// tarea de fondo (elegir noticias) va primero al respaldo, así no le
    /// come cupo a lo que le pedís a Nova.
    pub async fn texto(&self, mensajes: &[Mensaje], de_fondo: bool) -> Resultado<String> {
        let estimado = estimar_tokens(&armar_pedido("", mensajes, None));
        let cliente = match (&self.respaldo, de_fondo) {
            (Some(respaldo), true) if cupo::de(respaldo.tipo()).alcanza(estimado, Instant::now()) => respaldo,
            _ => self.cual(estimado),
        };
        let resultado = cliente.texto(mensajes).await;
        self.con_respaldo(cliente, resultado, |c| c.texto(mensajes)).await
    }

    /// "Probar conexión": prueba los dos y dice cuál falló.
    pub async fn probar(&self) -> Resultado<()> {
        self.principal.probar().await?;
        if let Some(respaldo) = &self.respaldo {
            respaldo.probar().await.map_err(|e| {
                Error::Validacion(format!("{} anda, pero el respaldo no: {e}", self.principal.tipo().nombre()))
            })?;
        }
        Ok(())
    }
}

impl ProveedorIa for CadenaIa {
    async fn completar(&self, mensajes: &[Mensaje], herramientas: &Value) -> Resultado<RespuestaModelo> {
        let con = herramientas.as_array().is_some_and(|h| !h.is_empty()).then_some(herramientas);
        let estimado = estimar_tokens(&armar_pedido("", mensajes, con));
        let cliente = self.cual(estimado);
        let resultado = cliente.completar(mensajes, herramientas).await;
        self.con_respaldo(cliente, resultado, |c| c.completar(mensajes, herramientas)).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn elige_al_principal_mientras_le_alcanza() {
        let ahora = Instant::now();
        let en = |s| ahora + Duration::from_secs(s);
        let libre = Cupo::default();
        let groq_justo = Cupo { quedan: Some((1800, en(25))), ..Default::default() };
        let groq_pausado = Cupo { pausado_hasta: Some(en(40)), ..Default::default() };

        assert_eq!(elegir(libre, Some(libre), 2600, ahora), Eleccion::Principal);
        assert_eq!(elegir(groq_justo, Some(libre), 2600, ahora), Eleccion::Respaldo);
        assert_eq!(elegir(groq_justo, Some(libre), 1500, ahora), Eleccion::Principal, "un pedido chico entra");
        assert_eq!(elegir(groq_justo, Some(libre), 2600, en(26)), Eleccion::Principal, "se renovó el minuto");
        assert_eq!(elegir(groq_pausado, Some(libre), 100, ahora), Eleccion::Respaldo);
        // Sin respaldo, o con el respaldo también sin cupo, se intenta igual
        // con el principal (que va a explicar el límite).
        assert_eq!(elegir(groq_pausado, None, 100, ahora), Eleccion::Principal);
        assert_eq!(elegir(groq_pausado, Some(groq_pausado), 100, ahora), Eleccion::Principal);
    }

    #[test]
    fn pasa_al_respaldo_solo_por_cupo() {
        let ahora = Instant::now();
        let pausado = Cupo { pausado_hasta: Some(ahora + Duration::from_secs(9)), ..Default::default() };
        assert!(pasar_al_respaldo(pausado, true, ahora));
        assert!(!pasar_al_respaldo(pausado, false, ahora), "no hay a quién");
        // Una key mala no pausa al proveedor: el error se muestra tal cual.
        assert!(!pasar_al_respaldo(Cupo::default(), true, ahora));
    }
}
