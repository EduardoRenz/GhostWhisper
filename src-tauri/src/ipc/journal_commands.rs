//! Comandos IPC do diário de perguntas (aba "Sessão", BR-052/BR-053).

use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::domain::journal::{JournalEntry, JournalImage, JournalRequestSource};
use crate::domain::llm::CompletionImage;
use crate::error::Result;
use crate::ports::ask_controller::{AskOutcome, AskRequest};
use crate::state::AppState;

/// Origem da pergunta (vinda do frontend).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestSourceDto {
    Search,
    Preset,
    Screenshot,
}

impl From<RequestSourceDto> for JournalRequestSource {
    fn from(value: RequestSourceDto) -> Self {
        match value {
            RequestSourceDto::Search => JournalRequestSource::Search,
            RequestSourceDto::Preset => JournalRequestSource::Preset,
            RequestSourceDto::Screenshot => JournalRequestSource::Screenshot,
        }
    }
}

/// Imagem anexada (vinda do frontend, em base64).
#[derive(Debug, Clone, Deserialize)]
pub struct ImageDto {
    /// Tipo MIME (ex.: `image/jpeg`).
    pub mime: String,
    /// Dados em base64 (sem o prefixo `data:`).
    pub data: String,
}

/// Pedido de pergunta (vindo do frontend).
#[derive(Debug, Deserialize)]
pub struct AskDto {
    /// Sessão alvo (vazia = a ativa ou a mais recente).
    #[serde(default)]
    pub session_id: Option<String>,
    /// A pergunta.
    pub question: String,
    /// Origem da pergunta.
    #[serde(default)]
    pub source: Option<RequestSourceDto>,
    /// Imagens anexadas (no máximo uma — BR-055).
    #[serde(default)]
    pub images: Vec<ImageDto>,
}

/// Sessão alvo do diário: a informada ou a ativa (a mais recente).
///
/// `None` quando não há sessão nenhuma — o diário fica **vazio**, não é
/// erro: a aba mostra o estado vazio (BR-052).
fn target_session(state: &AppState, session_id: Option<String>) -> Option<String> {
    match session_id {
        Some(id) if !id.trim().is_empty() => Some(id),
        _ => state.session.current_session_id(),
    }
}

/// Diário de uma sessão, em ordem cronológica.
#[tauri::command(async)]
pub fn journal_list(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<Vec<JournalEntry>> {
    let Some(session_id) = target_session(&state, session_id) else {
        return Ok(Vec::new());
    };
    state.journal.list_by_session(&session_id)
}

/// Uma entrada do diário.
#[tauri::command(async)]
pub fn journal_get(state: State<'_, AppState>, id: String) -> Result<Option<JournalEntry>> {
    state.journal.get(&id)
}

/// Busca full-text no diário da sessão.
#[tauri::command(async)]
pub fn journal_search(
    state: State<'_, AppState>,
    session_id: Option<String>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<JournalEntry>> {
    let Some(session_id) = target_session(&state, session_id) else {
        return Ok(Vec::new());
    };
    state
        .journal
        .search(&session_id, &query, limit.unwrap_or(50))
}

/// Imagens de uma entrada (para a miniatura no diário).
#[tauri::command(async)]
pub fn journal_images(state: State<'_, AppState>, entry_id: String) -> Result<Vec<JournalImage>> {
    state.journal.images_of(&entry_id)
}

/// Remove uma entrada do diário.
#[tauri::command(async)]
pub fn journal_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    state.journal.delete(&id)
}

/// Faz a pergunta e registra no diário.
#[tauri::command(async)]
pub fn journal_ask(state: State<'_, AppState>, request: AskDto) -> Result<AskOutcome> {
    let images = decode_images(request.images)?;
    state.ask.ask(AskRequest {
        session_id: request.session_id,
        question: request.question,
        source: request
            .source
            .map(Into::into)
            .unwrap_or(JournalRequestSource::Search),
        images,
    })
}

/// Cancela uma pergunta em curso.
#[tauri::command(async)]
pub fn journal_cancel(state: State<'_, AppState>, id: String) -> Result<()> {
    state.ask.cancel(&id)
}

/// Decodifica as imagens em base64 (no máximo uma — BR-055).
fn decode_images(images: Vec<ImageDto>) -> Result<Vec<CompletionImage>> {
    if images.len() > 1 {
        return Err(crate::error::GhostWhisperError::Config(
            "só uma imagem por pergunta".to_string(),
        ));
    }
    images
        .into_iter()
        .map(|image| {
            let data = decode_base64(&image.data)?;
            Ok(CompletionImage {
                mime: image.mime,
                data,
            })
        })
        .collect()
}

/// Decodifica base64 padrão (ignora espaços e quebras de linha).
fn decode_base64(value: &str) -> Result<Vec<u8>> {
    let compact: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(compact.as_bytes())
        .map_err(|error| {
            crate::error::GhostWhisperError::Config(format!("imagem em base64 inválida: {error}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(decode_base64("aGVsbG8=").expect("decode"), b"hello");
        assert_eq!(decode_base64("").expect("decode"), Vec::<u8>::new());
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(decode_base64("a").is_err(), "comprimento inválido");
        assert!(decode_base64("!!!!").is_err(), "caractere inválido");
    }

    #[test]
    fn accepts_only_one_image() {
        let one = ImageDto {
            mime: "image/jpeg".into(),
            data: "aGVsbG8=".into(),
        };
        assert_eq!(decode_images(vec![one.clone()]).expect("uma").len(), 1);
        assert!(decode_images(vec![one.clone(), one]).is_err());
    }
}
