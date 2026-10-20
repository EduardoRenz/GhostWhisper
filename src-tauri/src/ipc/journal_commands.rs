//! Comandos IPC do diário de perguntas (aba "Sessão", BR-052/BR-053).

use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::domain::journal::{JournalEntry, JournalImage, JournalRequestSource};
use crate::domain::llm::CompletionImage;
use crate::error::Result;
use crate::ports::ask_controller::{AskOutcome, AskRequest};
use crate::ports::session_controller::SessionController;
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

/// Sessão do pedido de pergunta: a informada ou a **sessão ativa** — abrindo
/// uma **pausada** quando o usuário ainda não iniciou a transcrição (BR-019).
fn resolve_ask_session(
    session: &dyn SessionController,
    requested: Option<String>,
) -> Result<String> {
    match requested {
        Some(id) if !id.trim().is_empty() => Ok(id),
        _ => session.ensure_session(),
    }
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
///
/// **Sempre existe uma sessão** (BR-019): sem gravação, a pergunta abre uma
/// sessão **pausada** — é ela que dá dono ao diário, ao histórico e ao tempo.
#[tauri::command(async)]
pub fn journal_ask(state: State<'_, AppState>, request: AskDto) -> Result<AskOutcome> {
    let images = decode_images(request.images)?;
    let session_id = resolve_ask_session(state.session.as_ref(), request.session_id)?;
    state.ask.ask(AskRequest {
        session_id: Some(session_id),
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::ports::session_controller::AudioChannel;

    /// Sessão de teste: conta os `ensure_session` (e devolve `s1`).
    struct FakeSession {
        ensured: AtomicUsize,
    }

    impl SessionController for FakeSession {
        fn start(&self, _channels: &[AudioChannel]) -> Result<String> {
            Ok("s1".into())
        }
        fn ensure_session(&self) -> Result<String> {
            self.ensured.fetch_add(1, Ordering::SeqCst);
            Ok("s1".into())
        }
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn pause(&self) -> Result<()> {
            Ok(())
        }
        fn resume(&self) -> Result<()> {
            Ok(())
        }
        fn set_channels(&self, _channels: &[AudioChannel]) -> Result<()> {
            Ok(())
        }
        fn continue_session(&self, id: &str, _channels: &[AudioChannel]) -> Result<String> {
            Ok(id.to_string())
        }
        fn set_language(&self, _language: &str) -> Result<()> {
            Ok(())
        }
        fn current_session_id(&self) -> Option<String> {
            Some("s1".into())
        }
    }

    #[test]
    fn ask_without_a_session_opens_a_paused_one() {
        // Perguntar sem transcrição sempre tem sessão (BR-019/BR-052).
        let session = FakeSession {
            ensured: AtomicUsize::new(0),
        };
        assert_eq!(
            resolve_ask_session(&session, None).expect("sessão"),
            "s1"
        );
        assert_eq!(session.ensured.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_explicit_session_is_used_as_is() {
        let session = FakeSession {
            ensured: AtomicUsize::new(0),
        };
        assert_eq!(
            resolve_ask_session(&session, Some("s9".into())).expect("sessão"),
            "s9"
        );
        assert_eq!(session.ensured.load(Ordering::SeqCst), 0);
    }

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
