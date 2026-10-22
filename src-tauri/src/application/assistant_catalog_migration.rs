//! Semeadura **única** dos agentes de fábrica (BR-075).
//!
//! O agente **Padrão** é protegido (BR-074) e vive no core. Os demais agentes
//! de fábrica (Reunião, Entrevista Técnica, LeetCode, System Design) são
//! **materializados** na lista do usuário uma única vez: a partir daí são
//! agentes dele — podem ser editados e excluídos de vez.
//!
//! A migração é **idempotente e única** (marca `assistant_modes_seeded`):
//! depois de ligada, um agente excluído não volta no próximo boot.
//!
//! Também remove, dos agentes de fábrica já semeados, as linhas que passaram a
//! ser **gerais** no `TRANSCRIPT_CONTRACT` (BR-082) — preservando o resto do
//! texto e as edições do usuário.

use std::sync::Arc;

use crate::application::prompts::assistant::factory_modes;
use crate::domain::settings::Settings;
use crate::error::Result;
use crate::ports::settings_store::SettingsStore;

/// Trecho antigo do prompt de Reunião: o contrato de transcrição que agora é
/// comum a todos os agentes (BR-082).
const LEGACY_MEETING_TRANSCRIPT_BLOCK: &str = r#"## Transcript contract

- ME is the application user who reads and speaks; THEY is the other participant (meeting audio, the other side of the call, or an external speaker).
- Accepted lines: "ME: text" and "THEY: text".
- Use THEY lines to decide what ME should answer, ask, clarify, confirm, challenge, or propose next.
- A direct question or request from THEY is always the highest-priority context.
- Match the dominant language and tone of the meeting. Do not repeat words ME already said."#;

/// Bloco novo de Reunião (o que sobra depois de tirar o contrato comum).
const NEW_MEETING_RULES_BLOCK: &str = r#"## Meeting rules

- Use THEY lines to decide what ME should answer, ask, clarify, confirm, challenge, or propose next.
- Match the tone of the meeting. Do not repeat words ME already said."#;

/// Linha antiga dos agentes técnicos.
const LEGACY_INTERVIEW_ME_LINE: &str =
    "- ME is the user who reads and speaks; THEY is the interviewer.\n";

/// Linha antiga de Entrevista Técnica.
const LEGACY_TECHNICAL_THEY_LINE: &str = "- Use THEY lines to answer what was asked. A direct technical question or request is always the highest-priority context.\n";

/// Linha antiga de Reunião: a proibição de exibir tags agora é comum.
const LEGACY_MEETING_TAG_LINE: &str = "- Never display ME/THEY tags in the output. Refer to ME in the first person and to THEY by name when known.";
const NEW_MEETING_TAG_LINE: &str =
    "- Refer to ME in the first person and to THEY by name when known.";

/// Prefixo antigo do system prompt de Reunião.
const LEGACY_MEETING_SYSTEM_PREFIX: &str =
    "Treat ME/THEY lines as participant speech and the source of truth";
const NEW_MEETING_SYSTEM_PREFIX: &str = "Treat participant speech as the source of truth";

/// `true` para os ids dos agentes de fábrica (os únicos que a normalização
/// toca; agentes personalizados ficam intactos).
fn is_factory_id(id: &str) -> bool {
    factory_modes().iter().any(|mode| mode.id == id)
}

/// Tira dos prompts de fábrica as linhas que agora vivem no contrato comum.
///
/// A substituição é textual e **idempotente**: só mexe nos trechos conhecidos
/// da semente antiga; o restante do prompt (inclusive edições do usuário) fica
/// como está.
fn normalize_legacy_prompts(settings: &mut Settings) -> bool {
    let mut changed = false;

    for mode in &mut settings.custom_assistant_modes {
        if !is_factory_id(&mode.id) {
            continue;
        }

        let normalized_suggestion = mode
            .suggestion_prompt
            .replace(LEGACY_MEETING_TRANSCRIPT_BLOCK, NEW_MEETING_RULES_BLOCK)
            .replace(LEGACY_INTERVIEW_ME_LINE, "")
            .replace(LEGACY_TECHNICAL_THEY_LINE, "")
            .replace(LEGACY_MEETING_TAG_LINE, NEW_MEETING_TAG_LINE);
        if normalized_suggestion != mode.suggestion_prompt {
            mode.suggestion_prompt = normalized_suggestion;
            changed = true;
        }

        let normalized_system = mode
            .system_prompt
            .replace(LEGACY_MEETING_SYSTEM_PREFIX, NEW_MEETING_SYSTEM_PREFIX);
        if normalized_system != mode.system_prompt {
            mode.system_prompt = normalized_system;
            changed = true;
        }
    }

    changed
}

