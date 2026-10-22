//! Trailer de **resumo** na própria resposta (BR-053).
//!
//! Pedir um resumo numa chamada separada custaria mais uma ida ao provider a
//! cada pergunta. Em vez disso, o modelo termina a resposta com um bloco
//! combinado:
//!
//! ```text
//! <!--resumo: adotar arquitetura orientada a eventos; idempotência no gateway-->
//! ```
//!
//! O backend **remove** o bloco do texto exibido/persistido (inclusive
//! enquanto ele chega pelo streaming) e guarda o resumo na entrada do diário,
//! para o contexto das próximas perguntas.
//!
//! O bloco é um comentário HTML: se escapar para a UI antes do filtro, o
//! `MarkdownView` não o renderiza.

/// Abertura do bloco de resumo (no **início de uma linha**).
pub const SUMMARY_OPEN: &str = "<!--resumo:";

/// Fecho do bloco de resumo.
pub const SUMMARY_CLOSE: &str = "-->";

/// Teto do resumo guardado (caracteres).
pub const SUMMARY_MAX_CHARS: usize = 240;

/// Índice do bloco de resumo.
///
/// Vale o marcador numa **linha própria** (o formato pedido). Se o modelo
/// colar o bloco no fim da última frase, ele ainda conta **desde que feche o
/// texto** — o que evita cortar uma resposta que só *menciona* o marcador no
/// meio (ex.: explicando este formato).
fn marker_index(text: &str) -> Option<usize> {
    let at_line_start = text.match_indices(SUMMARY_OPEN).find_map(|(index, _)| {
        let before = &text[..index];
        (before.is_empty() || before.ends_with('\n')).then_some(index)
    });
    if at_line_start.is_some() {
        return at_line_start;
    }

    let index = text.rfind(SUMMARY_OPEN)?;
    let rest = &text[index + SUMMARY_OPEN.len()..];
    let end = rest.find(SUMMARY_CLOSE)?;
    let after = &rest[end + SUMMARY_CLOSE.len()..];
    after.trim().is_empty().then_some(index)
}

/// Quantos caracteres do fim podem ser o **começo** do marcador.
///
/// O streaming não pode piscar um `<!--res` na tela: segura esse sufixo até o
/// próximo delta decidir se é (ou não) o bloco.
fn held_back(text: &str) -> usize {
    // Olha o **fim do texto**, não só a última linha: o modelo pode colar o
    // bloco no fim da frase.
    let limit = text.len().min(SUMMARY_OPEN.len() - 1);
    for size in (1..=limit).rev() {
        if text.ends_with(&SUMMARY_OPEN[..size]) {
            return size;
        }
    }
    0
}

/// Texto **visível** enquanto a resposta chega (sem o trailer, mesmo parcial).
///
/// É monotônico: o texto emitido só cresce — o sufixo ambíguo fica retido.
pub fn visible_stream(text: &str) -> &str {
    match marker_index(text) {
        Some(index) => &text[..index],
        None => {
            let hold = held_back(text);
            &text[..text.len() - hold]
        }
    }
}

/// Resposta final: sem o trailer e sem o sufixo retido pelo streaming.
pub fn without_summary(text: &str) -> &str {
    match marker_index(text) {
        Some(index) => text[..index].trim_end(),
        None => text.trim_end(),
    }
}

/// Resumo **completo** (sem o marcador), se o modelo o mandou.
pub fn extract(text: &str) -> Option<String> {
    let index = marker_index(text)?;
    let rest = &text[index + SUMMARY_OPEN.len()..];
    let end = rest.find(SUMMARY_CLOSE)?;
    let summary = rest[..end].trim();
    if summary.is_empty() {
        return None;
    }
    Some(truncate(summary, SUMMARY_MAX_CHARS))
}

/// Resumo determinístico de reserva (o modelo não mandou o bloco).
///
/// Junta a pergunta (curta) com o começo da resposta, numa linha só.
pub fn digest(question: &str, answer: &str) -> String {
    let question = truncate(&single_line(question), 160);
    let answer = truncate(&single_line(answer), 300);
    match (question.is_empty(), answer.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("R: {answer}"),
        (false, true) => format!("P: {question}"),
        (false, false) => format!("P: {question} / R: {answer}"),
    }
}

