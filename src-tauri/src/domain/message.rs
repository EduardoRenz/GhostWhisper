//! Mensagens de uma sessão (sugestões/insumos da IA).
//!
//! Espelha a tabela `messages` do schema (a mesma do Perssua), que existia desde
//! a task 05 e estava sem uso: as sugestões geradas não eram persistidas, então
//! sumiam ao fechar o painel e não entravam na linha temporal.

use serde::Serialize;

/// Papel de quem produziu a mensagem (o `CHECK` do schema usa estes valores).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Falante/usuário (ex.: um comando digitado — ainda não usado).
    User,
    /// Sugestão/resposta da IA.
    Assistant,
    /// Mensagem de sistema.
    System,
}

impl MessageRole {
    /// Representação persistida.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(MessageRole::User),
            "assistant" => Some(MessageRole::Assistant),
            "system" => Some(MessageRole::System),
            _ => None,
        }
    }
}

/// Uma mensagem dentro de uma sessão.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConversationMessage {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    /// Milissegundos desde o **início da sessão** (mesma escala das
    /// transcrições, para as duas formarem uma única linha temporal).
    pub timestamp_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_round_trip() {
        for role in [
            MessageRole::User,
            MessageRole::Assistant,
            MessageRole::System,
        ] {
            assert_eq!(MessageRole::from_str(role.as_str()), Some(role));
        }
        assert_eq!(MessageRole::from_str("outro"), None);
    }

    #[test]
    fn role_serializes_as_lowercase() {
        let json = serde_json::to_string(&MessageRole::Assistant).expect("json");
        assert_eq!(json, "\"assistant\"");
    }

    /// Contrato com o frontend (`ConversationMessage` em `transcript.ts`).
    #[test]
    fn message_serializes_with_the_fields_the_ui_reads() {
        let json: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&ConversationMessage {
                id: "m1".into(),
                session_id: "s1".into(),
                role: MessageRole::Assistant,
                content: "faça X".into(),
                timestamp_ms: 9_000,
            })
            .expect("json"),
        )
        .expect("parse");

        assert_eq!(json["id"], "m1");
        assert_eq!(json["session_id"], "s1");
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["content"], "faça X");
        assert_eq!(json["timestamp_ms"], 9_000);
    }
}
