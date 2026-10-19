//! Prompts usados pelos casos de uso de LLM (insights e tradução).

use crate::domain::settings::language_name;

pub mod assistant;
pub mod summary;

/// Normaliza um Markdown vindo do modelo: `trim` e remoção de cerca externa
/// (```markdown … ```) quando ela envolve **todo** o texto.
pub fn normalize_markdown(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") || !trimmed.trim_end().ends_with("```") {
        return trimmed.to_string();
    }
    let mut lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() < 2 {
        return trimmed.to_string();
    }
    lines.remove(0);
    lines.pop();
    lines.join("\n").trim().to_string()
}

/// Separa o título (`Subject:`) do corpo de uma sugestão.
///
/// Retorno **estruturado leve**, o mesmo padrão do resumo: o agente começa a
/// resposta com uma linha `Subject: <título curto>`; o corpo é o Markdown sem
/// essa linha. Uma linha parcial (`Subj…`) durante o streaming também sai do
/// corpo, para o texto não piscar.
///
/// Devolve `(Option<título>, corpo normalizado)`.
pub fn parse_suggestion_response(text: &str) -> (Option<String>, String) {
    let mut subject: Option<String> = None;
    let mut body: Vec<&str> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        let before_body = index == 0 || body.iter().all(|item| item.trim().is_empty());
        if before_body && subject.is_none() {
            // `Subject: …`, `**Subject:** …`, `SUBJECT: …`.
            if let Some(colon) = trimmed.find(':') {
                let head = trimmed[..colon].trim_matches('*').trim().to_lowercase();
                if head == "subject" {
                    let value = trimmed[colon + 1..]
                        .trim()
                        .trim_matches('*')
                        .trim()
                        .trim_matches('"')
                        .trim();
                    if !value.is_empty() {
                        subject = Some(value.to_string());
                    }
                    continue;
                }
            }
            // Linha ainda incompleta do título (streaming): não mostra no corpo.
            if "subject:".starts_with(&lower) && lower.len() < "subject:".len() {
                continue;
            }
        }
        body.push(line);
    }

    (subject, normalize_markdown(&body.join("\n")))
}

/// Regra de idioma para as respostas do assistente.
///
/// `Some("pt")`/`Some("en")` fixa o idioma; `None`/`auto` pede para detectar o
/// idioma dominante e traduzir os títulos de seção.
pub fn output_language_rule(language: Option<&str>) -> String {
    match language
        .map(str::trim)
        .filter(|code| !code.is_empty() && *code != "auto")
    {
        Some(code) => format!("Write your entire response in {}.", language_name(code)),
        None => "Detect the dominant language of the conversation and write your entire response in that language; translate section titles into it.".to_string(),
    }
}

/// Prompt de sistema para tradução de um segmento.
pub fn translation_system_prompt(target_language: &str) -> String {
    let target = language_name(target_language);
    format!(
        "Traduza o texto do usuário para {target}. Responda apenas com a \
         tradução, sem comentários, aspas ou o texto original."
    )
}

/// Prompt de usuário com o texto a traduzir.
pub fn translation_user_prompt(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_prompt_targets_language() {
        assert!(translation_system_prompt("en").contains("inglês"));
        assert!(translation_system_prompt("pt").contains("português"));
        assert_eq!(translation_user_prompt("bom dia"), "bom dia");
    }

    #[test]
    fn parses_the_subject_line() {
        let (subject, body) = parse_suggestion_response(
            "Subject: Como inverter uma árvore binária em Python\n\n## Easy\n\nEu faria assim.",
        );
        assert_eq!(
            subject.as_deref(),
            Some("Como inverter uma árvore binária em Python")
        );
        assert!(body.starts_with("## Easy"));
        assert!(!body.contains("Subject:"));
    }

    #[test]
    fn accepts_bold_and_uppercase_subject() {
        let (subject, body) = parse_suggestion_response("**SUBJECT:** Sliding window\n\ntexto");
        assert_eq!(subject.as_deref(), Some("Sliding window"));
        assert_eq!(body, "texto");
    }

    #[test]
    fn handles_a_partial_subject_line_while_streaming() {
        for partial in ["S", "Sub", "Subj", "Subjec", "Subject"] {
            let (subject, body) = parse_suggestion_response(partial);
            assert!(subject.is_none(), "título inesperado em {partial}");
            assert!(body.is_empty(), "corpo inesperado em {partial}");
        }
    }

    #[test]
    fn keeps_text_without_a_subject() {
        let (subject, body) = parse_suggestion_response("> peça o prazo");
        assert!(subject.is_none());
        assert_eq!(body, "> peça o prazo");
    }
}