/// Colapsa quebras de linha/espaços numa linha só.
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Trunca sem quebrar caractere UTF-8.
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAIL: &str = "<!--resumo: arquitetura orientada a eventos-->";

    fn raw() -> String {
        format!("Adote eventos com buffer.\n\n- idempotência\n{TAIL}")
    }

    #[test]
    fn extracts_the_summary_and_hides_it_from_the_answer() {
        let text = raw();
        assert_eq!(
            extract(&text).as_deref(),
            Some("arquitetura orientada a eventos")
        );
        assert_eq!(
            without_summary(&text),
            "Adote eventos com buffer.\n\n- idempotência"
        );
    }

    /// Prefixos em **fronteiras de caractere** (o texto tem acentos).
    fn prefixes(text: &str) -> Vec<&str> {
        text.char_indices()
            .map(|(index, _)| &text[..index])
            .chain(std::iter::once(text))
            .collect()
    }

    #[test]
    fn a_streaming_prefix_never_shows_the_marker() {
        let text = raw();
        // De qualquer prefixo do texto, o visível nunca contém o marcador.
        for prefix in prefixes(&text) {
            let visible = visible_stream(prefix);
            assert!(
                !visible.contains(SUMMARY_OPEN),
                "vazou em {prefix:?}: {visible:?}"
            );
            assert!(!visible.contains("<!--res"), "sufixo ambíguo em {prefix:?}");
        }
    }

    #[test]
    fn the_visible_text_never_shrinks() {
        let text = raw();
        let mut last = String::new();
        for prefix in prefixes(&text) {
            let visible = visible_stream(prefix).to_string();
            assert!(
                visible.starts_with(&last),
                "encolheu: {last:?} → {visible:?}"
            );
            last = visible;
        }
    }

    #[test]
    fn a_partial_marker_is_held_until_it_is_decided() {
        assert_eq!(visible_stream("Olá <"), "Olá ");
        // Colado no fim da frase, o começo do bloco também fica retido.
        assert_eq!(visible_stream("Olá.<!--res"), "Olá.");
        assert_eq!(visible_stream("Olá\n<!--res"), "Olá\n");
        // O bloco só vale começando uma linha.
        assert_eq!(visible_stream("Olá\n<!--resumo: x-->"), "Olá\n");
        // Não é o marcador: o texto volta a fluir.
        assert_eq!(visible_stream("Olá <b>"), "Olá <b>");
    }

    #[test]
    fn without_summary_keeps_a_legitimate_angle_bracket() {
        // Sem marcador, nada é retido no texto final.
        assert_eq!(without_summary("a < b"), "a < b");
        assert_eq!(without_summary("texto   "), "texto");
    }

    #[test]
    fn a_marker_in_the_middle_of_the_answer_is_not_the_trailer() {
        let text = "falei de <!--resumo: dentro--> da frase";
        assert_eq!(extract(text), None);
        assert_eq!(without_summary(text), text);
    }

    #[test]
    fn a_block_glued_to_the_last_sentence_still_counts() {
        // O modelo colou o bloco sem quebrar linha: ele **fecha** o texto.
        let text = "Use eventos.<!--resumo: eventos-->";
        assert_eq!(extract(text).as_deref(), Some("eventos"));
        assert_eq!(without_summary(text), "Use eventos.");
    }

    #[test]
    fn an_incomplete_block_falls_back() {
        assert_eq!(extract("resposta\n<!--resumo: sem fecho"), None);
        assert_eq!(
            without_summary("resposta\n<!--resumo: sem fecho"),
            "resposta"
        );
    }

    #[test]
    fn caps_a_summary_that_repeats_the_answer() {
        let long = "x".repeat(SUMMARY_MAX_CHARS + 50);
        let text = format!("ok\n<!--resumo: {long}-->");
        let summary = extract(&text).expect("resumo");
        assert_eq!(summary.chars().count(), SUMMARY_MAX_CHARS);
    }

    #[test]
    fn digest_uses_the_question_and_the_answer() {
        let text = digest("Como otimizar?", "Use eventos.\n\n- buffer\n- idempotência");
        assert_eq!(
            text,
            "P: Como otimizar? / R: Use eventos. - buffer - idempotência"
        );
        assert_eq!(digest("", "só resposta"), "R: só resposta");
        assert_eq!(digest("só pergunta", "  "), "P: só pergunta");
        assert_eq!(digest("", ""), "");
    }
}
