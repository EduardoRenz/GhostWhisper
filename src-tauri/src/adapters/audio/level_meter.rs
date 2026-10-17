//! `AudioConsumer` que emite o nível de áudio (RMS/pico) para a UI.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::adapters::events::tauri_event_publisher::{AudioLevelPayload, AUDIO_LEVEL_EVENT};
use crate::domain::audio::AudioLevel;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::session_controller::AudioChannel;

/// Intervalo mínimo entre eventos de nível.
const EMIT_INTERVAL: Duration = Duration::from_millis(100);

/// Mede o nível do bloco recebido e emite `audio_level` de tempos em tempos.
pub struct LevelMeter {
    app: AppHandle,
    channel: AudioChannel,
    last_emit: Mutex<Instant>,
}

impl LevelMeter {
    /// Cria o medidor para um canal (emite eventos com o canal no payload).
    pub fn new(app: AppHandle, channel: AudioChannel) -> Self {
        let last_emit = Instant::now()
            .checked_sub(EMIT_INTERVAL)
            .unwrap_or_else(Instant::now);
        Self {
            app,
            channel,
            last_emit: Mutex::new(last_emit),
        }
    }

    /// Decide se é hora de emitir (throttle) e atualiza o marcador.
    fn should_emit(&self) -> bool {
        self.last_emit
            .lock()
            .map(|mut last| {
                if last.elapsed() >= EMIT_INTERVAL {
                    *last = Instant::now();
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
}

impl AudioConsumer for LevelMeter {
    fn consume(&self, samples: &[f32]) {
        let Some(level) = AudioLevel::from_samples(samples) else {
            return;
        };
        if self.should_emit() {
            let _ = self.app.emit(
                AUDIO_LEVEL_EVENT,
                AudioLevelPayload {
                    channel: self.channel.as_str(),
                    rms: level.rms,
                    peak: level.peak,
                },
            );
        }
    }
}
