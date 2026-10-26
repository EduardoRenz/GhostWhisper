//! Blocos de conversa e decisão de **quando** sugerir (BR-092…BR-094).
//!
//! O modo automático usa um timer; o modo classificador usa esta camada para
//! transformar falas finalizadas em **blocos** e decidir se um bloco deve gerar
//! sugestão. Tudo aqui é puro: sem I/O, sem threads, sem relógio de parede.
//!
//! - [`build_blocks`] fecha blocos por silêncio, duração máxima ou nº de turnos.
//! - [`heuristic_cue`] é o backend local (sem IA) do classificador.
//! - [`decide`] aplica as guardas finais (categoria, confiança, cooldown,
//!   duplicidade e geração em curso).
//!
//! TODO(task 03): remover o `allow(dead_code)` quando o gate estiver ligado ao
//! scheduler; até lá o módulo é exercitado só pelos testes.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::domain::transcription::{Speaker, TranscriptSegment};

/// Estratégia de disparo das sugestões (BR-092).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionMode {
    /// Timer/cooldown atual.
    #[default]
    Auto,
    /// Classificador por blocos de conversa.
    Classifier,
}

impl SuggestionMode {
    /// Representação persistida.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Classifier => "classifier",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "classifier" => Some(Self::Classifier),
            _ => None,
        }
    }
}

/// Sensibilidade do classificador (BR-097).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierSensitivity {
    Low,
    #[default]
    Balanced,
    High,
}

impl ClassifierSensitivity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Balanced => "balanced",
            Self::High => "high",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "low" => Some(Self::Low),
            "balanced" => Some(Self::Balanced),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// Limiar de confiança efetivo.
    pub fn threshold(self) -> f32 {
        match self {
            Self::Low => 0.85,
            Self::Balanced => 0.70,
            Self::High => 0.55,
        }
    }

    /// Limiar calibrado para o Laya.
    ///
    /// O `laya-multilingual` é **não calibrado** (ECE ~0,31); as confianças
    /// ficam em 0,4–0,6. O mesmo preset usa um limiar mais baixo no Laya para
    /// não transformar toda hesitação em `insufficient_context`.
    pub fn laya_threshold(self) -> f32 {
        match self {
            Self::Low => 0.75,
            Self::Balanced => 0.50,
            Self::High => 0.35,
        }
    }

    /// Preset de cues positivas (o avançado pode restringir).
    pub fn default_allowed_cues(self) -> &'static [SuggestionCue] {
        match self {
            Self::Low => &[SuggestionCue::Question, SuggestionCue::Hesitation],
            Self::Balanced => &[
                SuggestionCue::Question,
                SuggestionCue::Hesitation,
                SuggestionCue::LostThread,
                SuggestionCue::Silence,
            ],
            Self::High => &[
                SuggestionCue::Question,
                SuggestionCue::Hesitation,
                SuggestionCue::LostThread,
                SuggestionCue::Invitation,
                SuggestionCue::Silence,
            ],
        }
    }
}

/// O que o classificador recomenda fazer (BR-094).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionCue {
    /// THEY fez uma pergunta direta ou pediu esclarecimento.
    Question,
    /// ME está travando, repetindo, se corrigindo ou sem terminar.
    Hesitation,
    /// ME perdeu o fio da conversa ou não sabe o que dizer.
    LostThread,
    /// THEY fez um ponto que convida ME a responder.
    Invitation,
    /// ME falou e a conversa ficou em silêncio.
    Silence,
    /// Nada útil a acrescentar.
    NoSuggestion,
    /// Contexto insuficiente/abstenção.
    InsufficientContext,
}

impl SuggestionCue {
    /// Representação persistida/exposta.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Hesitation => "hesitation",
            Self::LostThread => "lost_thread",
            Self::Invitation => "invitation",
            Self::Silence => "silence",
            Self::NoSuggestion => "no_suggestion",
            Self::InsufficientContext => "insufficient_context",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "question" => Some(Self::Question),
            "hesitation" => Some(Self::Hesitation),
            "lost_thread" => Some(Self::LostThread),
            "invitation" => Some(Self::Invitation),
            "silence" => Some(Self::Silence),
            "no_suggestion" => Some(Self::NoSuggestion),
            "insufficient_context" => Some(Self::InsufficientContext),
            _ => None,
        }
    }

    /// As duas categorias negativas nunca geram sugestão.
    pub fn can_suggest(self) -> bool {
        !matches!(self, Self::NoSuggestion | Self::InsufficientContext)
    }
}

/// Um turno dentro de um bloco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockTurn {
    pub speaker: Speaker,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// Um bloco de conversa fechado ou ainda em andamento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationBlock {
    /// Id determinístico: `blk_{session}_{start}_{end}_{last_segment}`.
    pub id: String,
    pub session_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub turns: Vec<BlockTurn>,
    pub me_chars: usize,
    pub they_chars: usize,
    pub is_open: bool,
}

