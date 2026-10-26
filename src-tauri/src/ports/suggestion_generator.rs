//! Porta do caso de uso de sugestão (inbound port).
//!
//! Substitui o antigo `InsightGenerator`: agora a sugestão tem **modo/agente**,
//! pode ser efêmera (janela) ou persistida (chat/atalho/fixar) e sabe se pulou
//! um tick sem fala nova.

use serde::Serialize;

use crate::domain::transcription::Speaker;
use crate::error::Result;

use super::manual_suggestion_context::ManualSuggestionContext;

/// Pedido de sugestão.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuggestionRequest {
    /// Sessão alvo (`None` = ativa ou a mais recente).
    pub session_id: Option<String>,
    /// Agente (`None` = o selecionado nas configurações).
    pub mode_id: Option<String>,
    /// `true` grava a sugestão em `messages` (chat/atalho/fixar).
    pub persist: bool,
    /// Último instante já considerado (`None` força uma nova análise).
    pub since_ms: Option<i64>,
    /// Sugestão anterior, para o modelo não repetir.
    pub last_suggestion: Option<String>,
    /// Contexto colado pelo usuário (fonte suplementar, nunca instrução).
    pub extra_context: Option<String>,
    /// Gera primeiro um **rascunho curto** (fast-first) antes do texto completo.
    ///
    /// Transposto do Perssua (`fastFirstInsight`): a primeira bolha aparece
    /// rápido e curta; o texto completo vem em seguida (BR-028).
    pub fast_first: bool,
    /// `true` gera **mesmo sem fala nova** (botão “Sugerir agora”).
    ///
    /// Sem `force`, o serviço devolve a última sugestão da sessão quando não
    /// houve fala nova — nenhuma chamada ao LLM (BR-039).
    pub force: bool,
    /// Mensagem selecionada na Transcrição (sugestão manual — BR-041).
    ///
    /// Ação explícita do usuário: entra como foco do prompt e sempre gera,
    /// mesmo sem fala nova.
    pub manual_context: Option<ManualSuggestionContext>,
    /// Cue que disparou a sugestão (modo classificador — BR-094).
    pub cue: Option<crate::domain::suggestion_gate::SuggestionCue>,
    /// Foco curto vindo do classificador.
    pub focus: Option<String>,
    /// Trigger explícito (classificador); `None` mantém a inferência atual.
    pub trigger: Option<SuggestionTrigger>,
}

impl SuggestionRequest {
    /// Pedido persistido para o chat/atalho, com o agente selecionado.
    pub fn persist(session_id: Option<String>) -> Self {
        Self {
            session_id,
            persist: true,
            ..Self::default()
        }
    }

    /// Pedido efêmero para a janela de sugestões.
    pub fn ephemeral(session_id: Option<String>, mode_id: String) -> Self {
        Self {
            session_id,
            mode_id: Some(mode_id),
            persist: false,
            ..Self::default()
        }
    }

    /// Define o instante mínimo considerado.
    pub fn with_since_ms(mut self, since_ms: Option<i64>) -> Self {
        self.since_ms = since_ms;
        self
    }

    /// Inclui a sugestão anterior.
    pub fn with_last_suggestion(mut self, last_suggestion: Option<String>) -> Self {
        self.last_suggestion = last_suggestion.filter(|text| !text.trim().is_empty());
        self
    }

    /// Inclui o contexto extra do usuário.
    pub fn with_extra_context(mut self, extra_context: Option<String>) -> Self {
        self.extra_context = extra_context.filter(|text| !text.trim().is_empty());
        self
    }

    /// Pede o rascunho curto (fast-first) antes do passe completo.
    pub fn with_fast_first(mut self, fast_first: bool) -> Self {
        self.fast_first = fast_first;
        self
    }

    /// Força a geração mesmo sem fala nova (“Sugerir agora”).
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Inclui a mensagem selecionada (sugestão manual).
    pub fn with_manual_context(mut self, manual_context: Option<ManualSuggestionContext>) -> Self {
        self.manual_context = manual_context;
        self
    }

    /// Inclui a cue/foco do classificador (modo classificador).
    // TODO(task 10): remover o allow quando o scheduler chamar este builder.
    #[allow(dead_code)]
    pub fn with_cue_focus(
        mut self,
        cue: Option<crate::domain::suggestion_gate::SuggestionCue>,
        focus: Option<String>,
    ) -> Self {
        self.cue = cue;
        self.focus = focus.filter(|value| !value.trim().is_empty());
        self
    }

    /// Define o trigger explicitamente (modo classificador).
    pub fn with_trigger(mut self, trigger: SuggestionTrigger) -> Self {
        self.trigger = Some(trigger);
        self
    }
}

