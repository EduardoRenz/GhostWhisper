//! Porta do monitor atrasado do microfone ("ouvir minha voz").

use crate::error::Result;

/// Reproduz o áudio do microfone com atraso, para o usuário se ouvir.
///
/// Existe porque conferir a captura só por números (nível/RMS) é abstrato: ouvir
/// a própria voz confirma na hora o dispositivo e a qualidade. O **atraso** é o
/// que evita a realimentação (microfonia) quando o retorno sai pela caixa de som;
/// ainda assim, fone de ouvido é o cenário recomendado.
///
/// Implementado por `PulseDelayedMonitor`; dobra de teste: `FakeAudioMonitor`.
pub trait AudioMonitor: Send + Sync {
    /// Liga o monitor com o atraso dado (ms). Idempotente: religa com o novo
    /// atraso. Devolve erro quando a saída de áudio não pôde ser aberta.
    fn start(&self, delay_ms: u64) -> Result<()>;
    /// Enfileira um bloco de áudio (no-op quando desligado).
    fn push(&self, samples: &[f32]);
    /// Desliga o monitor e descarta o que estava pendente.
    fn stop(&self) -> Result<()>;
    /// Se está reproduzindo.
    fn is_running(&self) -> bool;
}
