//! Rotas de IA por escopo (BR-048).
//!
//! Uma rota **vincula** uma fonte de IA a um escopo: um agente
//! (`mode_id`), uma capacidade (resumo, sugestões, tradução) ou o padrão.
//! A resolução é determinística: `Agent` > `Capability` > `Default`.
//!
//! A UI de vínculo é da PRD seguinte; o modelo de dados já existe para que o
//! roteador não precise ser refatorado depois (padrão `analysis_model` +
//! `analysis_model_source` do Perssua — R13).

use serde::{Deserialize, Serialize};

/// Capacidade da aplicação que pode ter uma fonte própria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiCapability {
    /// Resumo incremental da sessão.
    Summary,
    /// Sugestões contextuais.
    Suggestions,
    /// Tradução ao vivo.
    Translation,
}

impl AiCapability {
    /// Rótulo curto (usado no log e no evento `llm_source_used`).
    pub fn label(&self) -> &'static str {
        match self {
            AiCapability::Summary => "summary",
            AiCapability::Suggestions => "suggestions",
            AiCapability::Translation => "translation",
        }
    }
}

/// Escopo de uma rota de IA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AiRouteScope {
    /// Rota padrão (a fonte primária da lista).
    Default,
    /// Rota por capacidade.
    Capability(AiCapability),
    /// Rota por agente (id do `AssistantMode`).
    Agent(String),
}

impl AiRouteScope {
    /// Rótulo curto do escopo (log de debug e evento `llm_source_used`).
    ///
    /// Formato: `default`, `capability:summary`, `agent:meeting`.
    pub fn label(&self) -> String {
        match self {
            AiRouteScope::Default => "default".to_string(),
            AiRouteScope::Capability(capability) => format!("capability:{}", capability.label()),
            AiRouteScope::Agent(mode_id) => format!("agent:{mode_id}"),
        }
    }
}

/// Vínculo entre um escopo e uma fonte de IA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiRoute {
    /// Escopo vinculado.
    pub scope: AiRouteScope,
    /// Id da fonte (`AiSource.id`).
    pub source_id: String,
}

impl AiRoute {
    /// Cria um vínculo.
    ///
    /// Só os testes usam este atalho até a UI de rotas existir (BR-048).
    #[cfg(test)]
    pub fn new(scope: AiRouteScope, source_id: impl Into<String>) -> Self {
        Self {
            scope,
            source_id: source_id.into(),
        }
    }
}

/// Especificidade de um escopo (maior = mais específico).
fn specificity(scope: &AiRouteScope) -> u8 {
    match scope {
        AiRouteScope::Default => 0,
        AiRouteScope::Capability(_) => 1,
        AiRouteScope::Agent(_) => 2,
    }
}

/// `true` quando a rota casa com o escopo pedido.
fn matches(route: &AiRoute, scope: &AiRouteScope) -> bool {
    match (&route.scope, scope) {
        (AiRouteScope::Default, AiRouteScope::Default) => true,
        (AiRouteScope::Capability(a), AiRouteScope::Capability(b)) => a == b,
        (AiRouteScope::Agent(a), AiRouteScope::Agent(b)) => a == b,
        // Um escopo específico **não** casa com o padrão pedido: se o usuário
        // pediu o padrão, sources sem rota própria usam a primária.
        _ => false,
    }
}

/// Resolve a fonte de um escopo: a rota **mais específica** que casa.
///
/// `None` quando não há rota ⇒ o chamador usa a fonte primária.
pub fn resolve_route<'a>(routes: &'a [AiRoute], scope: &AiRouteScope) -> Option<&'a str> {
    routes
        .iter()
        .filter(|route| matches(route, scope))
        .max_by_key(|route| specificity(&route.scope))
        .map(|route| route.source_id.as_str())
}

/// Valida as rotas (id de fonte não vazio; escopo de agente com `mode_id`).
pub fn validate_routes(routes: &[AiRoute]) -> Result<(), String> {
    for route in routes {
        if route.source_id.trim().is_empty() {
            return Err("rota de IA precisa de um id de fonte".to_string());
        }
        if let AiRouteScope::Agent(mode_id) = &route.scope {
            if mode_id.trim().is_empty() {
                return Err("rota de agente precisa de um id de agente".to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(scope: AiRouteScope, source_id: &str) -> AiRoute {
        AiRoute {
            scope,
            source_id: source_id.to_string(),
        }
    }

    #[test]
    fn agent_route_wins_over_capability() {
        let routes = vec![
            route(AiRouteScope::Default, "default-src"),
            route(
                AiRouteScope::Capability(AiCapability::Summary),
                "summary-src",
            ),
            route(AiRouteScope::Agent("meeting".into()), "meeting-src"),
        ];
        assert_eq!(
            resolve_route(&routes, &AiRouteScope::Agent("meeting".into())),
            Some("meeting-src")
        );
        assert_eq!(
            resolve_route(&routes, &AiRouteScope::Capability(AiCapability::Summary)),
            Some("summary-src")
        );
    }

    #[test]
    fn capability_wins_over_default() {
        let routes = vec![
            route(AiRouteScope::Default, "default-src"),
            route(
                AiRouteScope::Capability(AiCapability::Translation),
                "tr-src",
            ),
        ];
        assert_eq!(
            resolve_route(
                &routes,
                &AiRouteScope::Capability(AiCapability::Translation)
            ),
            Some("tr-src")
        );
    }

    #[test]
    fn no_route_returns_none() {
        let routes = vec![route(AiRouteScope::Default, "default-src")];
        assert_eq!(
            resolve_route(
                &routes,
                &AiRouteScope::Capability(AiCapability::Suggestions)
            ),
            None
        );
        assert_eq!(resolve_route(&[], &AiRouteScope::Default), None);
    }

    #[test]
    fn a_specific_scope_does_not_inherit_the_default_route() {
        let routes = vec![route(AiRouteScope::Default, "default-src")];
        assert_eq!(
            resolve_route(&routes, &AiRouteScope::Default),
            Some("default-src")
        );
        assert_eq!(
            resolve_route(&routes, &AiRouteScope::Agent("meeting".into())),
            None
        );
    }

    #[test]
    fn rejects_empty_ids() {
        let routes = vec![route(AiRouteScope::Default, "  ")];
        assert!(validate_routes(&routes).is_err());
        let routes = vec![route(AiRouteScope::Agent("  ".into()), "src")];
        assert!(validate_routes(&routes).is_err());
    }
}
