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
use super::speech_gate::SpeechGate;
use super::whisper_speech_to_text::WhisperSpeechToText;

/// Motor em cache, válido enquanto (modelo, backend) não mudarem.
///
/// O **prompt** e o **idioma** não invalidam o cache: são parâmetros de cada
/// passe, então mudá-los só chama `update_prompt`/`update_language` (sem
/// recarregar o modelo — BR-034).
struct CachedEngine {
    model: String,
    use_gpu: bool,
    whisper: Arc<WhisperSpeechToText>,
    asr: Arc<dyn SpeechToText>,
}

/// Cria o motor de fala, reusando o mesmo contexto entre os canais.
pub struct WhisperSpeechEngineFactory {
    models: Arc<dyn ModelStore>,
    settings: Arc<dyn SettingsStore>,
    gate: Arc<SpeechGate>,
    publisher: Option<Arc<dyn EventPublisher>>,
    /// A GPU foi desabilitada após um encerramento sujo (marca de crash do boot).
    force_cpu: bool,
    cached: Mutex<Option<CachedEngine>>,
}

impl WhisperSpeechEngineFactory {
    /// Cria a fábrica com o store de modelos e as configurações do usuário.
    pub fn new(models: Arc<dyn ModelStore>, settings: Arc<dyn SettingsStore>) -> Self {
        Self {
            models,
            settings,
            gate: Arc::new(SpeechGate::new()),
            publisher: None,
            force_cpu: false,
            cached: Mutex::new(None),
        }
    }

    /// Liga um publicador para reportar download/carga do modelo à UI.
    pub fn with_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.publisher = Some(publisher);
        self
    }

    /// Desabilita a GPU nesta execução (marca de crash do boot anterior).
    pub fn with_force_cpu(mut self, force_cpu: bool) -> Self {
        self.force_cpu = force_cpu;
        self
    }

    /// Idioma do motor em cache (usado em testes).
    #[cfg(test)]
    fn cached_language(&self) -> Option<String> {
        self.cached
            .lock()
            .ok()
            .and_then(|cache| cache.as_ref().map(|engine| engine.whisper.language()))
    }
}

/// Parâmetros do motor resolvidos das configurações.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EngineParams {
    model: String,
    language: String,
    prompt: String,
    use_gpu: bool,
}

/// Resolve (modelo, idioma, prompt, gpu) do whisper a partir das configurações.
///
/// `force_cpu` vem do composition root quando o boot detectou um encerramento sujo
/// com GPU ativa (marca de crash) — nesse caso `auto`/`gpu` rodam em CPU.
fn engine_params(settings: &Settings, force_cpu: bool) -> EngineParams {
    let model = match settings.whisper_model.trim() {
        "" => "tiny".to_string(),
        value => value.to_string(),
    };
    let language = map_whisper_language(&settings.source_language);
    let prompt = decode_prompt(&settings.whisper_prompt);
    let use_gpu = match settings.acceleration.trim() {
        "cpu" => false,
        "gpu" => !force_cpu,
        // `auto` (e qualquer valor inválido já saneado): GPU, salvo após crash.
        _ => !force_cpu,
    };
    EngineParams {
        model,
        language,
        prompt,
        use_gpu,
    }
}

