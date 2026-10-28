//! Atividade de fala por sessão/canal (porta de leitura/escrita em memória).
//!
//! O pipeline publica "estou processando fala neste canal"; o gate de sugestões
//! consulta para segurar uma sugestão enquanto o outro lado ainda está falando
//! (debounce estrutural). É memória de sessão: não vai para o banco.

use super::session_controller::AudioChannel;

/// Estado de fala por sessão e canal.
pub trait TranscriptActivity: Send + Sync {
    /// Marca/limpa a fala do canal na sessão.
    fn set_speaking(&self, session_id: &str, channel: AudioChannel, speaking: bool);
    /// O canal está falando/processando fala nesta sessão?
    fn is_speaking(&self, session_id: &str, channel: AudioChannel) -> bool;
    /// Remove o estado de uma sessão (troca/encerramento).
    fn clear_session(&self, session_id: &str);
}
