//! Porta do gate de sugestões por blocos (BR-093/BR-094).
//!
//! O serviço de aplicação monta os blocos, chama o classificador e guarda a
//! última classificação por sessão. O agendador decide **quando** chamar
//! `classify_latest` e **quando** gerar; o gate aplica as guardas.
//!
//! TODO(task 10): remover o `allow(dead_code)` quando o scheduler consumir a
//! porta.
#![allow(dead_code)]

use crate::domain::suggestion_gate::{BlockConfig, GateDecision, HeuristicConfig, SuggestionCue};
use crate::ports::suggestion_cue_classifier::{ClassificationOutcome, ClassifierError};

/// Configuração do gate; a camada de settings monta isto na task 04.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionGateConfig {
    pub enabled: bool,
    pub block: BlockConfig,
    pub heuristic: HeuristicConfig,
    pub confidence_min: f32,
    pub cooldown_ms: i64,
    /// Allowlist efetiva. Vazia = todas as cues positivas.
    pub allowed_cues: Vec<SuggestionCue>,
    pub context_blocks: usize,
    /// Janela de falas carregadas a cada classificação.
    ///
    /// Maior que o antigo para o classificador enxergar o fio da conversa e
    /// reconhecer perguntas repetidas.
    pub window_ms: i64,
    pub max_segments: usize,
    /// Quantas perguntas já respondidas entram no histórico do classificador.
    pub answered_questions: usize,
    /// Idioma passado ao classificador.
    pub locale: String,
    /// Agente selecionado (contexto do classificador).
    pub mode_id: String,
    /// Contexto curto do agente selecionado (nome/categoria/foco).
    pub mode_context: String,
    /// Classifica bloco de um falante só (escopos `adaptive`/`they_only`).
    pub allow_single_speaker: bool,
    /// Só classifica quando THEY é o último a falar e projeta o bloco só com
    /// turnos de THEY (escopo `they_only`).
    pub they_only: bool,
    /// Janela de silêncio de THEY antes de classificar um bloco fechado
    /// (debounce estrutural; 0 desliga).
    pub they_quiet_ms: i64,
}

impl Default for SuggestionGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            block: BlockConfig::default(),
            heuristic: HeuristicConfig::default(),
            confidence_min: 0.70,
            cooldown_ms: 15_000,
            allowed_cues: Vec::new(),
            context_blocks: 8,
            window_ms: 300_000,
            max_segments: 120,
            answered_questions: 5,
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: false,
            they_only: false,
            they_quiet_ms: 0,
        }
    }
}

impl SuggestionGateConfig {
    /// `true` se a cue pode gerar sugestão conforme a allowlist.
    pub fn allows(&self, cue: SuggestionCue) -> bool {
        cue.can_suggest() && (self.allowed_cues.is_empty() || self.allowed_cues.contains(&cue))
    }
}

/// Falhas do gate.
#[derive(Debug, thiserror::Error)]
pub enum SuggestionGateError {
    #[error("nenhum bloco classificável")]
    NoBlock,
    #[error(transparent)]
    Classifier(#[from] ClassifierError),
    #[error("erro ao carregar falas: {0}")]
    Repository(String),
}

/// Classifica blocos e decide se a classificação pode gerar sugestão.
pub trait SuggestionGate: Send + Sync {
    /// Classifica o bloco mais recente e classificável.
    fn classify_latest(
        &self,
        session_id: &str,
    ) -> std::result::Result<ClassificationOutcome, SuggestionGateError>;

    /// Última classificação da sessão (memória; não chama o classificador).
    fn last(&self, session_id: &str) -> Option<ClassificationOutcome>;

    /// Decisão que ficou aguardando o cooldown (se houver).
    fn pending(&self, session_id: &str) -> Option<ClassificationOutcome>;

    /// Remove e devolve a decisão pendente.
    fn take_pending(&self, session_id: &str) -> Option<ClassificationOutcome>;

    /// Decide se o outcome atual deve gerar uma sugestão agora.
    ///
    /// Uma decisão positiva com cooldown pendente fica em `pending`; o próximo
    /// tick pode gerar sem reclassificar o bloco.
    fn should_generate(
        &self,
        outcome: &ClassificationOutcome,
        now_ms: i64,
        generating: bool,
    ) -> GateDecision;

    /// Registra que uma sugestão foi gerada para o bloco.
    fn mark_generated(&self, session_id: &str, block_id: &str, at_ms: i64);

    /// Limpa o cache da sessão (troca de sessão).
    fn reset(&self, session_id: &str);

    /// Aplica configurações novas sem recriar o gate.
    ///
    /// O agendador chama isto a cada tick com `config_from_settings`; salvar
    /// sensibilidade/cues/cooldown passa a valer no próximo bloco. Default
    /// no-op para dobras de teste que não observam configuração.
    fn reconfigure(&self, _config: SuggestionGateConfig) {}
}

/// Resultado auxiliar para a camada de aplicação que precisa publicar estado.
pub type GateResult<T> = std::result::Result<T, SuggestionGateError>;
