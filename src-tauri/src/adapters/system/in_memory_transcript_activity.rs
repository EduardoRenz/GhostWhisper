//! Atividade de fala em memória (dobra de teste e uso local).

use std::collections::HashSet;
use std::sync::Mutex;

use crate::ports::session_controller::AudioChannel;
use crate::ports::transcript_activity::TranscriptActivity;

/// Guarda apenas os canais marcados como falando, por sessão.
#[derive(Debug, Default)]
pub struct InMemoryTranscriptActivity {
    speaking: Mutex<HashSet<(String, AudioChannel)>>,
}

impl InMemoryTranscriptActivity {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TranscriptActivity for InMemoryTranscriptActivity {
    fn set_speaking(&self, session_id: &str, channel: AudioChannel, speaking: bool) {
        if let Ok(mut speaking_set) = self.speaking.lock() {
            let key = (session_id.to_string(), channel);
            if speaking {
                speaking_set.insert(key);
            } else {
                speaking_set.remove(&key);
            }
        }
    }

    fn is_speaking(&self, session_id: &str, channel: AudioChannel) -> bool {
        self.speaking
            .lock()
            .map(|speaking| speaking.contains(&(session_id.to_string(), channel)))
            .unwrap_or(false)
    }

    fn clear_session(&self, session_id: &str) {
        if let Ok(mut speaking) = self.speaking.lock() {
            speaking.retain(|(session, _)| session != session_id);
        }
    }
}