impl SpeechEngineFactory for WhisperSpeechEngineFactory {
    fn create(&self) -> Result<Arc<dyn SpeechToText>> {
        let settings = self.settings.load().unwrap_or_default();
        let params = engine_params(&settings, self.force_cpu);

        let mut cache = self
            .cached
            .lock()
            .map_err(|_| GhostWhisperError::Other("cache do ASR envenenado".into()))?;

        // Mesmo modelo/backend ⇒ devolve o mesmo contexto (uma carga só). O
        // prompt e o idioma mudam sem recarregar (são parâmetros de cada passe).
        if let Some(engine) = cache.as_ref() {
            if engine.model == params.model && engine.use_gpu == params.use_gpu {
                engine.whisper.update_language(params.language.clone());
                engine.whisper.update_prompt(params.prompt.clone());
                return Ok(Arc::clone(&engine.asr));
            }
        }

        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&self.models),
            params.model.clone(),
            params.language.clone(),
            params.prompt.clone(),
            params.use_gpu,
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
            use_gpu: params.use_gpu,
            whisper,
            asr: Arc::clone(&asr),
        });
        Ok(asr)
    }

    /// Aplica um novo idioma ao motor **em cache**, sem recarregar o modelo
    /// (BR-034). O prompt é só contexto e não depende do idioma (ISS-059), então
    /// não é recomputado. Sem motor em cache (nenhuma sessão ativa), é no-op —
    /// a próxima sessão já nasce com a configuração nova.
    fn update_language(&self, language: &str) -> Result<()> {
        let whisper_language = map_whisper_language(language);
        let cache = self
            .cached
            .lock()
            .map_err(|_| GhostWhisperError::Other("cache do ASR envenenado".into()))?;
        if let Some(engine) = cache.as_ref() {
            engine.whisper.update_language(whisper_language);
        }
        Ok(())
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
    fn update_language_switches_the_cached_engine_without_reloading() {
        let factory = factory(Settings::default());
        let asr_a = factory.create().expect("motor");
        assert_eq!(factory.cached_language().as_deref(), Some("auto"));

        // Trocar o idioma aplica ao motor vivo (BR-034)...
        factory.update_language("pt").expect("aplica idioma");
        assert_eq!(factory.cached_language().as_deref(), Some("pt"));

        // ...e não recarrega o modelo (mesma instância).
        let asr_b = factory.create().expect("motor");
        assert!(Arc::ptr_eq(&asr_a, &asr_b));
    }

    #[test]
    fn update_language_without_a_cached_engine_is_a_noop() {
        let factory = factory(Settings::default());
        factory.update_language("pt").expect("no-op");
        assert_eq!(factory.cached_language(), None);
    }

    #[test]
    fn engine_params_reads_settings() {
        let custom = Settings {
            whisper_model: "small".into(),
            source_language: "pt".into(),
            whisper_prompt: "Grafana, Kubernetes".into(),
            ..Settings::default()
        };
        let params = engine_params(&custom, false);
        assert_eq!(params.model, "small");
        assert_eq!(params.language, "pt");
        // ISS-059: o prompt é só contexto; a regra de idioma não é anexada.
        assert_eq!(params.prompt, "Grafana, Kubernetes");

        // Vazio cai nos padrões seguros; sem prompt por default.
        let blank = Settings {
            whisper_model: "  ".into(),
            source_language: "  ".into(),
            ..Settings::default()
        };
        let params = engine_params(&blank, false);
        assert_eq!(params.model, "tiny");
        assert_eq!(params.language, "auto");
        assert_eq!(params.prompt, "");
    }

    #[test]
    fn acceleration_mode_resolves_use_gpu() {
        let auto = Settings::default();
        assert!(engine_params(&auto, false).use_gpu);
        assert!(
            !engine_params(&auto, true).use_gpu,
            "crash do boot força CPU"
        );

        let cpu = Settings {
            acceleration: "cpu".into(),
            ..Settings::default()
        };
        assert!(!engine_params(&cpu, false).use_gpu);

        let gpu = Settings {
            acceleration: "gpu".into(),
            ..Settings::default()
        };
        assert!(engine_params(&gpu, false).use_gpu);
        assert!(
            !engine_params(&gpu, true).use_gpu,
            "a marca de crash vence o modo gpu"
        );
    }

    #[test]
    fn brazilian_portuguese_maps_to_pt_for_whisper() {
        let settings = Settings {
            source_language: "pt".into(),
            ..Settings::default()
        };
        assert_eq!(engine_params(&settings, false).language, "pt");
    }

    #[test]
    fn whisper_asr_exposes_configured_language() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(dir.path().to_path_buf()));
        let asr = WhisperSpeechToText::new(models, "tiny", "en", "prompt base", false, None);
        assert_eq!(asr.language(), "en");
        assert_eq!(asr.prompt(), "prompt base");
        asr.update_prompt("outro");
        assert_eq!(asr.prompt(), "outro");
        asr.update_language("pt");
        assert_eq!(asr.language(), "pt");
    }
}
