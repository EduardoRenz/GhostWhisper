//! Comandos IPC do resumo da sessão.

use serde::Deserialize;
use tauri::State;

use crate::domain::overview::MeetingSummary;
use crate::error::Result;
use crate::ports::summary_generator::{SummaryMode, SummaryRequest, SummaryResult};
use crate::state::AppState;

/// Modo do resumo escolhido pela UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryModeDto {
    Auto,
    Manual,
}

impl From<SummaryModeDto> for SummaryMode {
    fn from(value: SummaryModeDto) -> Self {
        match value {
            SummaryModeDto::Auto => SummaryMode::Auto,
            SummaryModeDto::Manual => SummaryMode::Manual,
        }
    }
}

/// Resumo já gravado da sessão (`None` se ainda não existe).
#[tauri::command(async)]
pub fn summary_get(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<Option<MeetingSummary>> {
    state.summary.get(session_id)
}

/// Gera/atualiza o resumo. `mode` omitido = automático (incremental).
#[tauri::command(async)]
pub fn summary_generate(
    state: State<'_, AppState>,
    session_id: Option<String>,
    mode: Option<SummaryModeDto>,
) -> Result<SummaryResult> {
    state.summary.generate(SummaryRequest {
        session_id,
        mode: mode.map(Into::into).unwrap_or(SummaryMode::Auto),
    })
}
