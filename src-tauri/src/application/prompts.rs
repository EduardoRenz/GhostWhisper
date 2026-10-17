//! Prompts usados pelos casos de uso de LLM (insights e tradução).

use crate::domain::settings::language_name;

/// Prompt de sistema para geração de sugestões/insights.
pub fn insight_system_prompt(source_language: &str, target_language: &str) -> String {
    let source = language_name(source_language);
    let target = language_name(target_language);
    format!(
        "Você é o GhostWhisper, um copiloto de reunião. Com base na transcrição recente \
         (idioma de origem: {source}), sugira de forma concisa e acionável o \
         que o usuário deveria dizer ou fazer a seguir. Responda em {target}, \
         em no máximo 3 tópicos curtos. Não invente fatos ausentes da transcrição."
    )
}

/// Prompt de usuário com o contexto da transcrição.
pub fn insight_user_prompt(context: &str) -> String {
    format!("Transcrição recente:\n{context}\n\nSugira os próximos passos.")
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
    fn insight_prompts_mention_languages_and_context() {
        let system = insight_system_prompt("pt", "en");
        assert!(system.contains("português"));
        assert!(system.contains("inglês"));

        // Origem/destino "auto" → mesmo idioma da transcrição.
        let auto = insight_system_prompt("auto", "auto");
        assert!(auto.contains("o mesmo idioma da transcrição"));

        let user = insight_user_prompt("olá mundo");
        assert!(user.contains("olá mundo"));
    }

    #[test]
    fn translation_prompt_targets_language() {
        assert!(translation_system_prompt("en").contains("inglês"));
        assert!(translation_system_prompt("pt").contains("português"));
        assert_eq!(translation_user_prompt("bom dia"), "bom dia");
    }
}
