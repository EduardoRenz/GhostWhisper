//! Comandos IPC do contexto de tela (BR-105/BR-106).
//!
//! A AskBar consulta `screen_context_snapshot` ao abrir; as Configurações
//! consultam `screen_context_status` para a linha do card. O texto completo
//! do snapshot **não** sai por aqui — só o preview já cortado (CA-04.3).

use serde::Serialize;
use tauri::State;

use crate::domain::screen_context::{ScreenContextSnapshot, PREVIEW_MAX_CHARS};
use crate::ports::screen_context_controller::ScreenContextStatus;
use crate::state::AppState;

/// DTO do snapshot para a pill (espelha `ScreenContextEvent` no front).
#[derive(Debug, Clone, Serialize)]
pub struct ScreenContextDto {
    pub available: bool,
    /// A última leitura falhou (AT-SPI/D-Bus inacessível) — pill
    /// "Contexto indisponível" em vez de "Sem contexto".
    pub failed: bool,
    pub app: String,
    pub window_title: String,
    pub preview: String,
    pub captured_at_ms: i64,
}

impl From<&ScreenContextSnapshot> for ScreenContextDto {
    fn from(snapshot: &ScreenContextSnapshot) -> Self {
        let preview: String = snapshot
            .text
            .chars()
            .take(PREVIEW_MAX_CHARS.saturating_sub(1))
            .collect();
        let preview = if snapshot.text.chars().count() > PREVIEW_MAX_CHARS {
            format!("{preview}…")
        } else {
            preview
        };
        Self {
            available: true,
            failed: false,
            app: snapshot.app.clone(),
            window_title: snapshot.window_title.clone(),
            preview,
            captured_at_ms: snapshot.captured_at_ms,
        }
    }
}

/// Snapshot do contexto de tela para a AskBar.
///
/// Contrato (ISS-092): com a flag **desligada** devolve `null` (a pill nem
/// renderiza). Com a flag ligada devolve **sempre** um DTO — `available`
/// quando há contexto, ou `failed` quando o mecanismo falhou/ainda não existe
/// (ex.: flag ligada sem reiniciar o app) — para a pill nunca ficar presa em
/// "Lendo contexto…".
///
/// Com o **checkbox desmarcado** (BR-106) não toca no adaptador: devolve o
/// DTO vazio (a pill mostra "Contexto desativado" e nem consulta).
///
/// **`(async)` é obrigatório:** a leitura da árvore AT-SPI roda em thread
/// própria e espera até o timeout do snapshot; como comando síncrono ela
/// bloquearia a **main thread** (congelamento da UI — ISS-097).
#[tauri::command(async)]
pub fn screen_context_snapshot(state: State<'_, AppState>) -> Option<ScreenContextDto> {
    // Flag desligada: nada a mostrar, nenhuma consulta ao adaptador.
    let enabled = state
        .settings
        .load()
        .map(|loaded| loaded.screen_context_enabled)
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    // Flag ligada mas sem fonte (ligada em runtime, sem reiniciar): o
    // adaptador não existe — pill "Contexto indisponível".
    let Some(controller) = state.screen_context.as_ref() else {
        return Some(ScreenContextDto {
            available: false,
            failed: true,
            ..empty_context()
        });
    };

    match controller.snapshot_for_display() {
        Some(snapshot) => Some(ScreenContextDto::from(&snapshot)),
        // Sem contexto **ou** falha: o status da última leitura distingue.
        None => Some(ScreenContextDto {
            available: false,
            failed: matches!(controller.status(), ScreenContextStatus::Unavailable),
            ..empty_context()
        }),
    }
}

/// DTO vazio (sem contexto) para o contrato acima.
fn empty_context() -> ScreenContextDto {
    ScreenContextDto {
        available: false,
        failed: false,
        app: String::new(),
        window_title: String::new(),
        preview: String::new(),
        captured_at_ms: 0,
    }
}

/// Status do recurso para a linha do card nas Configurações.
///
/// `(async)` pelo mesmo motivo do snapshot: nunca segurar a main thread
/// (ISS-097).
#[tauri::command(async)]
pub fn screen_context_status(state: State<'_, AppState>) -> &'static str {
    state
        .screen_context
        .as_ref()
        .map(|controller| controller.status().as_str())
        .unwrap_or("disabled")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::screen_context::ScreenContextSnapshot;

    #[test]
    fn dto_cuts_preview_at_budget() {
        let snapshot = ScreenContextSnapshot {
            app: "chrome".into(),
            window_title: "Doc".into(),
            text: "x".repeat(PREVIEW_MAX_CHARS + 100),
            captured_at_ms: 1,
        };
        let dto = ScreenContextDto::from(&snapshot);
        assert_eq!(dto.preview.chars().count(), PREVIEW_MAX_CHARS);
        assert!(dto.preview.ends_with('…'));
        assert!(dto.available);
    }

    #[test]
    fn dto_keeps_short_preview_intact() {
        let snapshot = ScreenContextSnapshot {
            app: "chrome".into(),
            window_title: "Doc".into(),
            text: "curto".into(),
            captured_at_ms: 1,
        };
        let dto = ScreenContextDto::from(&snapshot);
        assert_eq!(dto.preview, "curto");
    }
}
