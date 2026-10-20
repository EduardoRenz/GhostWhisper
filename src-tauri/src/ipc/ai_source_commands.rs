//! Comandos IPC das fontes de IA (BR-044, BR-045).
//!
//! A chave **nunca** é devolvida ao frontend: só a versão mascarada.
//! A **lista** de fontes é salva pela tela de Configurações junto com as
//! demais settings (`settings_set`) — não há um segundo caminho de escrita.

use tauri::State;

use crate::domain::ai_source::{AiSource, AiSourceHealth};
use crate::error::Result;
use crate::state::AppState;

/// Lista as fontes cadastradas, na ordem de prioridade.
#[tauri::command(async)]
pub fn ai_sources_list(state: State<'_, AppState>) -> Result<Vec<AiSource>> {
    state.ai_sources.list()
}

/// Informe de saúde de cada fonte (conectada, degradada, em fallback…).
#[tauri::command(async)]
pub fn ai_sources_health(state: State<'_, AppState>) -> Vec<AiSourceHealth> {
    state.ai_sources.health()
}

/// Grava a chave da fonte no armazenamento de segredos.
#[tauri::command(async)]
pub fn ai_source_set_key(state: State<'_, AppState>, source_id: String, key: String) -> Result<()> {
    state.ai_sources.set_key(&source_id, &key)
}

/// Indica se a fonte tem chave configurada.
#[tauri::command(async)]
pub fn ai_source_has_key(state: State<'_, AppState>, source_id: String) -> Result<bool> {
    state.ai_sources.has_key(&source_id)
}

/// Remove a chave da fonte.
#[tauri::command(async)]
pub fn ai_source_clear_key(state: State<'_, AppState>, source_id: String) -> Result<()> {
    state.ai_sources.clear_key(&source_id)
}

/// Versão mascarada da chave da fonte (`••••••••·abcd`).
#[tauri::command(async)]
pub fn ai_source_key_preview(state: State<'_, AppState>, source_id: String) -> Result<String> {
    state.ai_sources.key_preview(&source_id)
}

/// Testa a conexão com **uma** fonte ("Validar chave").
///
/// Recebe a fonte inteira para que uma fonte ainda no **rascunho** da tela
/// possa ser validada antes de ser salva.
#[tauri::command(async)]
pub fn ai_source_validate(state: State<'_, AppState>, source: AiSource) -> Result<String> {
    state.ai_sources.validate(&source)
}

#[cfg(test)]
mod tests {
    use crate::adapters::ai_sources::fake_ai_source_service::{
        FakeAiSourceService, FakeValidation,
    };
    use crate::adapters::ai_sources::in_memory_ai_source_service::InMemoryAiSourceService;
    use crate::domain::ai_source::{AiSource, AiSourceHealth, AiSourceState};
    use crate::domain::settings::{default_source, ProviderKind};
    use crate::ports::ai_source_service::AiSourceService;

    fn source(id: &str, kind: ProviderKind) -> AiSource {
        let mut source = AiSource::new(id, kind);
        source.model = "modelo".into();
        source
    }

    #[test]
    fn the_key_is_never_returned_in_clear() {
        let service = InMemoryAiSourceService::new().with_key("a", "sk-secreta-1234");
        let preview = service.key_preview("a").expect("preview");
        assert_eq!(preview, "••••••••·1234");
        assert!(!preview.contains("secreta"));
    }

    #[test]
    fn each_source_has_its_own_key() {
        let service = InMemoryAiSourceService::new()
            .with_key("a", "chave-a")
            .with_key("b", "chave-b");
        assert!(service.has_key("a").expect("a"));
        assert!(service.has_key("b").expect("b"));
        service.clear_key("a").expect("limpa");
        assert!(!service.has_key("a").expect("a"));
        assert!(service.has_key("b").expect("b"));
    }

    #[test]
    fn validate_reports_the_scripted_failure() {
        let service = FakeAiSourceService::new()
            .with_validation("a", FakeValidation::Err("chave inválida".into()));
        let error = service
            .validate(&source("a", ProviderKind::OpenAi))
            .expect_err("falha");
        assert!(error.to_string().contains("chave inválida"));
    }

    #[test]
    fn validate_returns_the_provider_answer() {
        let service =
            FakeAiSourceService::new().with_validation("a", FakeValidation::Ok("conectado".into()));
        assert_eq!(
            service
                .validate(&source("a", ProviderKind::OpenAi))
                .expect("ok"),
            "conectado"
        );
    }

    #[test]
    fn the_list_keeps_the_priority_order() {
        let service = InMemoryAiSourceService::with_sources(vec![
            source("a", ProviderKind::OpenAi),
            source("b", ProviderKind::DeepSeek),
        ]);
        let sources = service.list().expect("lista");
        assert_eq!(sources[0].id, "a");
        assert_eq!(sources[1].id, "b");
        // A fonte padrão do produto também existe por padrão.
        assert_eq!(default_source().id, "default");
    }

    #[test]
    fn the_health_reports_the_state_of_each_source() {
        let mut health = AiSourceHealth::new("a");
        health.state = AiSourceState::Ready;
        health.latency_ms = Some(142);
        let service = InMemoryAiSourceService::new().with_health(vec![health]);
        let entries = service.health();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, AiSourceState::Ready);
        assert_eq!(entries[0].latency_ms, Some(142));
    }

    #[test]
    fn the_fake_scripts_the_whole_source_list() {
        let service = FakeAiSourceService::new()
            .with_sources(vec![source("a", ProviderKind::OpenAi)])
            .with_key("a", "chave")
            .with_default_validation(FakeValidation::Ok("ok".into()))
            .with_health(vec![AiSourceHealth::new("a")]);
        assert_eq!(service.list().expect("lista").len(), 1);
        assert!(service.has_key("a").expect("chave"));
        assert_eq!(
            service
                .validate(&source("a", ProviderKind::OpenAi))
                .expect("ok"),
            "ok"
        );
        assert_eq!(service.health().len(), 1);
    }
}
