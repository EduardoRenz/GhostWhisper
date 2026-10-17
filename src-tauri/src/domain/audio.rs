//! Tipos de domínio para áudio.

use serde::Serialize;

/// Taxa de amostragem de trabalho (esperada pelo whisper.cpp).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Nível de áudio de um bloco (RMS e pico).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
}

impl AudioLevel {
    /// Calcula o nível de um bloco de amostras normalizadas.
    pub fn from_samples(samples: &[f32]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sum_squares = 0.0f32;
        let mut peak = 0.0f32;
        for &sample in samples {
            sum_squares += sample * sample;
            let abs = sample.abs();
            if abs > peak {
                peak = abs;
            }
        }
        Some(Self {
            rms: (sum_squares / samples.len() as f32).sqrt(),
            peak,
        })
    }
}

/// Duração mínima de fala (RMS acima de `threshold`) para considerar o áudio como
/// fala contínua, em vez de clique/ruído.
pub const MIN_SPEECH_MS: i64 = 160;

/// Janela de análise do gate de fala, em ms.
const GATE_FRAME_MS: i64 = 20;

/// Indica se o áudio contém fala **sustentada** (RMS > `threshold` por pelo menos
/// `MIN_SPEECH_MS` contínuos), rejeitando ruído e cliques antes do ASR.
pub fn has_sustained_speech(samples: &[f32], threshold: f32) -> bool {
    let frame = ((TARGET_SAMPLE_RATE as i64 * GATE_FRAME_MS / 1000) as usize).max(1);
    let mut voiced_ms = 0i64;
    for chunk in samples.chunks(frame) {
        let level = AudioLevel::from_samples(chunk);
        let voiced = level.is_some_and(|l| l.rms > threshold);
        if voiced {
            voiced_ms += (chunk.len() * 1000 / TARGET_SAMPLE_RATE as usize) as i64;
            if voiced_ms >= MIN_SPEECH_MS {
                return true;
            }
        } else {
            voiced_ms = 0;
        }
    }
    false
}

/// Um dispositivo de entrada de áudio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    /// Identificador estável usado para seleção (nome do dispositivo).
    pub id: String,
    /// Nome legível.
    pub name: String,
    /// Se é o dispositivo padrão do sistema.
    pub is_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_from_samples() {
        assert_eq!(AudioLevel::from_samples(&[]), None);
        let level = AudioLevel::from_samples(&[0.5, -0.5]).expect("level");
        assert!((level.rms - 0.5).abs() < 1e-6);
        assert!((level.peak - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sustained_speech_rejects_short_blip() {
        // 40 ms de tom forte: menos que MIN_SPEECH_MS ⇒ não é fala.
        let blip = vec![0.5f32; TARGET_SAMPLE_RATE as usize * 40 / 1000];
        assert!(!has_sustained_speech(&blip, 0.01));
    }

    #[test]
    fn sustained_speech_accepts_real_tone() {
        // 500 ms de tom forte acima do limiar ⇒ fala sustentada.
        let tone = vec![0.5f32; TARGET_SAMPLE_RATE as usize / 2];
        assert!(has_sustained_speech(&tone, 0.01));
    }

    #[test]
    fn sustained_speech_rejects_silence() {
        let silence = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        assert!(!has_sustained_speech(&silence, 0.01));
    }
}
