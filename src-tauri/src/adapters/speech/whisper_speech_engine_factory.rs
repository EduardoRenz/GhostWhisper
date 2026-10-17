//! Fábrica de motores de fala (VAD Silero + ASR whisper.cpp) por canal.

use std::sync::{Arc, Mutex};

use crate::domain::settings::Settings;
use crate::error::Result;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::model_store::ModelStore;
use crate::ports::settings_store::SettingsStore;
use crate::ports::speech_engine_factory::SpeechEngineFactory;
use crate::ports::speech_to_text::SpeechToText;
use crate::ports::voice_activity::VoiceActivityDetector;

use super::serialising_speech_to_text::SerialisingSpeechToText;
use super::silero_voice_activity::SileroVoiceActivityDetector;
use super::whisper_speech_to_text::WhisperSpeechToText;

/// Cria um par VAD+ASR novo (com contexto próprio) a cada chamada.
///
/// O modelo e o idioma do whisper são lidos das configurações a cada criação,
/// de modo que mudanças valem para a próxima sessão iniciada. Todos os motores
/// compartilham uma **fila global de decodificação**, para que dois canais não
/// disputem CPU ao mesmo tempo.
pub struct WhisperSpeechEngineFactory {
    models: Arc<dyn ModelStore>,
    settings: Arc<dyn SettingsStore>,
    gate: Arc<Mutex<()>>,
    publisher: Option<Arc<dyn EventPublisher>>,
}

impl WhisperSpeechEngineFactory {
    /// Cria a fábrica com o store de modelos e as configurações do usuário.
    pub fn new(models: Arc<dyn ModelStore>, settings: Arc<dyn SettingsStore>) -> Self {
        Self {
            models,
            settings,
            gate: Arc::new(Mutex::new(())),
            publisher: None,
        }
    }

    /// Liga um publicador para reportar download/carga do modelo à UI.
    pub fn with_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.publisher = Some(publisher);
        self
    }
}

/// Resolve (modelo, idioma) do whisper a partir das configurações.
fn engine_params(settings: &Settings) -> (String, String) {
    let model = match settings.whisper_model.trim() {
        "" => "tiny".to_string(),
        value => value.to_string(),
    };
    let language = match settings.source_language.trim() {
        "" => "auto".to_string(),
        value => value.to_string(),
    };
    (model, language)
}

impl SpeechEngineFactory for WhisperSpeechEngineFactory {
    fn create(&self) -> Result<(Arc<dyn VoiceActivityDetector>, Arc<dyn SpeechToText>)> {
        let settings = self.settings.load().unwrap_or_default();
        let (model, language) = engine_params(&settings);

        let vad: Arc<dyn VoiceActivityDetector> =
            Arc::new(SileroVoiceActivityDetector::new(Arc::clone(&self.models)));
        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&self.models),
            model,
            language,
            self.publisher.clone(),
        ));
        // Baixa/carrega o modelo em background e avisa a UI; enquanto isso o
        // pipeline apenas aguarda (ver `AsrNotReady`).
        whisper.preload();
        let asr: Arc<dyn SpeechToText> = Arc::new(SerialisingSpeechToText::new(
            whisper,
            Arc::clone(&self.gate),
        ));
        Ok((vad, asr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::adapters::speech::whisper_model_store::FileModelStore;

    fn factory(settings: Settings) -> WhisperSpeechEngineFactory {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(dir.path().to_path_buf()));
        let store: Arc<dyn SettingsStore> =
            Arc::new(InMemorySettingsStore::with_settings(settings));
        WhisperSpeechEngineFactory::new(models, store)
    }

    #[test]
    fn creates_independent_engines_per_call() {
        let factory = factory(Settings::default());

        let (vad_a, asr_a) = factory.create().expect("primeiro motor");
        let (vad_b, asr_b) = factory.create().expect("segundo motor");

        // Contextos distintos (nenhum compartilhamento entre canais).
        assert!(!Arc::ptr_eq(&vad_a, &vad_b));
        assert!(!Arc::ptr_eq(&asr_a, &asr_b));
    }

    #[test]
    fn engine_params_reads_settings() {
        let custom = Settings {
            whisper_model: "small".into(),
            source_language: "pt".into(),
            ..Settings::default()
        };
        assert_eq!(
            engine_params(&custom),
            ("small".to_string(), "pt".to_string())
        );

        // Vazio cai nos padrões seguros.
        let blank = Settings {
            whisper_model: "  ".into(),
            source_language: "  ".into(),
            ..Settings::default()
        };
        assert_eq!(
            engine_params(&blank),
            ("tiny".to_string(), "auto".to_string())
        );
    }

    #[test]
    fn whisper_asr_exposes_configured_language() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(dir.path().to_path_buf()));
        let asr = WhisperSpeechToText::new(models, "tiny", "en", None);
        assert_eq!(asr.language(), "en");
    }
}
