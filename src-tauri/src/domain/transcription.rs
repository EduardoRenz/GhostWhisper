//! Tipos de domínio para transcrição.

use serde::Serialize;

/// Quem falou: o usuário (`Me`) ou o outro lado (`They`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Speaker {
    #[serde(rename = "ME")]
    Me,
    #[serde(rename = "THEY")]
    They,
}

impl Speaker {
    /// Representação textual usada em eventos/DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Speaker::Me => "ME",
            Speaker::They => "THEY",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "ME" => Some(Speaker::Me),
            "THEY" => Some(Speaker::They),
            _ => None,
        }
    }
}

/// Fase de preparação do motor de transcrição.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AsrPhase {
    /// Baixando/carregando o modelo (minutos em modelos grandes).
    Preparing,
    /// Modelo carregado e pronto para transcrever.
    Ready,
    /// Falha ao preparar o motor.
    Failed,
}

/// Status do motor de transcrição, publicado para dar retorno ao usuário
/// enquanto o modelo é baixado/carregado. Trocar para um modelo maior leva
/// minutos e, antes, era totalmente silencioso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AsrStatus {
    pub phase: AsrPhase,
    pub model: String,
    pub message: String,
}

impl AsrStatus {
    /// Cria um status para o modelo informado.
    pub fn new(phase: AsrPhase, model: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            phase,
            model: model.into(),
            message: message.into(),
        }
    }
}

/// Um segmento de fala transcrito.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub session_id: String,
    pub speaker: Speaker,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    #[serde(rename = "final")]
    pub is_final: bool,
}

/// Texto transcrito pelo ASR (tempos em ms relativos ao trecho enviado).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscribedSpan {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// Qualidade esperada da decodificação.
///
/// - [`Preview`](SpeechQuality::Preview): passagem rápida durante a fala, com
///   custo baixo e resultado **revizável** (pode melhorar no passe final).
/// - [`Final`](SpeechQuality::Final): passe autoritativo sobre o turno inteiro,
///   com busca mais cara (`best_of`/`beam_size` maiores) e resultado estável.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechQuality {
    Preview,
    Final,
}

// --- Filtro de alucinações do Whisper ------------------------------------
//
// O whisper.cpp notoriously "alucina" texto plausível quando decodifica
// silêncio/ruído (frases como "thanks for watching", "[BLANK_AUDIO]", loops de
// uma mesma palavra). These heuristics run **before** anything is shown or
// persisted, mirroring the mature WhisperSessionSupport approach.

/// Padrões de alucinação conhecidos (frases-modelo do whisper.cpp).
const HALLUCINATION_PATTERNS: [&str; 10] = [
    "thank you for watching",
    "thanks for watching",
    "thank you for listening",
    "thanks for listening",
    "please subscribe",
    "subscribe to my channel",
    "subtitles by",
    "copyright",
    "transcription by",
    "amara.org",
];

/// Indica se o texto parece uma alucinação do Whisper (não fala real).
pub fn is_likely_hallucination(text: &str) -> bool {
    let normalized = normalize_for_match(text);
    if normalized.is_empty() {
        // Só pontuação/ruído não é fala.
        return true;
    }
    // Frase-modelo explícita.
    if HALLUCINATION_PATTERNS
        .iter()
        .any(|pattern| normalized.contains(pattern))
    {
        return true;
    }
    // Marcadores de silêncio/brackets (checados no texto original).
    let trimmed = text.trim();
    if trimmed.starts_with('[') || trimmed.starts_with('(') {
        return true;
    }
    // Só símbolos/notas (sem nenhuma letra).
    if !normalized.chars().any(|c| c.is_alphabetic()) {
        return true;
    }
    // Loop obvious: mesma palavra repetida ≥4× seguidas.
    if has_excessive_adjacent_repeats(&normalized) {
        return true;
    }
    false
}

/// Minúsculas, sem pontuação, espaços colapsados (para comparação robusta).
fn normalize_for_match(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Verdadeiro se há uma palavra repetida ≥4× seguidas ("a a a a").
fn has_excessive_adjacent_repeats(normalized: &str) -> bool {
    let words: Vec<&str> = normalized.split(' ').filter(|w| !w.is_empty()).collect();
    if words.len() < 4 {
        return false;
    }
    let mut run = 1;
    for i in 1..words.len() {
        if words[i] == words[i - 1] {
            run += 1;
        } else {
            run = 1;
        }
        if run >= 4 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_quality_variants() {
        assert_eq!(SpeechQuality::Preview, SpeechQuality::Preview);
        assert_ne!(SpeechQuality::Preview, SpeechQuality::Final);
    }

    #[test]
    fn rejects_known_hallucination_phrases() {
        assert!(is_likely_hallucination("Thanks for watching!"));
        assert!(is_likely_hallucination("Please subscribe to my channel"));
        assert!(is_likely_hallucination("[BLANK_AUDIO]"));
        assert!(is_likely_hallucination("(silence)"));
    }

    #[test]
    fn rejects_loops_and_symbols() {
        assert!(is_likely_hallucination("a a a a a"));
        assert!(is_likely_hallucination("♪♪♪"));
        assert!(is_likely_hallucination("..."));
        assert!(is_likely_hallucination("   "));
    }

    #[test]
    fn accepts_real_speech() {
        assert!(!is_likely_hallucination("vamos fechar o documento agora"));
        assert!(!is_likely_hallucination("The quarterly numbers are up"));
        // Uma palavra repetida 2–3× pode ser fala natural.
        assert!(!is_likely_hallucination("very very good"));
    }
}
