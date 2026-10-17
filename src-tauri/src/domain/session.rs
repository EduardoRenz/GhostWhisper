//! Entidade de sessão de transcrição.

use serde::Serialize;

/// Estado de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SessionStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "ended")]
    Ended,
}

impl SessionStatus {
    /// Representação persistida.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Ended => "ended",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "active" => Some(SessionStatus::Active),
            "ended" => Some(SessionStatus::Ended),
            _ => None,
        }
    }
}

/// Uma sessão de escuta/transcrição.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    /// Provider de análise (no MVP, sempre `local`).
    pub provider: String,
    pub model: Option<String>,
    pub transcription_model: Option<String>,
    pub user_id: Option<String>,
    pub subject: Option<String>,
    pub overview: Option<String>,
    pub status: SessionStatus,
}