impl ConversationBlock {
    /// Tem pelo menos uma fala de cada lado.
    pub fn has_both_speakers(&self) -> bool {
        self.turns.iter().any(|turn| turn.speaker == Speaker::Me)
            && self.turns.iter().any(|turn| turn.speaker == Speaker::They)
    }

    /// Bloco elegível para a classificação geral: fechado, com os dois falantes
    /// e com o mínimo de caracteres de cada lado.
    pub fn is_classifiable(&self, min_chars: usize) -> bool {
        self.is_classifiable_with(min_chars, false)
    }

    /// Variante do escopo adaptativo: aceita bloco de um falante só.
    ///
    /// O classificador/heurístico ainda decide a cue; um bloco curto ou sem
    /// sinal vira `insufficient_context` em vez de gerar sugestão.
    pub fn is_classifiable_with(&self, min_chars: usize, allow_single_speaker: bool) -> bool {
        if self.is_open || self.turns.is_empty() {
            return false;
        }
        if allow_single_speaker {
            return self.me_chars >= min_chars || self.they_chars >= min_chars;
        }
        self.has_both_speakers() && self.me_chars >= min_chars && self.they_chars >= min_chars
    }
}

/// Regras de montagem dos blocos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockConfig {
    /// Silêncio que fecha um bloco.
    pub gap_ms: i64,
    /// Duração máxima antes de forçar um novo bloco.
    pub max_block_ms: i64,
    /// Nº máximo de turnos por bloco.
    pub max_turns: usize,
    /// Caracteres mínimos de cada falante para o bloco ser classificável.
    pub min_chars: usize,
    /// Quantos blocos anteriores entram no contexto do classificador.
    pub context_blocks: usize,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            gap_ms: 3_000,
            max_block_ms: 45_000,
            max_turns: 12,
            min_chars: 8,
            context_blocks: 3,
        }
    }
}

/// Regras do backend heurístico (sem IA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeuristicConfig {
    /// Silêncio depois de ME que caracteriza `silence`.
    pub silence_ms: i64,
    /// Palavras mínimas para tentar classificar.
    pub min_words: usize,
    /// Fala contínua de ME que habilita o sinal de trailing off.
    pub hesitation_min_speech_ms: i64,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            silence_ms: 3_000,
            min_words: 3,
            hesitation_min_speech_ms: 4_000,
        }
    }
}

/// Observação de um turno em construção.
struct OpenBlock {
    session_id: String,
    start_ms: i64,
    end_ms: i64,
    turns: Vec<BlockTurn>,
    me_chars: usize,
    they_chars: usize,
    last_segment_id: String,
}

impl OpenBlock {
    fn new(segment: &TranscriptSegment) -> Self {
        let text = segment.text.trim().to_string();
        let chars = text.chars().count();
        let mut block = Self {
            session_id: segment.session_id.clone(),
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            turns: Vec::new(),
            me_chars: 0,
            they_chars: 0,
            last_segment_id: segment.id.clone(),
        };
        block.push(segment, text, chars);
        block
    }

    fn push(&mut self, segment: &TranscriptSegment, text: String, chars: usize) {
        self.end_ms = self.end_ms.max(segment.end_ms);
        self.last_segment_id = segment.id.clone();
        match segment.speaker {
            Speaker::Me => self.me_chars += chars,
            Speaker::They => self.they_chars += chars,
        }
        self.turns.push(BlockTurn {
            speaker: segment.speaker,
            text,
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
        });
    }

    fn duration_ms(&self) -> i64 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    fn finish(self, is_open: bool) -> ConversationBlock {
        ConversationBlock {
            id: format!(
                "blk_{}_{}_{}_{}",
                self.session_id, self.start_ms, self.end_ms, self.last_segment_id
            ),
            session_id: self.session_id,
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            turns: self.turns,
            me_chars: self.me_chars,
            they_chars: self.they_chars,
            is_open,
        }
    }
}

