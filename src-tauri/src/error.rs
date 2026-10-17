//! Erro comum do núcleo GhostWhisper.
//!
//! Erros são tipados e serializáveis para o frontend. Nunca inclua segredos
//! (chaves de API, áudio) nas mensagens.

use serde::Serializer;

/// Erro do núcleo.
#[derive(Debug, thiserror::Error)]
pub enum GhostWhisperError {
    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("erro de serialização: {0}")]
    Serialization(String),

    #[error("recurso ainda não implementado: {0}")]
    NotImplemented(&'static str),

    #[error("configuração inválida: {0}")]
    Config(String),

    #[error("erro de banco de dados: {0}")]
    Database(String),

    /// O motor de transcrição ainda está baixando/carregando o modelo.
    #[error("ASR ainda não está pronto (modelo carregando)")]
    AsrNotReady,

    /// Uma decodificação anterior ainda ocupa o motor (timeout estoura, mas
    /// `full()` não pode ser abortado). O chamador deve tentar depois.
    #[error("ASR ocupado com uma transcrição anterior")]
    AsrBusy,

    #[error("{0}")]
    Other(String),
}

impl From<rusqlite::Error> for GhostWhisperError {
    fn from(error: rusqlite::Error) -> Self {
        GhostWhisperError::Database(error.to_string())
    }
}

impl serde::Serialize for GhostWhisperError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Alias de resultado do núcleo.
pub type Result<T> = std::result::Result<T, GhostWhisperError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_human_readable_message() {
        let err = GhostWhisperError::NotImplemented("teste");
        let json = serde_json::to_string(&err).expect("serializa");
        assert_eq!(json, "\"recurso ainda não implementado: teste\"");
    }
}
