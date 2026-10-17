//! Transcrição local com whisper.cpp (`whisper-rs`).
//!
//! Decodificações longas podem travar (`full()` não pode ser abortado). Por isso
//! cada pedido roda numa **thread de trabalho** que detém o lock do estado
//! (`Arc<Mutex<..>>`) e sinaliza o fim por canal. Se o chamador desistir por
//! timeout, a thread segue sozinha até `full()` retornar e então libera o
//! estado — sem ponteiros brutos e sem dupla decodificação concorrente.

use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::domain::audio::TARGET_SAMPLE_RATE;
use crate::domain::transcription::{AsrPhase, AsrStatus, SpeechQuality, TranscribedSpan};
use crate::error::{GhostWhisperError, Result};
use crate::ports::event_publisher::EventPublisher;
use crate::ports::model_store::ModelStore;
use crate::ports::speech_to_text::SpeechToText;

/// Timestamps do whisper vêm em centissegundos (10 ms).
const CENTISECONDS_TO_MS: i64 = 10;

/// Tamanho aproximado do modelo, para o texto de status na UI.
fn model_download_hint(model: &str) -> &'static str {
    match model {
        "tiny" => "75 MB",
        "base" => "142 MB",
        "small" => "466 MB",
        "medium" => "1,5 GB",
        _ => "1,5 GB",
    }
}

/// Timeout base por pedido (ms), independente do tamanho do áudio.
const TIMEOUT_BASE_MS: u64 = 30_000;
/// Timeout por segundo de áudio (ms): ~15 s de parede por 1 s de áudio.
const TIMEOUT_PER_AUDIO_SEC_MS: u64 = 15_000;
/// Teto duro por pedido (ms).
const TIMEOUT_MAX_MS: u64 = 120_000;

/// Threads por contexto: metade dos núcleos (mínimo 1) para não sobrecarregar
/// a CPU quando os dois canais transcrevem ao mesmo tempo.
fn asr_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|cores| (cores.get() / 2).max(1) as i32)
        .unwrap_or(1)
}

/// Timeout do pedido, escalado pelo tamanho do áudio.
fn timeout_for(audio_ms: usize) -> Duration {
    let audio_ms = audio_ms as u64;
    let scaled = audio_ms.saturating_mul(TIMEOUT_PER_AUDIO_SEC_MS) / 1000;
    Duration::from_millis(TIMEOUT_BASE_MS.max(scaled).min(TIMEOUT_MAX_MS))
}

/// Estado do whisper protegido por `Mutex`, compartilhado com a thread de
/// trabalho (`Arc`).
struct LoadedWhisper {
    _context: WhisperContext,
    state: WhisperState,
}

/// ASR local carregado sob demanda a partir do `ModelStore`.
pub struct WhisperSpeechToText {
    models: Arc<dyn ModelStore>,
    model_name: String,
    /// Idioma fixado (`auto` = detecção pelo whisper).
    language: String,
    threads: i32,
    loaded: Arc<Mutex<Option<LoadedWhisper>>>,
    /// Publicador opcional para o status de download/carga (feedback da UI).
    publisher: Option<Arc<dyn EventPublisher>>,
}

impl WhisperSpeechToText {
    /// Cria o ASR. O modelo é baixado/carregado por [`preload`](Self::preload).
    pub fn new(
        models: Arc<dyn ModelStore>,
        model_name: impl Into<String>,
        language: impl Into<String>,
        publisher: Option<Arc<dyn EventPublisher>>,
    ) -> Self {
        Self {
            models,
            model_name: model_name.into(),
            language: language.into(),
            threads: asr_threads(),
            loaded: Arc::new(Mutex::new(None)),
            publisher,
        }
    }

    /// Baixa/carrega o modelo em background, publicando o status para a UI.
    ///
    /// Sem isto, trocar para um modelo maior apenas travava a primeira
    /// transcrição em silêncio durante o download (BR: dar retorno ao usuário).
    pub fn preload(self: &Arc<Self>) {
        let loaded = Arc::clone(&self.loaded);
        let this = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("whisper-preload".into())
            .spawn(move || {
                let hint = model_download_hint(&this.model_name);
                this.emit_status(
                    AsrPhase::Preparing,
                    format!("Preparando modelo {hint}… (pode baixar ~{hint})"),
                );
                let mut guard = match loaded.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        this.emit_status(AsrPhase::Failed, "estado do ASR corrompido".into());
                        return;
                    }
                };
                match this.ensure_loaded(&mut guard) {
                    Ok(()) => this.emit_status(
                        AsrPhase::Ready,
                        format!("Modelo {} pronto", this.model_name),
                    ),
                    Err(error) => this.emit_status(AsrPhase::Failed, error.to_string()),
                }
            });
        if let Err(error) = spawned {
            tracing::warn!("falha ao iniciar thread de pré-carga do ASR: {error}");
        }
    }

    /// Publica o status do motor (silencioso sem publicador).
    fn emit_status(&self, phase: AsrPhase, message: String) {
        if let Some(publisher) = &self.publisher {
            publisher.emit_asr_status(&AsrStatus::new(
                phase,
                self.model_name.clone(),
                message.clone(),
            ));
        }
        match phase {
            AsrPhase::Failed => tracing::warn!("{message}"),
            _ => tracing::info!("{message}"),
        }
    }

    /// `true` quando o modelo já está carregado e pronto para decodificar.
    #[cfg(test)]
    pub fn is_ready(&self) -> bool {
        self.loaded
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Idioma configurado para a transcrição (usado em testes).
    #[cfg(test)]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Garante o modelo carregado sob o lock (requer o guard ativo).
    fn ensure_loaded(&self, guard: &mut Option<LoadedWhisper>) -> Result<()> {
        if guard.is_some() {
            return Ok(());
        }
        let path = self.models.whisper_model_path(&self.model_name)?;
        let path_str = path.to_string_lossy().to_string();
        let context =
            WhisperContext::new_with_params(&path_str, WhisperContextParameters::default())
                .map_err(|e| GhostWhisperError::Other(format!("falha ao carregar whisper: {e}")))?;
        let state = context
            .create_state()
            .map_err(|e| GhostWhisperError::Other(format!("falha ao criar estado: {e}")))?;
        *guard = Some(LoadedWhisper {
            _context: context,
            state,
        });
        Ok(())
    }
}

