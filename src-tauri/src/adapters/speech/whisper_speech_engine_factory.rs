//! Fábrica do motor de fala (ASR whisper.cpp) com **contexto compartilhado**.
//!
//! Transposto do Perssua, que mantém um único `whisper-server` para todos os
//! canais/falantes (o modelo é carregado uma vez). Aqui, o modelo e o idioma do
//! whisper vêm das configurações e um único `WhisperSpeechToText` é reusado
//! enquanto não mudarem — a serialização da decodificação continua garantida por
//! `SerialisingSpeechToText` (uma fila global).

use std::sync::{Arc, Mutex};

use crate::domain::settings::Settings;
use crate::domain::transcription::{decode_prompt, map_whisper_language};
use crate::error::{GhostWhisperError, Result};
use crate::ports::event_publisher::EventPublisher;
use crate::ports::model_store::ModelStore;
use crate::ports::settings_store::SettingsStore;
use crate::ports::speech_engine_factory::SpeechEngineFactory;
use crate::ports::speech_to_text::SpeechToText;

use super::serialising_speech_to_text::SerialisingSpeechToText;
use super::whisper_speech_to_text::WhisperSpeechToText;

/// Motor em cache, válido enquanto (modelo, idioma) não mudarem.
///
/// O **prompt** não invalida o cache: ele é um parâmetro de cada passe, então
/// mudar o prompt só chama [`WhisperSpeechToText::update_prompt`] (sem recarregar
/// o modelo).
struct CachedEngine {
    model: String,
    language: String,
    whisper: Arc<WhisperSpeechToText>,
    asr: Arc<dyn SpeechToText>,
}

/// Cria o motor de fala, reusando o mesmo contexto entre os canais.
pub struct WhisperSpeechEngineFactory {
    models: Arc<dyn ModelStore>,
    settings: Arc<dyn SettingsStore>,
    gate: Arc<Mutex<()>>,
    publisher: Option<Arc<dyn EventPublisher>>,
    cached: Mutex<Option<CachedEngine>>,
}

impl WhisperSpeechEngineFactory {
    /// Cria a fábrica com o store de modelos e as configurações do usuário.
    pub fn new(models: Arc<dyn ModelStore>, settings: Arc<dyn SettingsStore>) -> Self {
        Self {
            models,
            settings,
            gate: Arc::new(Mutex::new(())),
            publisher: None,
            cached: Mutex::new(None),
        }
    }

    /// Liga um publicador para reportar download/carga do modelo à UI.
    pub fn with_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.publisher = Some(publisher);
        self
    }
}

/// Parâmetros do motor resolvidos das configurações.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EngineParams {
    model: String,
    language: String,
    prompt: String,
}

/// Resolve (modelo, idioma, prompt) do whisper a partir das configurações.
fn engine_params(settings: &Settings) -> EngineParams {
    let model = match settings.whisper_model.trim() {
        "" => "tiny".to_string(),
        value => value.to_string(),
    };
    let language = map_whisper_language(&settings.source_language);
    let prompt = decode_prompt(&settings.whisper_prompt, &settings.source_language);
    EngineParams {
        model,
        language,
        prompt,
    }
}

impl SpeechEngineFactory for WhisperSpeechEngineFactory {
    fn create(&self) -> Result<Arc<dyn SpeechToText>> {
        let settings = self.settings.load().unwrap_or_default();
        let params = engine_params(&settings);

        let mut cache = self
            .cached
            .lock()
            .map_err(|_| GhostWhisperError::Other("cache do ASR envenenado".into()))?;

        // Mesmo modelo/idioma ⇒ devolve o mesmo contexto (uma carga só). O prompt
        // muda sem recarregar.
        if let Some(engine) = cache.as_ref() {
            if engine.model == params.model && engine.language == params.language {
                engine.whisper.update_prompt(params.prompt);
                return Ok(Arc::clone(&engine.asr));
            }
        }

        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&self.models),
            params.model.clone(),
            params.language.clone(),
            params.prompt,
            self.publisher.clone(),
        ));
        // Baixa/carrega o modelo em background e avisa a UI; enquanto isso o
        // pipeline apenas aguarda (ver `AsrNotReady`).
        whisper.preload();
        let asr: Arc<dyn SpeechToText> = Arc::new(SerialisingSpeechToText::new(
            Arc::clone(&whisper) as Arc<dyn SpeechToText>,
            Arc::clone(&self.gate),
        ));
        *cache = Some(CachedEngine {
            model: params.model,
            language: params.language,
            whisper,
            asr: Arc::clone(&asr),
        });
        Ok(asr)
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
    fn reuses_the_same_engine_for_same_settings() {
        let factory = factory(Settings::default());

        let asr_a = factory.create().expect("primeiro motor");
        let asr_b = factory.create().expect("segundo motor");

        // Um contexto só, compartilhado pelos canais (Perssua: um `whisper-server`).
        assert!(Arc::ptr_eq(&asr_a, &asr_b));
    }

    #[test]
    fn engine_params_reads_settings() {
        let custom = Settings {
            whisper_model: "small".into(),
            source_language: "pt".into(),
            ..Settings::default()
        };
        let params = engine_params(&custom);
        assert_eq!(params.model, "small");
        assert_eq!(params.language, "pt");
        assert!(params.prompt.contains("Transcribe it in Portuguese."));

        // Vazio cai nos padrões seguros; `auto` não acrescenta a regra de idioma.
        let blank = Settings {
            whisper_model: "  ".into(),
            source_language: "  ".into(),
            ..Settings::default()
        };
        let params = engine_params(&blank);
        assert_eq!(params.model, "tiny");
        assert_eq!(params.language, "auto");
        assert_eq!(params.prompt, Settings::default().whisper_prompt);
    }

    #[test]
    fn brazilian_portuguese_maps_to_pt_for_whisper() {
        let settings = Settings {
            source_language: "pt".into(),
            ..Settings::default()
        };
        assert_eq!(engine_params(&settings).language, "pt");
    }

    #[test]
    fn whisper_asr_exposes_configured_language() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(dir.path().to_path_buf()));
        let asr = WhisperSpeechToText::new(models, "tiny", "en", "prompt base", None);
        assert_eq!(asr.language(), "en");
        assert_eq!(asr.prompt(), "prompt base");
        asr.update_prompt("outro");
        assert_eq!(asr.prompt(), "outro");
    }
}