/// Resultado da semeadura (para o log do boot).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SeedOutcome {
    /// Ids dos agentes de fábrica materializados nesta execução.
    pub added: Vec<String>,
    /// A lista já está semeada.
    pub seeded: bool,
}

/// Materializa os agentes de fábrica na lista do usuário (uma vez só).
pub struct AssistantCatalogMigration {
    settings: Arc<dyn SettingsStore>,
}

impl AssistantCatalogMigration {
    /// Cria a migração com a porta resolvida.
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// Executa a semeadura (idempotente).
    pub fn seed(&self) -> Result<SeedOutcome> {
        let mut settings = self.settings.load()?;
        let normalized = normalize_legacy_prompts(&mut settings);
        if settings.assistant_modes_seeded {
            if normalized {
                self.settings.save(&settings)?;
            }
            return Ok(SeedOutcome {
                added: Vec::new(),
                seeded: true,
            });
        }

        let mut added = Vec::new();
        for mode in factory_modes() {
            if !settings
                .custom_assistant_modes
                .iter()
                .any(|existing| existing.id == mode.id)
            {
                added.push(mode.id.clone());
                settings.custom_assistant_modes.push(mode);
            }
        }
        settings.assistant_modes_seeded = true;
        self.settings.save(&settings)?;
        Ok(SeedOutcome {
            added,
            seeded: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::assistant::AssistantMode;
    use crate::domain::settings::Settings;

    fn store_with(settings: Settings) -> Arc<InMemorySettingsStore> {
        Arc::new(InMemorySettingsStore::with_settings(settings))
    }

    fn agent(id: &str, name: &str) -> AssistantMode {
        AssistantMode {
            id: id.into(),
            name: name.into(),
            category: "custom".into(),
            system_prompt: "Você ajuda.".into(),
            suggestion_prompt: "Sugira.".into(),
            summary_prompt: String::new(),
            context: String::new(),
            icon: "sparkle".into(),
            color: "slate".into(),
            source_id: None,
        }
    }

    #[test]
    fn seeds_the_four_factory_agents_once() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());

        let outcome = migration.seed().expect("semeia");
        assert!(outcome.seeded);
        assert_eq!(
            outcome.added,
            vec![
                "meeting",
                "technical_interview",
                "leetcode",
                "system_design"
            ]
        );

        let settings = store.load().expect("lê");
        assert!(settings.assistant_modes_seeded);
        assert_eq!(settings.custom_assistant_modes.len(), 4);
    }

    #[test]
    fn seeding_twice_changes_nothing() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());

        migration.seed().expect("primeira");
        let after_first = store.load().expect("lê");

