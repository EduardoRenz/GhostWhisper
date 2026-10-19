//! Porta da **referência de áudio** usada pelo cancelamento de eco (AEC).
//!
//! É o áudio do sistema (o que sai pelo alto-falante e pode voltar pelo
//! microfone) numa janela recente. O microfone consome essa janela atrasada para
//! estimar e subtrair o eco (BR-029).

/// Referência de áudio compartilhada entre o canal do sistema e o microfone.
///
/// Adaptador: `SharedAudioReference` (anel em memória) alimentado por
/// `SystemAudioReferenceTap`.
pub trait AudioReference: Send + Sync {
    /// Guarda um bloco de referência (áudio do sistema, mono 16 kHz).
    fn push(&self, samples: &[f32]);
    /// Janela de `len` amostras terminando `delay_samples` atrás do fim atual.
    ///
    /// Histórico curto é completado com zeros à esquerda (o cancelador não vê
    /// referência e não altera o sinal).
    fn delayed(&self, delay_samples: usize, len: usize) -> Vec<f32>;
}
