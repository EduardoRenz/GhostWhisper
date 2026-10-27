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
    ///
    /// A taxonomia nova só tem duas decisões acionáveis: responder a uma
    /// pergunta ou comentar um ponto. A sensibilidade muda apenas o limiar de
    /// confiança, não as categorias.
    pub fn default_allowed_cues(self) -> &'static [SuggestionCue] {
        &[SuggestionCue::Question, SuggestionCue::Comment]
    }
}

/// O que o classificador recomenda fazer (BR-094).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionCue {
    /// THEY fez uma pergunta que ME ainda não respondeu.
    Question,
    /// THEY fez um ponto/opinião que convida ME a comentar.
    Comment,
    /// A pergunta atual repete uma pergunta já respondida no histórico
    /// recente (ou ME já respondeu dentro do próprio bloco).
    AlreadyAnswered,
    /// Conversa casual, explicação ou fala sem pergunta/convite: não intervir.
    NoAction,
    /// Contexto insuficiente/abstenção.
    InsufficientContext,
}

impl SuggestionCue {
    /// Representação persistida/exposta.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Comment => "comment",
            Self::AlreadyAnswered => "already_answered",
            Self::NoAction => "no_action",
            Self::InsufficientContext => "insufficient_context",
        }
    }

    /// Converte do valor persistido.
    ///
    /// Aceita os nomes antigos para não quebrar settings salvas: `invitation`
    /// vira `comment`; `no_suggestion` vira `no_action`; as cues removidas de
    /// hesitação/silêncio são descartadas (o sanitize mantém só positivas).
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "question" => Some(Self::Question),
            "comment" | "invitation" => Some(Self::Comment),
            "already_answered" => Some(Self::AlreadyAnswered),
            "no_action" | "no_suggestion" => Some(Self::NoAction),
            "insufficient_context" => Some(Self::InsufficientContext),
            _ => None,
        }
    }

    /// Só pergunta e comentário geram sugestão.
    pub fn can_suggest(self) -> bool {
        matches!(self, Self::Question | Self::Comment)
    }
}

/// Uma pergunta já respondida que o classificador usa para detectar repetição.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorQuestion {
    /// Título curto (a própria pergunta, truncada).
    pub title: String,
    /// Bloco que originou a pergunta.
    pub block_id: String,
    /// Instante (ms da sessão) em que a sugestão foi gerada.
    pub answered_at_ms: i64,
}

impl PriorQuestion {
    /// Título seguro para prompt/UI (≤ 180 caracteres).
    pub fn normalized(mut self) -> Self {
        self.title = self.title.chars().take(180).collect();
        self
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
            // A classificação precisa de mais histórico para detectar perguntas
            // repetidas e entender o fio da conversa.
            context_blocks: 8,
        }
    }
}

/// Regras do backend heurístico (sem IA).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeuristicConfig {
    /// Palavras mínimas para tentar classificar.
    pub min_words: usize,
    /// Similaridade Jaccard mínima para considerar uma pergunta repetida.
    pub repeated_question_similarity: f32,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            min_words: 3,
            repeated_question_similarity: 0.60,
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

fn is_question_text(text: &str) -> bool {
    if has_question_mark(text) {
        return true;
    }
    let lower = text.to_lowercase();
    let words = normalized_words(text);
    let Some(first) = words.first().map(String::as_str) else {
        return false;
    };
    const LEADING: [&str; 26] = [
        "qual", "quais", "quando", "onde", "quem", "quanto", "quantos", "quantas", "porque",
        "what", "how", "why", "when", "where", "who", "which", "can", "could", "would", "should",
        "do", "does", "did", "is", "are", "will",
    ];
    if LEADING.contains(&first) {
        return true;
    }
    const REQUESTS: [&str; 12] = [
        "me explica",
        "me conta",
        "me diz",
        "fale sobre",
        "pode falar",
        "pode detalhar",
        "walk me through",
        "tell me",
        "explain",
        "describe",
        "give me an example",
        "show me",
    ];
    REQUESTS.iter().any(|marker| lower.contains(marker))
}