/// Fase da sugestão (BR-038).
///
/// A janela de sugestões mostra **dois blocos**: a resposta curta (que chega
/// primeiro, do passe *fast-first*) e a resposta completa (Markdown). A fase
/// diz à UI qual bloco o texto pertence — a rápida **não** substitui a
/// completa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionPhase {
    /// Rascunho curto (passe *fast-first*).
    Quick,
    /// Texto completo (Markdown).
    Full,
}

/// Origem da geração (BR-041/BR-042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionTrigger {
    /// Agendador automático (janela aberta + intervalo).
    Auto,
    /// Ação explícita “Sugerir agora”/atalho.
    Now,
    /// Sugestão manual a partir de uma mensagem da Transcrição.
    Manual,
    /// Disparada pelo classificador de blocos.
    // TODO(task 10): remover o allow quando o scheduler usar o trigger.
    #[allow(dead_code)]
    Classifier,
}

/// Snapshot **leve** da mensagem que originou uma sugestão manual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestionSource {
    pub speaker: Speaker,
    /// Tempo da mensagem desde o início da sessão (ms).
    pub at_ms: i64,
    /// Primeiros caracteres da mensagem (sem o texto completo).
    pub preview: String,
}

/// Sugestão gerada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    pub session_id: String,
    /// Id do agente que gerou.
    pub mode_id: String,
    /// Nome do agente (rótulo da UI).
    pub mode_name: String,
    /// Ícone do agente no momento da resposta (snapshot — BR-037).
    pub mode_icon: String,
    /// Cor do agente no momento da resposta (snapshot — BR-037).
    pub mode_color: String,
    /// UUID da geração; `quick`/`full`/deltas da mesma chamada compartilham.
    pub generation_id: String,
    /// Origem da geração (BR-041/BR-042).
    pub trigger: SuggestionTrigger,
    /// Snapshot da mensagem de origem (só na manual; BR-041).
    pub source: Option<SuggestionSource>,
    /// Fase da sugestão (BR-038).
    pub phase: SuggestionPhase,
    /// Título curto gerado pelo agente (`Subject:`), quando houver (BR-038).
    ///
    /// Vem do **retorno estruturado leve** do modelo; vazio nas fases antigas
    /// e na resposta rápida (a UI cai para o título derivativo).
    pub title: String,
    /// Markdown sugerido (sem a linha `Subject:`).
    pub text: String,
    /// Milissegundos desde o início da sessão.
    pub at_ms: i64,
    /// Início da última fala considerada (cursor do próximo pedido).
    pub context_until_ms: i64,
    /// Id da mensagem gravada (quando `persist`/`pin`).
    pub message_id: Option<String>,
    /// `true` quando não havia fala nova e o LLM não foi chamado.
    pub skipped: bool,
    /// Cue do classificador (quando `trigger = Classifier`).
    pub cue: Option<crate::domain::suggestion_gate::SuggestionCue>,
}

/// Pedido de fixação de uma sugestão já gerada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinSuggestionRequest {
    pub session_id: String,
    pub mode_id: String,
    pub text: String,
    /// Instante original da sugestão (ms desde o início da sessão).
    pub at_ms: i64,
}

/// Gera sugestões contextuais por agente.
///
/// Implementado pela aplicação (`SuggestionService`) e chamado pelo IPC.
pub trait SuggestionGenerator: Send + Sync {
    /// Gera (e opcionalmente persiste) uma sugestão.
    fn generate(&self, request: SuggestionRequest) -> Result<Suggestion>;
    /// Persiste uma sugestão já gerada, sem chamar o LLM.
    fn pin(&self, request: PinSuggestionRequest) -> Result<Suggestion>;
    /// Última sugestão lembrada da sessão (cache; não chama o LLM).
    ///
    /// Usada pela janela para hidratar o cartão ao montar/reabrir (BR-042).
    fn last_suggestion(&self, session_id: &str) -> Result<Option<Suggestion>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_trigger_serializes_classifier() {
        let json = serde_json::to_value(SuggestionTrigger::Classifier).expect("json");
        assert_eq!(json, "classifier");
    }

    #[test]
    fn cue_focus_builder_keeps_only_non_empty_focus() {
        let request = SuggestionRequest::ephemeral(None, "default".into()).with_cue_focus(
            Some(crate::domain::suggestion_gate::SuggestionCue::Question),
            Some("  ".into()),
        );
        assert_eq!(
            request.cue,
            Some(crate::domain::suggestion_gate::SuggestionCue::Question)
        );
        assert!(request.focus.is_none());
    }
}