/// Monta os blocos de conversa a partir das falas **finalizadas**.
///
/// A função ignora falas não finais, vazias e de outra sessão. O último bloco
/// fica `is_open = true` até o silêncio (`now_ms - end_ms >= gap_ms`) ou até
/// atingir o limite de duração/turnos.
pub fn build_blocks(
    session_id: &str,
    segments: &[TranscriptSegment],
    config: &BlockConfig,
    now_ms: i64,
) -> Vec<ConversationBlock> {
    let mut ordered: Vec<&TranscriptSegment> = segments
        .iter()
        .filter(|segment| {
            segment.is_final && segment.session_id == session_id && !segment.text.trim().is_empty()
        })
        .collect();
    ordered.sort_by_key(|segment| (segment.start_ms, segment.end_ms));

    let mut blocks: Vec<ConversationBlock> = Vec::new();
    let mut open: Option<OpenBlock> = None;

    for segment in ordered {
        let text = segment.text.trim().to_string();
        let chars = text.chars().count();
        let Some(current) = open.as_mut() else {
            open = Some(OpenBlock::new(segment));
            continue;
        };

        let gap = segment.start_ms.saturating_sub(current.end_ms);
        let would_exceed_turns = current.turns.len() >= config.max_turns;
        let would_exceed_duration =
            segment.end_ms.saturating_sub(current.start_ms) > config.max_block_ms;

        if gap >= config.gap_ms || would_exceed_turns || would_exceed_duration {
            if let Some(finished) = open.take() {
                blocks.push(finished.finish(false));
            }
            open = Some(OpenBlock::new(segment));
        } else {
            current.push(segment, text, chars);
        }
    }

    if let Some(last) = open {
        let silence = now_ms.saturating_sub(last.end_ms) >= config.gap_ms;
        let limit =
            last.turns.len() >= config.max_turns || last.duration_ms() >= config.max_block_ms;
        blocks.push(last.finish(!silence && !limit));
    }

    blocks
}

fn normalized_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_question_mark(text: &str) -> bool {
    text.contains('?') || text.contains('？')
}

fn has_filler(words: &[String]) -> bool {
    const FILLERS: [&str; 12] = [
        "um",
        "uh",
        "hmm",
        "like",
        "basically",
        "well",
        "tipo",
        "ah",
        "eh",
        "ahn",
        "é",
        "né",
    ];
    words
        .iter()
        .any(|word| FILLERS.iter().any(|filler| word == filler))
}

fn has_adjacent_repeat(words: &[String]) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == pair[1] && pair[0].len() > 1)
}

fn has_self_correction(text: &str) -> bool {
    const MARKERS: [&str; 11] = [
        "i mean",
        "actually",
        "wait",
        "quer dizer",
        "na verdade",
        "espera",
        "digo",
        "ou melhor",
        "como é que fala",
        "how can i say",
        "what's the word",
    ];
    let lower = text.to_lowercase();
    MARKERS.iter().any(|marker| lower.contains(marker))
}

/// Incerteza explícita (“não tenho certeza”, “deixa eu pensar”).
///
/// É o sinal de hesitação que aparece **sem** uma pergunta de THEY; por isso
/// também vale para o bloco aberto de ME (sonda).
fn has_uncertainty(text: &str) -> bool {
    const MARKERS: [&str; 18] = [
        "não tenho certeza",
        "nao tenho certeza",
        "não sei",
        "nao sei",
        "deixa eu pensar",
        "deixa eu ver",
        "vou pensar",
        "não lembro",
        "nao lembro",
        "estou pensando",
        "como é que",
        "i'm not sure",
        "i am not sure",
        "not sure",
        "let me think",
        "let me see",
        "i don't know",
        "i dont know",
    ];
    let lower = text.to_lowercase();
    MARKERS.iter().any(|marker| lower.contains(marker))
}

fn has_trailing_off(text: &str, me_speech_ms: i64, min_speech_ms: i64) -> bool {
    if me_speech_ms < min_speech_ms {
        return false;
    }
    let trimmed = text.trim_end();
    trimmed.ends_with("...") || trimmed.ends_with('…')
}

/// Backend heurístico local: pergunta, hesitação, silêncio ou nada.
///
/// `lost_thread` e `invitation` são exclusivos do classificador de IA; o
/// heurístico não tenta adivinhar.
pub fn heuristic_cue(
    block: &ConversationBlock,
    now_ms: i64,
    config: &HeuristicConfig,
) -> SuggestionCue {
    let total_words: usize = block
        .turns
        .iter()
        .map(|turn| normalized_words(&turn.text).len())
        .sum();

    if total_words < config.min_words {
        return SuggestionCue::InsufficientContext;
    }

    let they_text = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::They)
        .map(|turn| turn.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if has_question_mark(&they_text) {
        return SuggestionCue::Question;
    }

    let me_text = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::Me)
        .map(|turn| turn.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let me_words = normalized_words(&me_text);
    let me_speech_ms: i64 = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::Me)
        .map(|turn| turn.end_ms.saturating_sub(turn.start_ms).max(0))
        .sum();

    // Hesitação pode existir num bloco só de ME: é o caso da sonda aberta
    // (BR-093). Por isso ela é avaliada antes da exigência dos dois falantes.
    let hesitation = me_words.len() >= config.min_words
        && (has_filler(&me_words)
            || has_adjacent_repeat(&me_words)
            || has_self_correction(&me_text)
            || has_uncertainty(&me_text)
            || has_trailing_off(&me_text, me_speech_ms, config.hesitation_min_speech_ms));
    if hesitation {
        return SuggestionCue::Hesitation;
    }

    if !block.has_both_speakers() {
        return SuggestionCue::InsufficientContext;
    }

    let last_is_me = block
        .turns
        .last()
        .is_some_and(|turn| turn.speaker == Speaker::Me);
    let silent = now_ms.saturating_sub(block.end_ms) >= config.silence_ms;
    if !block.is_open && last_is_me && silent {
        return SuggestionCue::Silence;
    }

    SuggestionCue::NoSuggestion
}