impl SpeechToText for WhisperSpeechToText {
    fn transcribe(&self, samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }

        let loaded = Arc::clone(&self.loaded);
        let language = self.language.clone();
        let threads = self.threads;
        let audio: Vec<f32> = samples.to_vec();

        // O modelo é baixado/carregado em background por `preload()`; aqui só
        // verificamos se já está pronto. Baixar dentro da requisição travava o
        // primeiro preview sem dar nenhum retorno ao usuário.
        let ready = loaded
            .lock()
            .map(|guard| guard.is_some())
            .map_err(|_| GhostWhisperError::Other("lock do ASR envenenado".into()))?;
        if !ready {
            return Err(GhostWhisperError::AsrNotReady);
        }

        // `full()` é bloqueante e não pode ser abortado; isolamos numa thread
        // que detém o lock e reporta o resultado por canal. Os params são
        // montados dentro da thread a partir de uma `String` propriedade, para
        // não prender o tempo de vida de `&self`.
        let (tx, rx) = mpsc::channel::<Result<Vec<TranscribedSpan>>>();
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let params = {
                    let strategy = match quality {
                        SpeechQuality::Preview => SamplingStrategy::Greedy { best_of: 1 },
                        SpeechQuality::Final => SamplingStrategy::BeamSearch {
                            beam_size: 5,
                            patience: 1.0,
                        },
                    };
                    let mut params = FullParams::new(strategy);
                    params.set_language(Some(&language));
                    params.set_n_threads(threads);
                    params.set_translate(false);
                    params.set_print_special(false);
                    params.set_print_progress(false);
                    params.set_print_realtime(false);
                    params.set_print_timestamps(false);
                    params
                };
                // `try_lock`: se a decodificação anterior estourou o timeout ela
                // continua rodando com o lock preso. Bloquear aqui, junto com a
                // fila global de decodificação,=deadlock permanente do ASR.
                let result = match loaded.try_lock() {
                    Ok(mut guard) => match guard.as_mut() {
                        Some(state) => decode(&mut state.state, params, &audio),
                        None => Err(GhostWhisperError::Other("estado do ASR ausente".into())),
                    },
                    Err(std::sync::TryLockError::WouldBlock) => Err(GhostWhisperError::AsrBusy),
                    Err(std::sync::TryLockError::Poisoned(_)) => {
                        Err(GhostWhisperError::Other("lock do ASR envenenado".into()))
                    }
                };
                let _ = tx.send(result);
            })
            .map_err(|e| {
                GhostWhisperError::Other(format!("falha ao iniciar thread do ASR: {e}"))
            })?;

        let audio_ms = samples.len() * 1000 / TARGET_SAMPLE_RATE as usize;
        let timeout = timeout_for(audio_ms);
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(GhostWhisperError::Other(format!(
                "ASR excedeu {}s para {audio_ms}ms de áudio",
                timeout.as_secs()
            ))),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(GhostWhisperError::Other(
                "thread do ASR encerrou sem resultado".into(),
            )),
        }
    }
}

/// Executa a decodificação no `state` e converte em spans de domínio.
fn decode(
    state: &mut WhisperState,
    params: FullParams,
    audio: &[f32],
) -> Result<Vec<TranscribedSpan>> {
    state
        .full(params, audio)
        .map_err(|e| GhostWhisperError::Other(format!("falha na transcrição: {e}")))?;

    let count = state.full_n_segments();
    let mut spans = Vec::new();
    for index in 0..count {
        let Some(segment) = state.get_segment(index) else {
            continue;
        };
        let text = segment
            .to_str_lossy()
            .map_err(|e| GhostWhisperError::Other(format!("segmento inválido: {e}")))?;
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        spans.push(TranscribedSpan {
            text: text.to_string(),
            start_ms: segment.start_timestamp() * CENTISECONDS_TO_MS,
            end_ms: segment.end_timestamp() * CENTISECONDS_TO_MS,
        });
    }
    Ok(spans)
}
