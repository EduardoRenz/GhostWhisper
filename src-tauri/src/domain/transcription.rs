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
    // Loop de **frase**: a mesma sentença repetida até virar o texto (ISS-019).
    if has_repeated_phrase_loop(&normalized) {
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

// --- Loop de frase do Whisper (ISS-019) -------------------------------------
//
// O whisper.cpp só descarta uma sequência repetitiva quando ela passa de 32
// tokens **e** a entropia média fica abaixo de `entropy_thold`
// (`whisper.cpp`: `if (decoder.sequence.result_len > 32 && decoder.sequence.entropy
// < params.entropy_thold) decoder.failed = true;`). Com `entropy_thold = 2.2` e
// `temperature_inc = 0` — a configuração do Perssua, que espelhamos — a repetição
// "confiante" morre, mas a **repetição de frase com entropia acima do limiar**
// passa e chega inteira na tela ("O que eu quero fazer?" ×15). Daí este filtro
// determinístico, que não depende do modelo ter se arrependido.

/// Nº máximo de palavras da frase repetida (Perssua: 5).
const LOOP_MAX_PHRASE_WORDS: usize = 5;
/// Nº máximo de palavras no corte do loop no **fim** do texto.
///
/// Maior que o do Perssua de propósito: aqui o loop termina o texto e exige ≥3
/// repetições exatas, então uma frase longa (a sentença inteira repetida) também
/// pode ser cortada sem risco de falso positivo.
const LOOP_MAX_TRAIL_PHRASE_WORDS: usize = 12;
/// Nº mínimo de palavras do texto para procurar loop (Perssua: 6).
const LOOP_MIN_WORDS: usize = 6;
/// Repetições mínimas da mesma frase para virar loop (Perssua: 3).
const LOOP_MIN_REPEATS: usize = 3;
/// Cobertura mínima do texto pela repetição (Perssua: 0.7).
const LOOP_MIN_COVERAGE: f64 = 0.7;

/// Verdadeiro se o texto **inteiro** é a mesma frase repetida ("O que eu quero
/// fazer? O que eu quero fazer? …").
///
/// Transposto de `h` do Perssua (`build/static/js/22.75b118cd.chunk.js`): a frase
/// (até 5 palavras) repetida ≥3× seguidas a partir do início e cobrindo ≥70% do
/// texto. O Perssua só aplica isso ao provider Gemini; aplicamos a todos porque o
/// whisper local repete assim (ISS-019) e a guarda do whisper.cpp não pega este
/// caso.
pub fn has_repeated_phrase_loop(text: &str) -> bool {
    let words: Vec<&str> = text.split(' ').filter(|word| !word.is_empty()).collect();
    if words.len() < LOOP_MIN_WORDS {
        return false;
    }
    let max_phrase = LOOP_MAX_PHRASE_WORDS.min(words.len() / LOOP_MIN_REPEATS);
    for phrase_len in 1..=max_phrase {
        let phrase = &words[..phrase_len];
        let mut consumed = 0;
        while consumed + phrase_len <= words.len()
            && &words[consumed..consumed + phrase_len] == phrase
        {
            consumed += phrase_len;
        }
        let repeats = consumed / phrase_len;
        let coverage = consumed as f64 / words.len() as f64;
        if repeats >= LOOP_MIN_REPEATS && coverage >= LOOP_MIN_COVERAGE {
            return true;
        }
    }
    false
}

/// Corta um loop que **termina** o texto, devolvendo a fala sem a repetição.
///
/// É o complemento do [`has_repeated_phrase_loop`]: quando o modelo degenera no
/// meio do turno ("bom dia pessoal" + "O que eu quero fazer?" ×15), o trecho
/// repetido encolhe para **uma** ocorrência — o turno passou pelo portão de fala
/// sustentada, então houve fala de verdade ali e devolver vazio perderia a fala
/// (**desvio consciente**: o Perssua devolve `null`). Loop de **uma palavra só** (o
/// clássico "a a a a") é descartado inteiro: palavra repetida é alucinação, não
/// fala.
pub fn strip_phrase_loop(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < LOOP_MIN_WORDS {
        return text.trim().to_string();
    }
    let Some(loop_) = trailing_loop(&words) else {
        return words.join(" ");
    };
    let keep = if loop_.phrase_len == 1 {
        loop_.start
    } else {
        loop_.start + loop_.phrase_len
    };
    words[..keep].join(" ")
}

/// Loop que vai até o fim do texto: onde começa e o tamanho da frase repetida.
struct PhraseLoop {
    start: usize,
    phrase_len: usize,
}

/// Encontra um loop que vai até o fim do texto.
///
/// Devolve o início **mais cedo** possível, para o corte ser o mais generoso
/// (mantém o máximo de fala real). Exige ≥3 repetições, frase de até 12 palavras e
/// pelo menos 6 palavras de repetição — "não não não" ou "muito muito muito bom"
/// não são loop.
fn trailing_loop(words: &[&str]) -> Option<PhraseLoop> {
    let max_phrase = LOOP_MAX_TRAIL_PHRASE_WORDS
        .min(words.len() / LOOP_MIN_REPEATS)
        .max(1);
    for start in 0..words.len() {
        let region = &words[start..];
        if region.len() < LOOP_MIN_WORDS {
            break;
        }
        for phrase_len in 1..=max_phrase {
            if region.len() % phrase_len != 0 {
                continue;
            }
            let repeats = region.len() / phrase_len;
            if repeats < LOOP_MIN_REPEATS {
                continue;
            }
            let phrase: Vec<String> = region[..phrase_len]
                .iter()
                .map(|word| normalize_token(word))
                .collect();
            let all_equal = region.chunks(phrase_len).all(|block| {
                block
                    .iter()
                    .map(|word| normalize_token(word))
                    .eq(phrase.iter().cloned())
            });
            if all_equal {
                return Some(PhraseLoop { start, phrase_len });
            }
        }
    }
    None
}

/// Palavra sem pontuação e em minúsculas (para comparar repetições).
fn normalize_token(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
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
    fn rejects_a_repeated_phrase_loop() {
        // O caso relatado pelo usuário (ISS-019): a mesma frase repetida.
        let looped = "O que eu quero fazer? ".repeat(15);
        assert!(is_likely_hallucination(&looped));
        assert!(has_repeated_phrase_loop(&looped));
        // Três repetições já bastam (Perssua: ≥3 e ≥70% de cobertura).
        assert!(has_repeated_phrase_loop(
            "que dia é hoje que dia é hoje que dia é hoje"
        ));
        // Fala normal com repetição natural não é loop de frase.
        assert!(!has_repeated_phrase_loop("vamos fechar o documento agora"));
        assert!(!has_repeated_phrase_loop("very very good"));
    }

    #[test]
    fn phrase_loop_detection_keeps_the_same_guards_as_perssua() {
        // Menos de 6 palavras não é loop (Perssua: `t.length < 6`).
        assert!(!has_repeated_phrase_loop("oi oi oi"));
        // A repetição precisa cobrir ≥70% do texto: fala longa + 3 repetições no
        // fim não é "texto que é loop" (mas é cortada por `strip_phrase_loop`).
        assert!(!has_repeated_phrase_loop(
            "bom dia pessoal, tudo bem com vocês hoje? oi oi oi"
        ));
    }

    #[test]
    fn strip_phrase_loop_keeps_one_occurrence_of_the_repeated_phrase() {
        // O caso relatado: 15 repetições viram **uma** (a fala existiu, o loop não).
        assert_eq!(
            strip_phrase_loop(&"O que eu quero fazer? ".repeat(15)),
            "O que eu quero fazer?"
        );
        // Fala real + loop: a fala fica e a frase repetida aparece uma vez.
        assert_eq!(
            strip_phrase_loop(
                "bom dia pessoal O que eu quero fazer? O que eu quero fazer? O que eu quero fazer?"
            ),
            "bom dia pessoal O que eu quero fazer?"
        );
        // Frase repetida no fim de um texto maior.
        assert_eq!(
            strip_phrase_loop("então vamos revisar o orçamento vamos revisar o orçamento vamos revisar o orçamento"),
            "então vamos revisar o orçamento"
        );
        // Frase longa (a sentença inteira) repetida também encolhe.
        assert_eq!(
            strip_phrase_loop(
                "eu quero fechar o documento amanhã pela manhã eu quero fechar o documento amanhã pela manhã eu quero fechar o documento amanhã pela manhã"
            ),
            "eu quero fechar o documento amanhã pela manhã"
        );
    }

    #[test]
    fn strip_phrase_loop_drops_a_single_word_loop() {
        // "a a a a a" é alucinação, não fala: não vira "a" na conversa.
        assert_eq!(strip_phrase_loop("a a a a a a a"), "");
        assert_eq!(
            strip_phrase_loop("bom dia pessoal a a a a a a"),
            "bom dia pessoal"
        );
    }

    #[test]
    fn strip_phrase_loop_collapses_a_repeated_short_phrase() {
        // "tá bom tá bom tá bom tá bom" é fala plausível: encolhe, não desaparece.
        assert_eq!(strip_phrase_loop("tá bom tá bom tá bom tá bom"), "tá bom");
    }

    #[test]
    fn strip_phrase_loop_keeps_natural_speech() {
        // Ênfase natural com poucas palavras não é loop.
        assert_eq!(strip_phrase_loop("não não não"), "não não não");
        assert_eq!(
            strip_phrase_loop("muito muito muito bom"),
            "muito muito muito bom"
        );
        assert_eq!(
            strip_phrase_loop("vamos fechar o documento agora"),
            "vamos fechar o documento agora"
        );
    }

    #[test]
    fn accepts_real_speech() {
        assert!(!is_likely_hallucination("vamos fechar o documento agora"));
        assert!(!is_likely_hallucination("The quarterly numbers are up"));
        // Uma palavra repetida 2–3× pode ser fala natural.
        assert!(!is_likely_hallucination("very very good"));
    }
}
