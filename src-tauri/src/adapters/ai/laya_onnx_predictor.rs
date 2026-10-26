//! Binding ao `laya-onnx` (feature `laya` — BR-098).
//!
//! O artefato ONNX (~1,2 GB) é carregado **uma vez**, na primeira
//! classificação, dentro da thread do `LayaSuggestionCueClassifier` — o boot do
//! app não fica preso. Nada de Python, HTTP ou `laya-serve`: a inferência roda
//! no ONNX Runtime em processo.
//!
//! `allow_download=false`: quem baixa é o model manager, com confirmação e
//! checksum. O token do HF é suprimido para não vazar credencial de ambiente.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::ports::suggestion_cue_classifier::ClassifierError;

use super::laya_suggestion_cue_classifier::{
    cue_option_description, LayaInput, LayaPrediction, LayaPredictor, CUE_QUESTION,
};

/// Engine ONNX do Laya, carregado sob demanda.
pub struct OnnxLayaPredictor {
    directory: PathBuf,
    engine: Mutex<Option<laya::LayaEngine>>,
    /// Depois de uma falha de carga, não tenta de novo a cada bloco: o usuário
    /// reinstala/reinicia. Evita pagar 1,2 GB de leitura a cada classificação.
    failed: AtomicBool,
}

impl OnnxLayaPredictor {
    /// Valida o diretório; a carga do engine fica para a primeira chamada.
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self, ClassifierError> {
        let directory = directory.into();
        if !has_model(&directory) {
            return Err(ClassifierError::Unavailable);
        }
        Ok(Self {
            directory,
            engine: Mutex::new(None),
            failed: AtomicBool::new(false),
        })
    }

    fn ensure_engine(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<laya::LayaEngine>>, ClassifierError> {
        let mut guard = self
            .engine
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return Ok(guard);
        }
        if self.failed.load(Ordering::Relaxed) {
            return Err(ClassifierError::Unavailable);
        }

        let loaded = laya::LayaEngine::create(laya::LayaOptions {
            model_directory: Some(self.directory.clone()),
            allow_download: false,
            hugging_face_token: Some(String::new()),
            ..Default::default()
        })
        .map_err(|error| {
            // O erro pode citar caminhos, nunca o texto da conversa.
            tracing::warn!("modelo Laya indisponível: {error}");
            self.failed.store(true, Ordering::Relaxed);
            ClassifierError::Unavailable
        })?;
        tracing::info!("modelo Laya carregado de {}", self.directory.display());
        *guard = Some(loaded);
        Ok(guard)
    }
}

impl LayaPredictor for OnnxLayaPredictor {
    fn predict(&self, input: &LayaInput) -> Result<LayaPrediction, ClassifierError> {
        let question = laya::Question::choice(
            input.question.clone(),
            input
                .options
                .iter()
                .map(|label| (label.clone(), cue_option_description(label))),
        )
        .map_err(|_| ClassifierError::InvalidResponse)?;

        let questions = laya::QuestionSet::new()
            .with(CUE_QUESTION, question)
            .map_err(|_| ClassifierError::InvalidResponse)?;

        let min_confidence = laya::MinConfidence::scalar(f64::from(input.min_confidence))
            .map_err(|_| ClassifierError::InvalidResponse)?;
        let options = laya::PredictOptions::new().min_confidence(min_confidence);

        let guard = self.ensure_engine()?;
        let engine = guard.as_ref().ok_or(ClassifierError::Unavailable)?;
        let result = engine
            .predict_with_options(input.state.clone(), &questions, &options)
            .map_err(map_predict_error)?;

        let answer = result
            .get(CUE_QUESTION)
            .ok_or(ClassifierError::InvalidResponse)?;
        Ok(LayaPrediction {
            choice: answer.as_choice().map(|choice| choice.choice.clone()),
            confidence: answer.answer_confidence() as f32,
            abstained: answer.abstention() == Some("abstained"),
            low_confidence: answer.low_confidence(),
        })
    }
}

fn map_predict_error(error: laya::LayaError) -> ClassifierError {
    use laya::LayaError;
    match error {
        LayaError::InvalidQuestion(_)
        | LayaError::InvalidState(_)
        | LayaError::InvalidMinConfidence(_)
        | LayaError::NoMarkers { .. } => ClassifierError::InvalidResponse,
        other => ClassifierError::Other(other.to_string()),
    }
}

/// `true` quando o diretório tem o grafo principal do artefato.
pub fn has_model(directory: &Path) -> bool {
    directory.join("model.onnx").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_directory_is_unavailable() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result = OnnxLayaPredictor::new(temp.path().join("nao-existe"));
        assert!(matches!(result, Err(ClassifierError::Unavailable)));
    }

    #[test]
    fn invalid_question_maps_to_invalid_response() {
        assert!(matches!(
            map_predict_error(laya::LayaError::InvalidQuestion("x".to_string())),
            ClassifierError::InvalidResponse
        ));
    }

    #[test]
    fn has_model_checks_the_graph_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert!(!has_model(temp.path()));
        std::fs::write(temp.path().join("model.onnx"), b"x").unwrap();
        assert!(has_model(temp.path()));
    }
}