/// Regras da sonda de bloco aberto (hesitação com ME ainda falando).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenProbeConfig {
    /// `suggestion_probe_open_block`.
    pub enabled: bool,
    /// A cue `hesitation` está na allowlist.
    pub hesitation_allowed: bool,
    /// Palavras mínimas na fala de ME.
    pub min_words: usize,
    /// Duração mínima da fala contínua de ME.
    pub hesitation_ms: i64,
    /// Intervalo mínimo entre duas sondas da mesma sessão.
    pub throttle_ms: i64,
}

impl Default for OpenProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hesitation_allowed: true,
            min_words: 3,
            hesitation_ms: 4_000,
            throttle_ms: 2_000,
        }
    }
}

/// `true` quando o bloco aberto deve ser sondado agora.
///
/// A sonda só roda com ME no fim do bloco, fala contínua de ME com o mínimo de
/// palavras/duração e fora do throttle. O bloco precisa estar **aberto** (o
/// silêncio ainda não o fechou); um bloco fechado é classificado pelo caminho
/// normal.
pub fn open_probe_eligible(
    block: &ConversationBlock,
    now_ms: i64,
    last_probe_ms: Option<i64>,
    config: &OpenProbeConfig,
) -> bool {
    if !config.enabled || !config.hesitation_allowed || !block.is_open {
        return false;
    }
    let Some(last) = block.turns.last() else {
        return false;
    };
    if last.speaker != Speaker::Me {
        return false;
    }

    let mut words = 0_usize;
    let mut run_start: Option<i64> = None;
    let mut run_end = 0_i64;
    for turn in block.turns.iter().rev() {
        if turn.speaker != Speaker::Me {
            break;
        }
        words += normalized_words(&turn.text).len();
        run_start = Some(turn.start_ms);
        run_end = run_end.max(turn.end_ms);
    }

    let Some(run_start) = run_start else {
        return false;
    };
    if words < config.min_words {
        return false;
    }
    if run_end.saturating_sub(run_start) < config.hesitation_ms.max(0) {
        return false;
    }
    if let Some(last_probe) = last_probe_ms {
        if now_ms.saturating_sub(last_probe) < config.throttle_ms.max(0) {
            return false;
        }
    }
    true
}

/// Motivo de uma decisão negativa do gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateSkipReason {
    /// Automático desligado.
    Disabled,
    /// Sem sessão aberta.
    NoSession,
    /// Cue negativa ou `should_suggest=false`.
    NoSuggestion,
    /// Categoria desligada nas settings.
    CueNotAllowed,
    /// Confiança abaixo do limiar (ou inválida).
    BelowConfidence,
    /// Já existe uma geração em curso.
    Generating,
    /// Este bloco já gerou sugestão.
    AlreadyGenerated,
    /// Cooldown ainda não venceu.
    Cooldown { remaining_ms: i64 },
}

/// Decisão final do gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// Pode chamar o gerador.
    Generate,
    /// Não chamar o gerador, pelo motivo indicado.
    Skip(GateSkipReason),
}

/// Observação de um tick do gate.
#[derive(Debug, Clone, PartialEq)]
pub struct GateTick {
    pub enabled: bool,
    pub has_session: bool,
    pub cue: SuggestionCue,
    pub should_suggest: bool,
    pub confidence: f32,
    pub cue_allowed: bool,
    pub confidence_min: f32,
    pub cooldown_ms: i64,
    pub elapsed_since_last_suggestion_ms: i64,
    pub last_generated_block_id: Option<String>,
    pub block_id: String,
    pub generating: bool,
}