/// A última pergunta feita por THEY no bloco.
pub fn question_text(block: &ConversationBlock) -> Option<String> {
    block
        .turns
        .iter()
        .rev()
        .filter(|turn| turn.speaker == Speaker::They)
        .find(|turn| is_question_text(&turn.text))
        .map(|turn| turn.text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// Título curto do bloco para o histórico de já respondidas.
///
/// Usa a fala completa de THEY (não só a última pergunta): a pergunta real pode
/// estar dividida em vários turnos ou vir depois de uma explicação. Se não
/// houver THEY, usa a última fala.
pub fn question_title(block: &ConversationBlock) -> String {
    let they_text = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::They)
        .map(|turn| turn.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let raw = if they_text.is_empty() {
        block
            .turns
            .last()
            .map(|turn| turn.text.trim().to_string())
            .unwrap_or_default()
    } else {
        they_text
    };
    raw.chars().take(180).collect()
}

/// Projeção do bloco para o **alvo**: falas de THEY desde a última fala de ME.
///
/// Retorna `None` quando o último turno não é de THEY: nesse caso não há
/// pergunta/convite pendente para o copiloto responder. O bloco completo
/// (com ME+THEY) continua disponível como contexto.
pub fn they_target(block: &ConversationBlock) -> Option<ConversationBlock> {
    if block.turns.last().map(|turn| turn.speaker) != Some(Speaker::They) {
        return None;
    }
    let start = block
        .turns
        .iter()
        .rposition(|turn| turn.speaker == Speaker::Me)
        .map(|index| index + 1)
        .unwrap_or(0);
    let turns: Vec<BlockTurn> = block.turns[start..]
        .iter()
        .filter(|turn| turn.speaker == Speaker::They)
        .cloned()
        .collect();
    if turns.is_empty() {
        return None;
    }
    let they_chars = turns.iter().map(|turn| turn.text.chars().count()).sum();
    Some(ConversationBlock {
        id: block.id.clone(),
        session_id: block.session_id.clone(),
        start_ms: block.start_ms,
        end_ms: block.end_ms,
        turns,
        me_chars: 0,
        they_chars,
        is_open: block.is_open,
    })
}

/// ME já respondeu dentro do próprio bloco depois da última pergunta de THEY.
fn answered_in_block(block: &ConversationBlock) -> bool {
    let Some(last_question) = block
        .turns
        .iter()
        .rposition(|turn| turn.speaker == Speaker::They && is_question_text(&turn.text))
    else {
        return false;
    };
    block.turns[last_question + 1..]
        .iter()
        .any(|turn| turn.speaker == Speaker::Me && !normalized_words(&turn.text).is_empty())
}

/// Conversa social curta (cumprimento/agradecimento) não é oportunidade.
fn is_small_talk(block: &ConversationBlock) -> bool {
    let they_text = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::They)
        .map(|turn| turn.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let words = normalized_words(&they_text);
    if words.is_empty() || words.len() > 12 {
        return false;
    }
    const SOCIAL: [&str; 33] = [
        "bom", "boa", "dia", "tarde", "noite", "tudo", "bem", "ótimo", "otimo", "e", "você",
        "voce", "obrigado", "obrigada", "de", "nada", "até", "ate", "logo", "como", "vai", "how",
        "are", "you", "good", "morning", "thank", "thanks", "too", "nice", "to", "meet", "am",
    ];
    let substantive = words
        .iter()
        .filter(|word| !SOCIAL.contains(&word.as_str()))
        .count();
    substantive == 0
        && words.iter().any(|word| {
            matches!(
                word.as_str(),
                "tudo"
                    | "ótimo"
                    | "otimo"
                    | "obrigado"
                    | "obrigada"
                    | "você"
                    | "voce"
                    | "thanks"
                    | "thank"
                    | "how"
                    | "nice"
            )
        })
}

/// THEY convidou ME **explicitamente** a comentar/opinar.
///
/// Comentário não é “THEY expressou uma opinião”: é um convite claro para ME
/// falar. Por isso os marcadores fortes valem sozinhos; os fracos (concorda,
/// faz sentido, e você) só valem com pergunta.
fn invites_comment(text: &str) -> bool {
    const STRONG: [&str; 32] = [
        "o que você acha",
        "o que acha",
        "o que vocês acham",
        "qual sua opinião",
        "qual é a sua opinião",
        "sua opinião",
        "seu ponto de vista",
        "seu comentário",
        "algum comentário",
        "alguma consideração",
        "quer acrescentar",
        "pode comentar",
        "what do you think",
        "what's your take",
        "what is your take",
        "your thoughts",
        "your opinion",
        "any comments",
        "would you add",
        "how about you",
        "thoughts?",
        "agree?",
        "qual a sua leitura",
        "como você vê",
        "faz sentido para você",
        "faz sentido pra você",
        "faz sentido para vocês",
        "você concorda",
        "concorda com",
        "do you agree",
        "would you agree",
        "does that make sense to you",
    ];
    const WEAK: [&str; 5] = [
        "faz sentido",
        "concorda",
        "concordam",
        "e você",
        "does that make sense",
    ];
    let lower = text.to_lowercase();
    if STRONG.iter().any(|marker| lower.contains(marker)) {
        return true;
    }
    has_question_mark(text) && WEAK.iter().any(|marker| lower.contains(marker))
}

fn word_set(text: &str) -> std::collections::HashSet<String> {
    normalized_words(text)
        .into_iter()
        .filter(|word| word.chars().count() > 2)
        .collect()
}

fn jaccard(
    left: &std::collections::HashSet<String>,
    right: &std::collections::HashSet<String>,
) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count() as f32;
    let union = left.union(right).count() as f32;
    intersection / union
}

/// `true` quando a pergunta atual repete uma pergunta já respondida.
pub fn is_repeated_question(
    question: &str,
    prior_questions: &[PriorQuestion],
    config: &HeuristicConfig,
) -> bool {
    let current = word_set(question);
    if current.is_empty() {
        return false;
    }
    let normalized_current = normalized_words(question).join(" ");
    prior_questions.iter().any(|prior| {
        if normalized_words(&prior.title).join(" ") == normalized_current {
            return true;
        }
        let previous = word_set(&prior.title);
        jaccard(&current, &previous) >= config.repeated_question_similarity
    })
}

/// Backend heurístico local.
///
/// A decisão é binária: responder a uma pergunta (`question`) ou comentar um
/// ponto (`comment`). Perguntas repetidas viram `already_answered`; conversa
/// casual/explicação vira `no_action`; bloco curto vira
/// `insufficient_context`. Não há mais silence/hesitation.
pub fn heuristic_cue(
    block: &ConversationBlock,
    config: &HeuristicConfig,
    prior_questions: &[PriorQuestion],
) -> SuggestionCue {
    let total_words: usize = block
        .turns
        .iter()
        .map(|turn| normalized_words(&turn.text).len())
        .sum();

    if total_words < config.min_words {
        return SuggestionCue::InsufficientContext;
    }
    if is_small_talk(block) {
        return SuggestionCue::NoAction;
    }

    // Se ME é o último a falar, ele já está respondendo/conduzindo: não há
    // pergunta pendente de THEY para o copiloto responder. Sem silêncio ou
    // hesitação na taxonomia, isso é `no_action` (ou `already_answered` se a
    // pergunta de THEY veio antes da resposta de ME neste bloco).
    if block
        .turns
        .last()
        .is_some_and(|turn| turn.speaker == Speaker::Me)
    {
        return if answered_in_block(block) {
            SuggestionCue::AlreadyAnswered
        } else {
            SuggestionCue::NoAction
        };
    }

    let they_text = block
        .turns
        .iter()
        .filter(|turn| turn.speaker == Speaker::They)
        .map(|turn| turn.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let question = question_text(block);
    let invitation = invites_comment(&they_text);

    // A deduplicação vale para as duas cues acionáveis. O título guardado é a
    // pergunta (quando existe) ou a última fala de THEY (convite de comentário).
    if question.is_some() || invitation {
        if answered_in_block(block) {
            return SuggestionCue::AlreadyAnswered;
        }
        // Compara a pergunta isolada e o bloco inteiro de THEY: a pergunta pode
        // estar fragmentada em vários turnos ("... uma fila. Quais são...").
        let repeated = question
            .iter()
            .chain(std::iter::once(&they_text))
            .any(|candidate| is_repeated_question(candidate, prior_questions, config));
        if repeated {
            return SuggestionCue::AlreadyAnswered;
        }
    }

    // Convite a comentar/opinar vence pergunta retórica ("o que você acha?").
    if invitation {
        return SuggestionCue::Comment;
    }
    if question.is_some() {
        return SuggestionCue::Question;
    }
    SuggestionCue::NoAction
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
            SuggestionCue::from_str("invitation"),
            Some(SuggestionCue::Comment)
        );
        assert_eq!(
            SuggestionCue::from_str("no_suggestion"),
            Some(SuggestionCue::NoAction)
        );
        assert_eq!(SuggestionCue::from_str("nope"), None);
        assert!(SuggestionCue::Question.can_suggest());
        assert!(SuggestionCue::Comment.can_suggest());
        assert!(!SuggestionCue::AlreadyAnswered.can_suggest());
        assert!(!SuggestionCue::NoAction.can_suggest());
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
    fn they_target_keeps_only_they_turns_after_the_last_me() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "minha resposta", 0, 500),
            ("b", Speaker::They, "primeira pergunta?", 600, 900),
            ("c", Speaker::Me, "resposta", 1_000, 1_300),
            ("d", Speaker::They, "pergunta atual?", 1_400, 1_700),
        ]);
        let target = they_target(&block).expect("alvo");
        assert_eq!(target.turns.len(), 1);
        assert_eq!(target.turns[0].text, "pergunta atual?");
        assert_eq!(target.me_chars, 0);
        assert_eq!(target.id, block.id);

        let ends_me = block_with_turns(vec![
            ("e", Speaker::They, "pergunta?", 0, 300),
            ("f", Speaker::Me, "resposta", 400, 700),
        ]);
        assert!(they_target(&ends_me).is_none());
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

    fn prior(title: &str) -> PriorQuestion {
        PriorQuestion {
            title: title.to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1_000,
        }
    }

    #[test]
    fn heuristic_detects_question_pt_en() {
        let pt = block_with_turns(vec![
            ("a", Speaker::Me, "eu consigo explicar isso", 0, 500),
            ("b", Speaker::They, "pode explicar melhor?", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&pt, &heuristic_config(), &[]),
            SuggestionCue::Question
        );

        let en = block_with_turns(vec![
            ("a", Speaker::Me, "i can explain that", 0, 500),
            ("b", Speaker::They, "can you explain it?", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&en, &heuristic_config(), &[]),
            SuggestionCue::Question
        );

        let request = block_with_turns(vec![(
            "a",
            Speaker::They,
            "me explica como o cache invalida",
            0,
            1_500,
        )]);
        assert_eq!(
            heuristic_cue(&request, &heuristic_config(), &[]),
            SuggestionCue::Question
        );
    }

    #[test]
    fn greeting_does_not_hide_a_real_question() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "bom dia", 0, 500),
            (
                "b",
                Speaker::They,
                "bom dia, tudo bem? qual é o prazo da entrega?",
                1_000,
                1_800,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&block, &heuristic_config(), &[]),
            SuggestionCue::Question
        );
    }

    #[test]
    fn heuristic_detects_comment_invitation() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "podemos priorizar a migração", 0, 500),
            (
                "b",
                Speaker::They,
                "eu acho que o banco é o gargalo. o que você acha?",
                1_000,
                1_800,
            ),
        ]);
        // A pergunta é retórica/convite, não um pedido de resposta factual:
        // ainda assim é uma oportunidade de comentar.
        assert_eq!(
            heuristic_cue(&block, &heuristic_config(), &[]),
            SuggestionCue::Comment
        );

        let comment = block_with_turns(vec![
            ("a", Speaker::Me, "podemos priorizar a migração", 0, 500),
            (
                "b",
                Speaker::They,
                "eu acho que devemos cortar escopo para cumprir a data. concorda?",
                1_000,
                1_800,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&comment, &heuristic_config(), &[]),
            SuggestionCue::Comment
        );

        let invitation = block_with_turns(vec![
            ("a", Speaker::Me, "podemos priorizar a migração", 0, 500),
            (
                "b",
                Speaker::They,
                "eu acho que o banco é o gargalo. qual sua opinião?",
                1_000,
                1_800,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&invitation, &heuristic_config(), &[]),
            SuggestionCue::Comment
        );

        let statement = block_with_turns(vec![
            ("a", Speaker::Me, "podemos priorizar a migração", 0, 500),
            (
                "b",
                Speaker::They,
                "eu acho que o banco é o gargalo. faz sentido para você?",
                1_000,
                1_800,
            ),
        ]);
        // "faz sentido" é convite; a pergunta também tem "?".
        assert_eq!(
            heuristic_cue(&statement, &heuristic_config(), &[]),
            SuggestionCue::Comment
        );
    }

    #[test]
    fn heuristic_marks_a_repeated_question_as_already_answered() {
        let block = block_with_turns(vec![
            ("a", Speaker::Me, "o desconto é 10%", 0, 500),
            (
                "b",
                Speaker::They,
                "e o desconto para 50 licenças, como fica?",
                1_000,
                1_500,
            ),
        ]);
        let answered = vec![prior("e o desconto para 50 licenças, como fica?")];
        assert_eq!(
            heuristic_cue(&block, &heuristic_config(), &answered),
            SuggestionCue::AlreadyAnswered
        );

        let unrelated = vec![prior("qual é o prazo da segunda fase?")];
        assert_eq!(
            heuristic_cue(&block, &heuristic_config(), &unrelated),
            SuggestionCue::Question
        );
    }

    #[test]
    fn heuristic_marks_a_question_answered_in_the_same_block() {
        let block = block_with_turns(vec![
            ("a", Speaker::They, "qual é o prazo?", 0, 500),
            ("b", Speaker::Me, "entrego na sexta", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&block, &heuristic_config(), &[]),
            SuggestionCue::AlreadyAnswered
        );
    }

    #[test]
    fn repeated_comment_invitation_is_already_answered() {
        let first = block_with_turns(vec![
            (
                "a",
                Speaker::Me,
                "o mutex é um bloqueio, não uma fila",
                0,
                1_000,
            ),
            (
                "b",
                Speaker::They,
                "Você mencionou o mutex para controlar o acesso concorrente. Mas me diga, o mutex é a mesma coisa que uma fila.",
                1_200,
                3_000,
            ),
            (
                "c",
                Speaker::They,
                "Quais são as diferenças entre eles e em quais situações você utilizaria cada um?",
                3_500,
                4_000,
            ),
        ]);
        let title = question_title(&first);
        let prior = vec![PriorQuestion {
            title,
            block_id: first.id.clone(),
            answered_at_ms: 10_000,
        }];

        let second = block_with_turns(vec![
            (
                "d",
                Speaker::They,
                "Você mencionou o mutex para controlar o acesso concorrente. Mas me diga, o mutex é a mesma coisa que o mutex.",
                0,
                2_000,
            ),
            ("e", Speaker::Me, "não, ele é um bloqueio", 2_500, 3_000),
            (
                "f",
                Speaker::They,
                "uma fila. Quais são as diferenças entre eles e em quais situações você utilizaria cada um?",
                3_500,
                5_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&second, &heuristic_config(), &prior),
            SuggestionCue::AlreadyAnswered
        );
    }

    #[test]
    fn heuristic_returns_no_action_for_casual_or_explanation() {
        let casual = block_with_turns(vec![
            ("a", Speaker::Me, "bom dia, tudo bem?", 0, 500),
            ("b", Speaker::They, "tudo ótimo, e você?", 1_000, 1_500),
        ]);
        assert_eq!(
            heuristic_cue(&casual, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );

        let explanation = block_with_turns(vec![
            ("a", Speaker::Me, "como funciona?", 0, 500),
            (
                "b",
                Speaker::They,
                "o sistema usa filas para desacoplar os serviços e retenta falhas.",
                1_000,
                3_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&explanation, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );

        let me_only = block_with_turns(vec![(
            "a",
            Speaker::Me,
            "eu estou explicando o desenho da arquitetura para o time",
            0,
            3_000,
        )]);
        assert_eq!(
            heuristic_cue(&me_only, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );
    }

    #[test]
    fn me_last_turn_is_not_a_suggestion_opportunity() {
        let answer = block_with_turns(vec![
            ("a", Speaker::They, "o que é um mutex?", 0, 500),
            (
                "b",
                Speaker::Me,
                "é um mecanismo de exclusão mútua que protege a seção crítica",
                1_000,
                2_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&answer, &heuristic_config(), &[]),
            SuggestionCue::AlreadyAnswered
        );

        let me_only = block_with_turns(vec![(
            "c",
            Speaker::Me,
            "o mutex protege a seção crítica e evita condição de corrida",
            0,
            2_000,
        )]);
        assert_eq!(
            heuristic_cue(&me_only, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );
    }

    #[test]
    fn a_mere_statement_is_not_a_comment_invitation() {
        let statement = block_with_turns(vec![
            ("a", Speaker::Me, "podemos revisar o escopo", 0, 500),
            (
                "b",
                Speaker::They,
                "eu acho que o prazo está curto e o escopo deveria ser menor.",
                1_000,
                2_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&statement, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );

        let weak_without_question = block_with_turns(vec![
            ("c", Speaker::Me, "podemos revisar o escopo", 0, 500),
            (
                "d",
                Speaker::They,
                "eu acho que o prazo está curto. faz sentido.",
                1_000,
                2_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&weak_without_question, &heuristic_config(), &[]),
            SuggestionCue::NoAction
        );

        let explicit = block_with_turns(vec![
            ("e", Speaker::Me, "podemos revisar o escopo", 0, 500),
            (
                "f",
                Speaker::They,
                "eu acho que o prazo está curto. o que você acha",
                1_000,
                2_000,
            ),
        ]);
        assert_eq!(
            heuristic_cue(&explicit, &heuristic_config(), &[]),
            SuggestionCue::Comment
        );
    }

    #[test]
    fn heuristic_returns_insufficient_context_for_short_blocks() {
        let short = block_with_turns(vec![
            ("a", Speaker::Me, "oi", 0, 100),
            ("b", Speaker::They, "ok", 200, 300),
        ]);
        assert_eq!(
            heuristic_cue(&short, &heuristic_config(), &[]),
            SuggestionCue::InsufficientContext
        );
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
            cue: SuggestionCue::NoAction,
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
