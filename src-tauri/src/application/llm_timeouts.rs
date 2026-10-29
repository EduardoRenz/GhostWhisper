//! Prazos das chamadas de IA (BR-045).
//!
//! Sugestão, pergunta e validação são **latência crítica**: durante a reunião
//! o usuário precisa da resposta agora. Um modelo pendurado não pode segurar a
//! chamada até o teto longo — o prazo curto faz a falha chegar rápido ao
//! circuit breaker, que já entrega a vez à próxima fonte.
//!
//! Resumo e tradução montam textos maiores e não bloqueiam a conversa, então
//! mantêm o teto maior (o mesmo que os adaptadores usam por padrão).
//!
//! Os valores são constantes de política: ajustar aqui muda todos os caminhos.

use std::time::Duration;

use crate::domain::ai_route::{AiCapability, AiRouteScope};

/// Prazo do caminho rápido (sugestões, pergunta e validação).
pub const FAST_TIMEOUT: Duration = Duration::from_secs(15);

/// Prazo das chamadas longas (resumo e tradução).
pub const LONG_TIMEOUT: Duration = Duration::from_secs(60);

/// Prazo aplicado a um escopo de rota.
pub fn timeout_for_scope(scope: &AiRouteScope) -> Duration {
    match scope {
        AiRouteScope::Capability(AiCapability::Summary | AiCapability::Translation) => LONG_TIMEOUT,
        _ => FAST_TIMEOUT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_paths_get_the_short_timeout() {
        assert_eq!(timeout_for_scope(&AiRouteScope::Default), FAST_TIMEOUT);
        assert_eq!(
            timeout_for_scope(&AiRouteScope::Agent("meeting".into())),
            FAST_TIMEOUT
        );
        assert_eq!(
            timeout_for_scope(&AiRouteScope::Capability(AiCapability::Suggestions)),
            FAST_TIMEOUT
        );
    }

    #[test]
    fn long_calls_keep_the_long_timeout() {
        assert_eq!(
            timeout_for_scope(&AiRouteScope::Capability(AiCapability::Summary)),
            LONG_TIMEOUT
        );
        assert_eq!(
            timeout_for_scope(&AiRouteScope::Capability(AiCapability::Translation)),
            LONG_TIMEOUT
        );
    }

    #[test]
    fn fast_timeout_is_short_enough_for_failover() {
        // A exigência é resposta rápida: o caminho rápido não pode esperar o
        // teto longo do resumo.
        assert!(FAST_TIMEOUT < LONG_TIMEOUT);
        assert!(FAST_TIMEOUT <= Duration::from_secs(15));
    }
}