/// Decide se a classificação atual deve gerar uma sugestão.
///
/// A ordem das guardas é intencional: primeiro o que desliga o fluxo, depois a
/// qualidade da decisão e, por fim, as proteções contra duplicidade/rajada.
pub fn decide(tick: &GateTick) -> GateDecision {
    use GateDecision::{Generate, Skip};
    use GateSkipReason::{
        AlreadyGenerated, BelowConfidence, Cooldown, CueNotAllowed, Disabled, Generating,
        NoSession, NoSuggestion,
    };

    if !tick.enabled {
        return Skip(Disabled);
    }
    if !tick.has_session {
        return Skip(NoSession);
    }
    if !tick.should_suggest || !tick.cue.can_suggest() {
        return Skip(NoSuggestion);
    }
    if !tick.cue_allowed {
        return Skip(CueNotAllowed);
    }

    let min = if tick.confidence_min.is_finite() {
        tick.confidence_min.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if !tick.confidence.is_finite() || tick.confidence < min {
        return Skip(BelowConfidence);
    }
    if tick.generating {
        return Skip(Generating);
    }
    if tick
        .last_generated_block_id
        .as_deref()
        .is_some_and(|last| last == tick.block_id)
    {
        return Skip(AlreadyGenerated);
    }

    let cooldown = tick.cooldown_ms.max(0);
    let elapsed = tick.elapsed_since_last_suggestion_ms.max(0);
    if elapsed < cooldown {
        return Skip(Cooldown {
            remaining_ms: cooldown - elapsed,
        });
    }

    Generate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(
        id: &str,
        speaker: Speaker,
        text: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: "s1".to_string(),
            speaker,
            text: text.to_string(),
            start_ms,
            end_ms,
            is_final: true,
        }
    }

    fn config() -> BlockConfig {
        BlockConfig {
            gap_ms: 1_000,
            max_block_ms: 100_000,
            max_turns: 10,
            min_chars: 1,
            context_blocks: 3,
        }
    }

    fn heuristic_config() -> HeuristicConfig {
        HeuristicConfig::default()
    }

    fn gate_tick() -> GateTick {
        GateTick {
            enabled: true,
            has_session: true,
            cue: SuggestionCue::Question,
            should_suggest: true,
            confidence: 0.9,
            cue_allowed: true,
            confidence_min: 0.7,
            cooldown_ms: 15_000,
            elapsed_since_last_suggestion_ms: 20_000,
            last_generated_block_id: None,
            block_id: "blk_1".to_string(),
            generating: false,
        }
    }

    #[test]
    fn closes_block_on_silence_gap() {
        let segments = vec![
            segment("a", Speaker::Me, "vamos começar a revisão", 0, 500),
            segment("b", Speaker::They, "podemos sim, claro", 3_000, 3_500),
        ];
        let blocks = build_blocks("s1", &segments, &config(), 4_000);

        assert_eq!(blocks.len(), 2);
        assert!(!blocks[0].is_open);
        assert!(blocks[1].is_open);
        assert_eq!(blocks[0].turns.len(), 1);
        assert_eq!(blocks[1].turns.len(), 1);
    }

    #[test]
    fn closes_block_on_max_duration() {
        let mut config = config();
        config.max_block_ms = 1_000;
        let segments = vec![
            segment("a", Speaker::Me, "primeira fala longa", 0, 600),
            segment("b", Speaker::They, "segunda fala longa", 700, 1_200),
        ];
        let blocks = build_blocks("s1", &segments, &config, 1_300);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start_ms, 0);
        assert_eq!(blocks[0].end_ms, 600);
        assert_eq!(blocks[1].start_ms, 700);
    }

    #[test]
    fn closes_block_on_max_turns() {
        let mut config = config();
        config.max_turns = 2;
        let segments = vec![
            segment("a", Speaker::Me, "primeira fala", 0, 100),
            segment("b", Speaker::They, "segunda fala", 200, 300),
            segment("c", Speaker::Me, "terceira fala", 400, 500),
        ];
        let blocks = build_blocks("s1", &segments, &config, 600);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].turns.len(), 2);
        assert_eq!(blocks[1].turns.len(), 1);
    }

    #[test]
    fn last_block_stays_open_until_gap() {
        let segments = vec![segment("a", Speaker::Me, "uma fala qualquer", 0, 500)];

        let open = build_blocks("s1", &segments, &config(), 1_000);
        assert!(open[0].is_open);

        let closed = build_blocks("s1", &segments, &config(), 1_600);
        assert!(!closed[0].is_open);
    }

    #[test]
    fn block_id_is_deterministic() {
        let segments = vec![
            segment("a", Speaker::Me, "fala do usuário", 0, 500),
            segment("b", Speaker::They, "resposta do outro", 1_500, 2_000),
        ];
        let first = build_blocks("s1", &segments, &config(), 3_000);
        let second = build_blocks("s1", &segments, &config(), 3_000);
        let mut other_segments = segments.clone();
        for segment in &mut other_segments {
            segment.session_id = "s2".to_string();
        }
        let other_session = build_blocks("s2", &other_segments, &config(), 3_000);

        assert_eq!(first[0].id, second[0].id);
        assert_ne!(first[0].id, other_session[0].id);
        assert!(first[0].id.contains("blk_s1_0_500_a"));
    }

    #[test]
    fn mode_and_cue_strings_are_stable() {
        assert_eq!(SuggestionMode::Auto.as_str(), "auto");
        assert_eq!(
            SuggestionMode::from_str("classifier"),
            Some(SuggestionMode::Classifier)
        );
        assert_eq!(SuggestionMode::from_str("nope"), None);

        assert_eq!(SuggestionCue::Question.as_str(), "question");
        assert_eq!(
            SuggestionCue::from_str("lost_thread"),
            Some(SuggestionCue::LostThread)
        );
        assert_eq!(SuggestionCue::from_str("nope"), None);
        assert!(SuggestionCue::Question.can_suggest());
        assert!(!SuggestionCue::NoSuggestion.can_suggest());
        assert!(!SuggestionCue::InsufficientContext.can_suggest());
    }

    #[test]
    fn classifiable_requires_both_speakers_and_min_chars() {
        let only_me = build_blocks(
            "s1",
            &[segment("a", Speaker::Me, "só eu falei aqui", 0, 500)],
            &config(),
            2_000,
        );
        assert!(!only_me[0].is_classifiable(1));

        let both = build_blocks(
            "s1",
            &[
                segment("a", Speaker::Me, "eu falei bastante", 0, 500),
                segment("b", Speaker::They, "o outro respondeu", 1_000, 1_500),
            ],
            &config(),
            3_000,
        );
        assert!(both[0].is_classifiable(1));
        assert!(!both[0].is_classifiable(100));
    }

    #[test]
    fn ignores_non_final_empty_and_other_session() {
        let mut non_final = segment("a", Speaker::Me, "não finalizada", 0, 500);
        non_final.is_final = false;
        let mut other_session = segment("b", Speaker::They, "outra sessão", 0, 500);
        other_session.session_id = "s2".to_string();
        let empty = segment("c", Speaker::They, "   ", 0, 500);
        let valid = segment("d", Speaker::Me, "fala válida", 0, 500);

        let blocks = build_blocks(
            "s1",
            &[non_final, other_session, empty, valid],
            &config(),
            1_000,
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].turns.len(), 1);
        assert_eq!(blocks[0].turns[0].text, "fala válida");
    }

    fn block_with_turns(turns: Vec<(&str, Speaker, &str, i64, i64)>) -> ConversationBlock {
        let segments: Vec<TranscriptSegment> = turns
            .into_iter()
            .map(|(id, speaker, text, start, end)| segment(id, speaker, text, start, end))
            .collect();
        build_blocks("s1", &segments, &config(), 10_000)
            .into_iter()
            .next()
            .expect("bloco")
    }

    fn open_block(turns: Vec<(Speaker, &str, i64, i64)>) -> ConversationBlock {
        let segments: Vec<TranscriptSegment> = turns
            .into_iter()
            .enumerate()
            .map(|(index, (speaker, text, start, end))| {
                segment(&format!("t{index}"), speaker, text, start, end)
            })
            .collect();
        let now = segments
            .last()
            .map(|segment| segment.end_ms + 400)
            .unwrap_or(0);
        build_blocks("s1", &segments, &config(), now)
            .into_iter()
            .last()
            .expect("bloco")
    }

    #[test]
    fn build_blocks_is_fast_for_forty_segments() {
        let segments: Vec<TranscriptSegment> = (0..40)
            .map(|index| {
                let start = index * 1_000;
                let speaker = if index % 2 == 0 {
                    Speaker::Me
                } else {
                    Speaker::They
                };
                segment(
                    &format!("s{index}"),
                    speaker,
                    "uma fala de tamanho razoável para o bloco",
                    start,
                    start + 800,
                )
            })
            .collect();
        let started = std::time::Instant::now();
        let blocks = build_blocks("s1", &segments, &config(), 25_000);
        let elapsed = started.elapsed();
        println!("build_blocks 40 segmentos: {elapsed:?}");
        assert!(!blocks.is_empty());
        // Alvo da task 11 é < 5 ms; a folga evita flakiness em CI lento.
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "build_blocks levou {elapsed:?}"
        );
    }

    #[test]
    fn adaptive_scope_accepts_single_speaker_blocks() {
        let me_only = block_with_turns(vec![("a", Speaker::Me, "deixa eu pensar aqui", 0, 5_000)]);
        assert!(!me_only.is_classifiable_with(8, false));
        assert!(me_only.is_classifiable_with(8, true));

        let they_only = block_with_turns(vec![("a", Speaker::They, "pode explicar?", 0, 1_000)]);
        assert!(!they_only.is_classifiable_with(8, false));
        assert!(they_only.is_classifiable_with(8, true));

        let too_short = block_with_turns(vec![("a", Speaker::Me, "oi", 0, 100)]);
        assert!(!too_short.is_classifiable_with(8, true));
    }

    #[test]
    fn open_probe_requires_me_hesitating_in_an_open_block() {
        let config = OpenProbeConfig::default();
        let eligible = open_block(vec![
            (Speaker::They, "certo", 0, 500),
            (Speaker::Me, "é... tipo assim... eu acho", 1_000, 5_500),
        ]);
        assert!(eligible.is_open);
        assert!(open_probe_eligible(&eligible, 6_000, None, &config));

        let they_last = open_block(vec![
            (Speaker::Me, "fala longa do usuário", 0, 4_500),
            (Speaker::They, "ok", 5_000, 5_500),
        ]);
        assert!(!open_probe_eligible(&they_last, 6_000, None, &config));

        let short = open_block(vec![
            (Speaker::They, "certo", 0, 500),
            (Speaker::Me, "é... tipo assim", 6_000, 7_000),
        ]);
        assert!(!open_probe_eligible(&short, 7_400, None, &config));

        let few_words = open_block(vec![
            (Speaker::They, "certo", 0, 500),
            (Speaker::Me, "é... ahn", 1_000, 5_500),
        ]);
        assert!(!open_probe_eligible(&few_words, 6_000, None, &config));

        let closed = open_block(vec![
            (Speaker::They, "certo", 0, 500),
            (Speaker::Me, "é... tipo assim... eu acho", 1_000, 5_500),
        ]);
        let closed = ConversationBlock {
            is_open: false,
            ..closed
        };
        assert!(!open_probe_eligible(&closed, 6_000, None, &config));

        assert!(!open_probe_eligible(&eligible, 7_000, Some(6_000), &config));
        assert!(open_probe_eligible(&eligible, 8_000, Some(6_000), &config));
        assert!(!open_probe_eligible(
            &eligible,
            8_000,
            None,
            &OpenProbeConfig {
                enabled: false,
                ..config
            }
        ));
        assert!(!open_probe_eligible(
            &eligible,
            8_000,
            None,
            &OpenProbeConfig {
                hesitation_allowed: false,
                ..config
            }
        ));
    }

    #[test]
    fn heuristic_detects_question_pt_en() {
        let pt = block_with_turns(vec![
            ("a", Speaker::Me, "eu consigo explicar isso", 0, 500),
            ("b", Speaker::They, "pode explicar melhor?", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&pt, 2_000, &heuristic_config()),
            SuggestionCue::Question
        );

        let en = block_with_turns(vec![
            ("a", Speaker::Me, "i can explain that", 0, 500),
            ("b", Speaker::They, "can you explain it?", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&en, 2_000, &heuristic_config()),
            SuggestionCue::Question
        );
    }

    #[test]
    fn heuristic_detects_hesitation_fillers_and_self_correction() {
        let pt = block_with_turns(vec![
            ("a", Speaker::Me, "é, tipo assim, eu acho", 0, 2_000),
            ("b", Speaker::They, "certo", 2_500, 2_800),
        ]);
        assert_eq!(
            heuristic_cue(&pt, 3_000, &heuristic_config()),
            SuggestionCue::Hesitation
        );

        let en = block_with_turns(vec![
            ("a", Speaker::Me, "i mean, actually, wait", 0, 2_000),
            ("b", Speaker::They, "ok", 2_500, 2_800),
        ]);
        assert_eq!(
            heuristic_cue(&en, 3_000, &heuristic_config()),
            SuggestionCue::Hesitation
        );
    }

    #[test]
    fn heuristic_detects_trailing_off_after_long_speech() {
        let long = block_with_turns(vec![
            ("a", Speaker::Me, "eu estava pensando que...", 0, 5_000),
            ("b", Speaker::They, "sim", 5_500, 5_800),
        ]);
        assert_eq!(
            heuristic_cue(&long, 6_000, &heuristic_config()),
            SuggestionCue::Hesitation
        );

        let short = block_with_turns(vec![
            ("a", Speaker::Me, "eu estava pensando que...", 0, 1_000),
            ("b", Speaker::They, "sim", 1_500, 1_800),
        ]);
        assert_eq!(
            heuristic_cue(&short, 2_000, &heuristic_config()),
            SuggestionCue::NoSuggestion
        );
    }

    #[test]
    fn heuristic_detects_silence_after_me() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "vamos seguir com o plano", 0, 500),
            ("b", Speaker::They, "certo", 1_000, 1_200),
            ("c", Speaker::Me, "entao podemos fechar assim", 1_500, 2_000),
        ]);
        // `build_blocks` recebeu now=10_000, então o bloco já está fechado.
        assert_eq!(
            heuristic_cue(&block, 6_000, &heuristic_config()),
            SuggestionCue::Silence
        );
    }

    #[test]
    fn heuristic_detects_uncertainty_in_me_only_block() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "Eu não tenho certeza.", 28_680, 32_182),
            (
                "b",
                Speaker::Me,
                "Teza, mas deixa eu pensar.",
                32_182,
                34_187,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&block, 35_000, &heuristic_config()),
            SuggestionCue::Hesitation
        );
    }

    #[test]
    fn heuristic_me_only_without_markers_is_insufficient() {
        let block = block_with_turns(vec![(
            "a",
            Speaker::Me,
            "Beleza, vamos começar então.",
            12,
            3_680,
        )]);
        assert_eq!(
            heuristic_cue(&block, 5_000, &heuristic_config()),
            SuggestionCue::InsufficientContext
        );
    }

    #[test]
    fn heuristic_returns_no_suggestion_for_fluent_exchange() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "vamos revisar o cronograma", 0, 1_000),
            ("b", Speaker::They, "concordo com o plano", 1_500, 2_000),
            (
                "c",
                Speaker::Me,
                "fechamos os proximos passos",
                2_500,
                3_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&block, 4_000, &heuristic_config()),
            SuggestionCue::NoSuggestion
        );
    }

    #[test]
    fn heuristic_returns_insufficient_context_for_short_or_one_sided_block() {
        let short = block_with_turns(vec![
            ("a", Speaker::Me, "oi", 0, 100),
            ("b", Speaker::They, "ok", 200, 300),
        ]);
        assert_eq!(
            heuristic_cue(&short, 1_000, &heuristic_config()),
            SuggestionCue::InsufficientContext
        );

        let one_sided =
            block_with_turns(vec![("a", Speaker::Me, "eu falei uma frase longa", 0, 500)]);
        assert_eq!(
            heuristic_cue(&one_sided, 1_000, &heuristic_config()),
            SuggestionCue::InsufficientContext
        );
    }

    #[test]
    fn heuristic_never_returns_lost_thread_or_invitation() {
        let fixtures = vec![
            block_with_turns(vec![
                ("a", Speaker::Me, "vamos revisar o cronograma", 0, 500),
                ("b", Speaker::They, "concordo com o plano", 1_000, 1_500),
            ]),
            block_with_turns(vec![
                ("a", Speaker::Me, "é, tipo assim", 0, 2_000),
                ("b", Speaker::They, "certo", 2_500, 2_800),
            ]),
        ];
        for block in fixtures {
            let cue = heuristic_cue(&block, 5_000, &heuristic_config());
            assert_ne!(cue, SuggestionCue::LostThread);
            assert_ne!(cue, SuggestionCue::Invitation);
        }
    }

    #[test]
    fn decide_skips_disabled_or_without_session() {
        let disabled = GateTick {
            enabled: false,
            ..gate_tick()
        };
        assert_eq!(
            decide(&disabled),
            GateDecision::Skip(GateSkipReason::Disabled)
        );

        let no_session = GateTick {
            has_session: false,
            ..gate_tick()
        };
        assert_eq!(
            decide(&no_session),
            GateDecision::Skip(GateSkipReason::NoSession)
        );
    }

    #[test]
    fn decide_skips_no_suggestion_and_disallowed_cue() {
        let no_suggestion = GateTick {
            should_suggest: false,
            ..gate_tick()
        };
        assert_eq!(
            decide(&no_suggestion),
            GateDecision::Skip(GateSkipReason::NoSuggestion)
        );

        let negative_cue = GateTick {
            cue: SuggestionCue::NoSuggestion,
            ..gate_tick()
        };
        assert_eq!(
            decide(&negative_cue),
            GateDecision::Skip(GateSkipReason::NoSuggestion)
        );

        let disallowed = GateTick {
            cue_allowed: false,
            ..gate_tick()
        };
        assert_eq!(
            decide(&disallowed),
            GateDecision::Skip(GateSkipReason::CueNotAllowed)
        );
    }

    #[test]
    fn decide_skips_below_confidence_or_invalid_confidence() {
        let below = GateTick {
            confidence: 0.2,
            ..gate_tick()
        };
        assert_eq!(
            decide(&below),
            GateDecision::Skip(GateSkipReason::BelowConfidence)
        );

        let nan = GateTick {
            confidence: f32::NAN,
            ..gate_tick()
        };
        assert_eq!(
            decide(&nan),
            GateDecision::Skip(GateSkipReason::BelowConfidence)
        );
    }

    #[test]
    fn decide_skips_generating_duplicate_and_cooldown() {
        let generating = GateTick {
            generating: true,
            ..gate_tick()
        };
        assert_eq!(
            decide(&generating),
            GateDecision::Skip(GateSkipReason::Generating)
        );

        let duplicate = GateTick {
            last_generated_block_id: Some("blk_1".to_string()),
            ..gate_tick()
        };
        assert_eq!(
            decide(&duplicate),
            GateDecision::Skip(GateSkipReason::AlreadyGenerated)
        );

        let cooldown = GateTick {
            elapsed_since_last_suggestion_ms: 5_000,
            ..gate_tick()
        };
        assert_eq!(
            decide(&cooldown),
            GateDecision::Skip(GateSkipReason::Cooldown {
                remaining_ms: 10_000
            })
        );
    }

    #[test]
    fn decide_generates_when_all_conditions_hold() {
        assert_eq!(decide(&gate_tick()), GateDecision::Generate);

        let no_cooldown = GateTick {
            cooldown_ms: 0,
            elapsed_since_last_suggestion_ms: 0,
            ..gate_tick()
        };
        assert_eq!(decide(&no_cooldown), GateDecision::Generate);
    }
}
