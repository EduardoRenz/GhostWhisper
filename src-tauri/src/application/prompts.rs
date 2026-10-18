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
}
