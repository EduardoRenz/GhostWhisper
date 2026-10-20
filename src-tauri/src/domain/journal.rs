//! Diário de perguntas da sessão (aba "Sessão", BR-052).
//!
//! Uma **entrada** é uma pergunta do usuário e a resposta da IA. O diário é
//! **por sessão** (`session_id`): a pergunta, a resposta, o instante (ms
//! relativos ao início da sessão — a mesma escala das transcrições, BR-019)
//! e a **origem** (o "Ask" do Perssua distingue `ask`/`followup`/`screenshot`).
//!
//! As imagens anexadas (prints de tela) ficam em `JournalImage` e **não** na
//! entrada — assim a listagem do diário não carrega bytes.

use serde::Serialize;

/// Estado de uma entrada do diário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalStatus {
    /// Pergunta registrada, resposta ainda não chegou.
    Pending,
    /// Respondida.
    Answered,
    /// Falhou (o provider recusou ou deu erro).
    Error,
    /// Cancelada pelo usuário antes da resposta.
    Cancelled,
}

impl JournalStatus {
    /// Representação persistida.
    pub fn as_str(&self) -> &'static str {
        match self {
            JournalStatus::Pending => "pending",
            JournalStatus::Answered => "answered",
            JournalStatus::Error => "error",
            JournalStatus::Cancelled => "cancelled",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(JournalStatus::Pending),
            "answered" => Some(JournalStatus::Answered),
            "error" => Some(JournalStatus::Error),
            "cancelled" => Some(JournalStatus::Cancelled),
            _ => None,
        }
    }
}

/// De onde veio a pergunta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalRequestSource {
    /// Digitada pelo usuário no composer.
    Search,
    /// Executada a partir de uma pergunta pronta (chip).
    Preset,
    /// Executada com um print de tela anexado.
    Screenshot,
}

impl JournalRequestSource {
    /// Representação persistida.
    pub fn as_str(&self) -> &'static str {
        match self {
            JournalRequestSource::Search => "search",
            JournalRequestSource::Preset => "preset",
            JournalRequestSource::Screenshot => "screenshot",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "search" => Some(JournalRequestSource::Search),
            "preset" => Some(JournalRequestSource::Preset),
            "screenshot" => Some(JournalRequestSource::Screenshot),
            _ => None,
        }
    }
}

/// Uma entrada do diário: a pergunta e a resposta.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JournalEntry {
    pub id: String,
    pub session_id: String,
    pub question: String,
    /// Resposta da IA (`None` enquanto `pending`).
    pub answer: Option<String>,
    /// Milissegundos desde o **início da sessão** (mesma escala das
    /// transcrições, BR-019).
    pub asked_at_ms: i64,
    /// Milissegundos desde o início da sessão em que a resposta chegou.
    pub answered_at_ms: Option<i64>,
    /// Instante **absoluto** da pergunta (epoch, ms) — a hora de parede do
    /// cabeçalho. `0` nas dobras/entradas antigas (a UI cai no início da
    /// sessão).
    pub created_at_ms: i64,
    pub status: JournalStatus,
    pub request_source: JournalRequestSource,
    /// Provider que respondeu (ex.: `openai`).
    pub provider: Option<String>,
    /// Modelo que respondeu (ex.: `gpt-4o`).
    pub model: Option<String>,
    /// Mensagem de erro (quando `status = error`).
    pub error: Option<String>,
    /// Resumo do par pergunta/resposta (até 240 caracteres): o que a IA já
    /// respondeu. Entra no contexto das perguntas seguintes (BR-053).
    pub summary: Option<String>,
}

/// Uma imagem anexada a uma entrada (print de tela).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JournalImage {
    pub id: String,
    pub entry_id: String,
    /// Tipo MIME (ex.: `image/jpeg`).
    pub mime: String,
    /// Bytes da imagem (já comprimidos).
    pub data: Vec<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trip() {
        for status in [
            JournalStatus::Pending,
            JournalStatus::Answered,
            JournalStatus::Error,
            JournalStatus::Cancelled,
        ] {
            assert_eq!(JournalStatus::from_str(status.as_str()), Some(status));
        }
        assert_eq!(JournalStatus::from_str("outro"), None);
    }

    #[test]
    fn request_source_round_trip() {
        for source in [
            JournalRequestSource::Search,
            JournalRequestSource::Preset,
            JournalRequestSource::Screenshot,
        ] {
            assert_eq!(
                JournalRequestSource::from_str(source.as_str()),
                Some(source)
            );
        }
        assert_eq!(JournalRequestSource::from_str("outro"), None);
    }

    /// Contrato com o frontend (`JournalEntry` em `src/core/domain/journal.ts`).
    #[test]
    fn entry_serializes_with_the_fields_the_ui_reads() {
        let json: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&JournalEntry {
                id: "e1".into(),
                session_id: "s1".into(),
                question: "o que eu disse?".into(),
                answer: Some("isto".into()),
                asked_at_ms: 9_000,
                answered_at_ms: Some(9_500),
                created_at_ms: 1_700_000_000_000,
                status: JournalStatus::Answered,
                request_source: JournalRequestSource::Search,
                provider: Some("openai".into()),
                model: Some("gpt-4o".into()),
                error: None,
                summary: Some("resumo".into()),
            })
            .expect("json"),
        )
        .expect("parse");

        assert_eq!(json["id"], "e1");
        assert_eq!(json["session_id"], "s1");
        assert_eq!(json["question"], "o que eu disse?");
        assert_eq!(json["answer"], "isto");
        assert_eq!(json["asked_at_ms"], 9_000);
        assert_eq!(json["answered_at_ms"], 9_500);
        assert_eq!(json["created_at_ms"], 1_700_000_000_000_i64);
        assert_eq!(json["status"], "answered");
        assert_eq!(json["request_source"], "search");
        assert_eq!(json["provider"], "openai");
        assert_eq!(json["model"], "gpt-4o");
        assert!(json["error"].is_null());
        assert_eq!(json["summary"], "resumo");
    }

    #[test]
    fn image_serializes_with_the_fields_the_ui_reads() {
        let json: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&JournalImage {
                id: "i1".into(),
                entry_id: "e1".into(),
                mime: "image/jpeg".into(),
                data: vec![1, 2, 3],
                width: Some(1280),
                height: Some(720),
            })
            .expect("json"),
        )
        .expect("parse");

        assert_eq!(json["id"], "i1");
        assert_eq!(json["entry_id"], "e1");
        assert_eq!(json["mime"], "image/jpeg");
        assert_eq!(json["width"], 1280);
        assert_eq!(json["height"], 720);
    }
}