        let outcome = migration.seed().expect("segunda");
        assert!(outcome.added.is_empty());
        assert_eq!(after_first, store.load().expect("lê"));
    }

    #[test]
    fn never_overwrites_a_user_agent_with_the_same_id() {
        let settings = Settings {
            custom_assistant_modes: vec![agent("meeting", "Reunião da empresa")],
            ..Settings::default()
        };
        let store = store_with(settings);
        let migration = AssistantCatalogMigration::new(store.clone());

        let outcome = migration.seed().expect("semeia");
        assert!(!outcome.added.contains(&"meeting".to_string()));

        let settings = store.load().expect("lê");
        let meeting = settings
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "meeting")
            .expect("reunião");
        assert_eq!(meeting.name, "Reunião da empresa");
        assert_eq!(settings.custom_assistant_modes.len(), 4);
    }

    #[test]
    fn a_deleted_agent_does_not_come_back() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());
        migration.seed().expect("semeia");

        // O usuário exclui um agente de fábrica.
        let mut settings = store.load().expect("lê");
        settings
            .custom_assistant_modes
            .retain(|mode| mode.id != "leetcode");
        assert_eq!(settings.assistant_mode_id, "default");
        store.save(&settings).expect("salva");

        // Um novo boot não ressuscita o agente.
        migration.seed().expect("segunda");
        let settings = store.load().expect("lê");
        assert!(!settings
            .custom_assistant_modes
            .iter()
            .any(|mode| mode.id == "leetcode"));
    }

    #[test]
    fn the_selected_factory_agent_stays_selected() {
        let settings = Settings {
            assistant_mode_id: "leetcode".to_string(),
            ..Settings::default()
        };
        let store = store_with(settings);
        AssistantCatalogMigration::new(store.clone())
            .seed()
            .expect("semeia");

        let settings = store.load().expect("lê");
        assert_eq!(settings.assistant_mode_id, "leetcode");
        assert!(settings.selected_mode_exists());
    }

    #[test]
    fn normalizes_the_legacy_transcript_lines_from_factory_agents() {
        let mut meeting = agent("meeting", "Reunião");
        meeting.suggestion_prompt =
            format!("{LEGACY_MEETING_TRANSCRIPT_BLOCK}\n\n## Output (Markdown)\n\n- Meu toque.\n");
        meeting.system_prompt =
            format!("You are the meeting copilot. {LEGACY_MEETING_SYSTEM_PREFIX} for claims.");

        let mut custom = agent("custom-vendas", "Vendas");
        custom.suggestion_prompt = format!("{LEGACY_MEETING_TRANSCRIPT_BLOCK}\n\nMeu texto.");
        custom.system_prompt = format!("{LEGACY_MEETING_SYSTEM_PREFIX}.");

        let settings = Settings {
            custom_assistant_modes: vec![meeting, custom],
            assistant_modes_seeded: true,
            ..Settings::default()
        };
        let store = store_with(settings);
        AssistantCatalogMigration::new(store.clone())
            .seed()
            .expect("normaliza");

        let saved = store.load().expect("lê");
        let meeting = saved
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "meeting")
            .expect("reunião");
        assert!(meeting.suggestion_prompt.contains("## Meeting rules"));
        assert!(meeting.suggestion_prompt.contains("Meu toque."));
        assert!(!meeting
            .suggestion_prompt
            .contains("ME is the application user"));
        assert!(!meeting.suggestion_prompt.contains("Transcript contract"));
        assert!(meeting
            .system_prompt
            .contains("Treat participant speech as the source of truth"));

        let custom = saved
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "custom-vendas")
            .expect("custom");
        // Agente personalizado não é tocado, mesmo com o texto antigo.
        assert!(custom
            .suggestion_prompt
            .contains("ME is the application user"));
        assert!(custom.system_prompt.contains(LEGACY_MEETING_SYSTEM_PREFIX));
    }

    #[test]
    fn removes_the_short_transcript_lines_from_factory_prompts() {
        let mut interview = agent("technical_interview", "Entrevista");
        interview.suggestion_prompt = format!(
            "Contract.\n\n## Rules\n\n{LEGACY_INTERVIEW_ME_LINE}{LEGACY_TECHNICAL_THEY_LINE}- One step at a time."
        );
        let mut leetcode = agent("leetcode", "LeetCode");
        leetcode.suggestion_prompt = format!("{LEGACY_INTERVIEW_ME_LINE}- Code.");
        let mut meeting = agent("meeting", "Reunião");
        meeting.suggestion_prompt = format!("- Intro.\n{LEGACY_MEETING_TAG_LINE}\n- Fim.");

        let settings = Settings {
            custom_assistant_modes: vec![interview, leetcode, meeting],
            assistant_modes_seeded: true,
            ..Settings::default()
        };
        let store = store_with(settings);
        AssistantCatalogMigration::new(store.clone())
            .seed()
            .expect("normaliza");

        let saved = store.load().expect("lê");
        let interview = saved
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "technical_interview")
            .expect("entrevista");
        assert!(!interview.suggestion_prompt.contains("ME is the user"));
        assert!(!interview
            .suggestion_prompt
            .contains("Use THEY lines to answer"));

        let leetcode = saved
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "leetcode")
            .expect("leetcode");
        assert!(!leetcode.suggestion_prompt.contains("ME is the user"));

        let meeting = saved
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "meeting")
            .expect("reunião");
        assert!(meeting
            .suggestion_prompt
            .contains("Refer to ME in the first person"));
        assert!(!meeting
            .suggestion_prompt
            .contains("Never display ME/THEY tags"));
    }

    #[test]
    fn normalizing_the_legacy_prompts_is_idempotent() {
        let mut meeting = agent("meeting", "Reunião");
        meeting.suggestion_prompt = LEGACY_MEETING_TRANSCRIPT_BLOCK.to_string();
        let settings = Settings {
            custom_assistant_modes: vec![meeting],
            assistant_modes_seeded: true,
            ..Settings::default()
        };
        let store = store_with(settings);
        let migration = AssistantCatalogMigration::new(store.clone());

        migration.seed().expect("primeira");
        let after_first = store.load().expect("lê");
        migration.seed().expect("segunda");
        assert_eq!(after_first, store.load().expect("lê"));
    }
}
