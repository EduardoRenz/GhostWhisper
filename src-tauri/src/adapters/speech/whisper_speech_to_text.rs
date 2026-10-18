//! Transcrição local com whisper.cpp (`whisper-rs`).
//!
//! Uma **thread de trabalho dedicada** possui o modelo e a fila de pedidos. Isso
//! resolve três patologias do modelo anterior (uma thread por chamada + `try_lock`):
//! - `full()` não pode ser abortado; quando o chamador desiste por timeout, a
//!   decodificação terminava com o lock preso e os pedidos seguintes recebiam
//!   `AsrBusy` em cascata, desperdiçando trabalho;
//! - o custo de criar thread (8 MB de pilha) e re-tentar a cada 50 ms;
//! - pedidos enfileirados atrás de um `full()` longo (o turno inteiro).
//!
//! A fila **coalesce prévias**: só a mais recente sobrevive, e um passe `Final`
//! nunca é descartado (tem prioridade e é processado antes de qualquer prévia
//! posterior). Espelha o Perssua, que serializa num `whisper-server` único e
//! re-agenda o próximo passe com o buffer mais novo
//! (`WhisperSessionStreaming.js`: `finishProcessing`/`processAudioWindow`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    WhisperVadParams,
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

// Timeouts por pedido. Prévia é descartável (pode ser refeita); o passe final é
// autoritativo e ganha mais folga.
/// Timeout base de uma prévia (ms).
const PREVIEW_TIMEOUT_MIN_MS: u64 = 15_000;
/// Teto duro do timeout de uma prévia (ms).
const PREVIEW_TIMEOUT_MAX_MS: u64 = 30_000;
/// Timeout base do passe final (ms).
const FINAL_TIMEOUT_MIN_MS: u64 = 30_000;
/// Timeout por segundo de áudio (ms): ~15 s de parede por 1 s de áudio.
const TIMEOUT_PER_AUDIO_SEC_MS: u64 = 15_000;
/// Teto duro do timeout do passe final (ms).
const FINAL_TIMEOUT_MAX_MS: u64 = 120_000;

// --- Decodificação e VAD nativo (equivalentes ao Perssua) -----------------
//
// O Perssua roda o VAD embutido do whisper.cpp (`--vad …`) e fixa os parâmetros
// de amostragem para evitar o "fallback" por temperatura, que multiplica
// decodificações e piora a latência. Transposto para o `whisper-rs`:
// `WhisperVad.js` (limiares) e `WhisperServerManager.js`/`runWhisperCli` (decode).

/// Temperatura fixa (sem amostragem estocástica).
const DECODE_TEMPERATURE: f32 = 0.0;
/// Incremento de temperatura 0 desliga o fallback (equivale a `-nf`).
const DECODE_TEMPERATURE_INC: f32 = 0.0;
/// Limiar de entropia (abaixo do padrão 2.4, menos alucinação).
const DECODE_ENTROPY_THOLD: f32 = 2.2;

/// Limiar de probabilidade de fala do VAD Silero (`--vad-threshold`).
const VAD_THRESHOLD: f32 = 0.5;
/// Duração mínima de fala (`--vad-min-speech-duration-ms`).
const VAD_MIN_SPEECH_MS: i32 = 250;
/// Silêncio mínimo para encerrar um trecho (`--vad-min-silence-duration-ms`).
const VAD_MIN_SILENCE_MS: i32 = 300;
/// Padding antes/depois da fala (`--vad-speech-pad-ms`).
const VAD_SPEECH_PAD_MS: i32 = 200;

/// Threads por contexto.
///
/// Padrão do whisper.cpp é `min(4, núcleos)`; usávamos `núcleos/2`, o que
/// deixava máquinas pequenas com poucas threads. Override por
/// `GHOSTWHISPER_ASR_THREADS` para ajuste fino sem recompilar.
fn asr_threads() -> i32 {
    resolve_threads(
        std::env::var("GHOSTWHISPER_ASR_THREADS").ok().as_deref(),
        std::thread::available_parallelism()
            .map(|cores| cores.get())
            .unwrap_or(1),
    )
}

/// Resolve o número de threads (regra pura, testável).
fn resolve_threads(override_raw: Option<&str>, cores: usize) -> i32 {
    if let Some(raw) = override_raw {
        if let Ok(parsed) = raw.trim().parse::<i32>() {
            if parsed > 0 {
                return parsed;
            }
        }
    }
    (cores as i32).clamp(1, 4)
}

/// Timeout do pedido, por qualidade e tamanho do áudio.
fn timeout_for(quality: SpeechQuality, audio_ms: usize) -> Duration {
    let audio_ms = audio_ms as u64;
    let scaled = audio_ms.saturating_mul(TIMEOUT_PER_AUDIO_SEC_MS) / 1000;
    let (min, max) = match quality {
        SpeechQuality::Preview => (PREVIEW_TIMEOUT_MIN_MS, PREVIEW_TIMEOUT_MAX_MS),
        SpeechQuality::Final => (FINAL_TIMEOUT_MIN_MS, FINAL_TIMEOUT_MAX_MS),
    };
    Duration::from_millis(min.max(scaled).min(max))
}

/// Pedido entregue à thread de trabalho.
enum Job {
    /// Decodifica um trecho. `deadline` permite descartar trabalho que ninguém
    /// mais espera (o chamador já desistiu por timeout).
    Decode {
        audio: Vec<f32>,
        quality: SpeechQuality,
        deadline: Instant,
        reply: mpsc::SyncSender<Result<Vec<TranscribedSpan>>>,
    },
}

impl Job {
    /// Responde ao chamador que o pedido foi superado (prévia antiga descartada).
    fn supersede(self) {
        let Job::Decode { reply, .. } = self;
        let _ = reply.send(Err(GhostWhisperError::AsrBusy));
    }

    fn is_final(&self) -> bool {
        matches!(
            self,
            Job::Decode {
                quality: SpeechQuality::Final,
                ..
            }
        )
    }
}

/// Handle da thread de trabalho (vivo enquanto o modelo está carregado).
struct WorkerHandle {
    tx: mpsc::Sender<Job>,
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

/// Estado da thread de trabalho dentro do adaptador.
enum WorkerState {
    /// Canal ainda não criado (aguardando `preload`).
    NotStarted,
    /// Thread viva.
    Running(WorkerHandle),
    /// Já esteve viva e foi encerrada.
    Stopped,
}

/// Estado do whisper pertencente à thread de trabalho.
struct LoadedWhisper {
    _context: WhisperContext,
    state: WhisperState,
}

impl LoadedWhisper {
    /// Carrega o contexto e o estado a partir do caminho do modelo.
    fn load(model_path: &str) -> Result<Self> {
        let context =
            WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
                .map_err(|e| GhostWhisperError::Other(format!("falha ao carregar whisper: {e}")))?;
        let state = context
            .create_state()
            .map_err(|e| GhostWhisperError::Other(format!("falha ao criar estado: {e}")))?;
        Ok(Self {
            _context: context,
            state,
        })
    }
}

/// ASR local carregado sob demanda a partir do `ModelStore`.
pub struct WhisperSpeechToText {
    models: Arc<dyn ModelStore>,
    model_name: String,
    /// Idioma fixado (`auto` = detecção pelo whisper).
    language: String,
    threads: i32,
    /// Publicador opcional para o status de download/carga (feedback da UI).
    publisher: Option<Arc<dyn EventPublisher>>,
    state: Mutex<WorkerState>,
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
            publisher,
            state: Mutex::new(WorkerState::NotStarted),
        }
    }

    /// Baixa/carrega o modelo em background e avisa a UI; em seguida atende os
    /// pedidos de decodificação na mesma thread (fila serializada).
    ///
    /// Sem isto, trocar para um modelo maior apenas travava a primeira
    /// transcrição em silêncio durante o download (BR: dar retorno ao usuário).
    pub fn preload(self: &Arc<Self>) {
        let taken = match self.state.lock() {
            Ok(mut guard) => std::mem::replace(&mut *guard, WorkerState::Stopped),
            Err(_) => return,
        };
        let started = matches!(taken, WorkerState::NotStarted);
        // Já iniciado (ou encerrado): nada a fazer.
        if !started {
            if let Ok(mut guard) = self.state.lock() {
                *guard = taken;
            }
            return;
        }

        let (tx, job_rx) = mpsc::channel();
        let ready = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        if let Ok(mut guard) = self.state.lock() {
            *guard = WorkerState::Running(WorkerHandle {
                tx,
                ready: Arc::clone(&ready),
                failed: Arc::clone(&failed),
            });
        }

        let worker = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("whisper-worker".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || run_worker(worker, job_rx, ready, failed));
        if let Err(error) = spawned {
            tracing::warn!("falha ao iniciar thread de trabalho do ASR: {error}");
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
        self.state
            .lock()
            .map(|guard| match &*guard {
                WorkerState::Running(handle) => handle.ready.load(Ordering::Relaxed),
                _ => false,
            })
            .unwrap_or(false)
    }

    /// Idioma configurado para a transcrição (usado em testes).
    #[cfg(test)]
    pub fn language(&self) -> &str {
        &self.language
    }
}

impl SpeechToText for WhisperSpeechToText {
    fn transcribe(&self, samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }

        let handle = {
            let guard = self
                .state
                .lock()
                .map_err(|_| GhostWhisperError::Other("estado do ASR envenenado".into()))?;
            match &*guard {
                WorkerState::Running(handle) => (
                    handle.tx.clone(),
                    Arc::clone(&handle.ready),
                    Arc::clone(&handle.failed),
                ),
                _ => return Err(GhostWhisperError::AsrNotReady),
            }
        };
        let (tx, ready, failed) = handle;
        if failed.load(Ordering::Relaxed) {
            return Err(GhostWhisperError::Other(
                "modelo de fala indisponível (falha na carga)".into(),
            ));
        }
        if !ready.load(Ordering::Relaxed) {
            return Err(GhostWhisperError::AsrNotReady);
        }

        let audio_ms = samples.len() * 1000 / TARGET_SAMPLE_RATE as usize;
        let timeout = timeout_for(quality, audio_ms);
        let deadline = Instant::now() + timeout;
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);

        tx.send(Job::Decode {
            audio: samples.to_vec(),
            quality,
            deadline,
            reply: reply_tx,
        })
        .map_err(|_| GhostWhisperError::AsrNotReady)?;

        match reply_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(GhostWhisperError::Other(format!(
                "ASR excedeu {}s para {audio_ms}ms de áudio",
                timeout.as_secs()
            ))),
            // Prévia superada por outra mais recente: transitório.
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(GhostWhisperError::AsrBusy),
        }
    }
}

/// Corpo da thread de trabalho: carrega o modelo e atende a fila para sempre.
fn run_worker(
    this: Arc<WhisperSpeechToText>,
    job_rx: mpsc::Receiver<Job>,
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
) {
    let hint = model_download_hint(&this.model_name);
    this.emit_status(
        AsrPhase::Preparing,
        format!("Preparando modelo {hint}… (pode baixar ~{hint})"),
    );

    // VAD nativo do whisper.cpp: o modelo Silero precisa estar em disco antes da
    // primeira decodificação (Perssua: `WhisperVad.js`).
    let vad_model_path = match this.models.vad_model_path() {
        Ok(path) => Some(path.to_string_lossy().to_string()),
        Err(error) => {
            tracing::warn!("VAD Silero indisponível: {error}");
            None
        }
    };

    let model_path = match this.models.whisper_model_path(&this.model_name) {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(error) => {
            this.emit_status(AsrPhase::Failed, error.to_string());
            failed.store(true, Ordering::Relaxed);
            return;
        }
    };

    let mut loaded = match LoadedWhisper::load(&model_path) {
        Ok(loaded) => loaded,
        Err(error) => {
            this.emit_status(AsrPhase::Failed, error.to_string());
            failed.store(true, Ordering::Relaxed);
            return;
        }
    };

    ready.store(true, Ordering::Relaxed);
    this.emit_status(
        AsrPhase::Ready,
        format!("Modelo {} pronto", this.model_name),
    );

    let mut pending: Option<Job> = None;
    loop {
        let job = match pending.take() {
            Some(job) => job,
            None => match job_rx.recv() {
                Ok(job) => job,
                // Todos os `Sender` foram soltos: o adaptador foi descartado.
                Err(_) => break,
            },
        };

        let Job::Decode {
            audio,
            quality,
            deadline,
            reply,
        } = job;

        // Trabalho que ninguém mais espera não precisa ser decodificado.
        if Instant::now() >= deadline {
            let _ = reply.send(Err(GhostWhisperError::AsrBusy));
        } else {
            let result = decode_with(
                &mut loaded,
                &audio,
                quality,
                &this.language,
                this.threads,
                vad_model_path.as_deref(),
            );
            let _ = reply.send(result);
        }

        // Coalescência: mantém no máximo um pedido pendente. `Final` tem
        // prioridade e não é descartado nem ultrapassado por prévias.
        while let Ok(next) = job_rx.try_recv() {
            let pending_is_final = pending.as_ref().is_some_and(Job::is_final);
            if pending_is_final && !next.is_final() {
                next.supersede();
                continue;
            }
            let next_is_final = next.is_final();
            if let Some(previous) = pending.replace(next) {
                previous.supersede();
            }
            if next_is_final {
                break;
            }
        }
    }
}

/// Monta os parâmetros e executa a decodificação no estado carregado.
fn decode_with(
    loaded: &mut LoadedWhisper,
    audio: &[f32],
    quality: SpeechQuality,
    language: &str,
    threads: i32,
    vad_model_path: Option<&str>,
) -> Result<Vec<TranscribedSpan>> {
    let strategy = match quality {
        SpeechQuality::Preview => SamplingStrategy::Greedy { best_of: 1 },
        SpeechQuality::Final => SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: 1.0,
        },
    };
    let mut params = FullParams::new(strategy);
    params.set_language(Some(language));
    params.set_n_threads(threads);
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    // Perssua (`WhisperServerManager.js` / `runWhisperCli`): fixa temperatura e
    // entropia e suprime tokens de não-fala. Sem isso o whisper faz "fallback"
    // por temperatura (até ~5 decodificações extras) em áudio difícil.
    params.set_temperature(DECODE_TEMPERATURE);
    params.set_temperature_inc(DECODE_TEMPERATURE_INC);
    params.set_entropy_thold(DECODE_ENTROPY_THOLD);
    params.set_suppress_nst(true);
    // VAD Silero embutido do whisper.cpp (Perssua: `WhisperVad.js`): corta
    // silêncio e devolve segmentos apenas de fala.
    if let Some(vad) = vad_model_path {
        params.set_vad_model_path(Some(vad));
        params.enable_vad(true);
        let mut vad_params = WhisperVadParams::new();
        vad_params.set_threshold(VAD_THRESHOLD);
        vad_params.set_min_speech_duration(VAD_MIN_SPEECH_MS);
        vad_params.set_min_silence_duration(VAD_MIN_SILENCE_MS);
        vad_params.set_speech_pad(VAD_SPEECH_PAD_MS);
        params.set_vad_params(vad_params);
    }

    decode(&mut loaded.state, params, audio)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_timeout_is_shorter_than_final() {
        // Áudio curto: ambos caem no piso do timeout.
        let preview = timeout_for(SpeechQuality::Preview, 100);
        let final_pass = timeout_for(SpeechQuality::Final, 100);
        assert!(
            preview < final_pass,
            "prévia descartável devia expirar antes do final: {preview:?} vs {final_pass:?}"
        );
        assert_eq!(preview, Duration::from_millis(PREVIEW_TIMEOUT_MIN_MS));
        assert_eq!(final_pass, Duration::from_millis(FINAL_TIMEOUT_MIN_MS));
    }

    #[test]
    fn timeouts_scale_with_audio_and_respect_caps() {
        assert_eq!(
            timeout_for(SpeechQuality::Final, 30_000),
            Duration::from_millis(FINAL_TIMEOUT_MAX_MS)
        );
        assert_eq!(
            timeout_for(SpeechQuality::Preview, 60_000),
            Duration::from_millis(PREVIEW_TIMEOUT_MAX_MS)
        );
        // 2 s de áudio ⇒ 30 s de parede (15 s por segundo de áudio).
        assert_eq!(
            timeout_for(SpeechQuality::Final, 2_000),
            Duration::from_millis(30_000)
        );
    }

    #[test]
    fn threads_are_bounded_and_overridable() {
        // Sem override: limitado a [1, 4] (whisper.cpp usa min(4, núcleos)).
        assert_eq!(resolve_threads(None, 1), 1);
        assert_eq!(resolve_threads(None, 4), 4);
        assert_eq!(resolve_threads(None, 32), 4);

        // Override válido vence; lixo/zero cai no padrão.
        assert_eq!(resolve_threads(Some("7"), 4), 7);
        assert_eq!(resolve_threads(Some(" 6 "), 4), 6);
        assert_eq!(resolve_threads(Some("0"), 4), 4);
        assert_eq!(resolve_threads(Some("abc"), 4), 4);
    }

    #[test]
    fn transcribe_before_preload_is_not_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(
            crate::adapters::speech::whisper_model_store::FileModelStore::new(
                dir.path().to_path_buf(),
            ),
        );
        let asr = WhisperSpeechToText::new(models, "tiny", "auto", None);
        match asr.transcribe(&[0.1; 100], SpeechQuality::Preview) {
            Err(GhostWhisperError::AsrNotReady) => {}
            other => panic!("esperava AsrNotReady, veio {other:?}"),
        }
    }

    #[test]
    fn empty_audio_is_a_noop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models: Arc<dyn ModelStore> = Arc::new(
            crate::adapters::speech::whisper_model_store::FileModelStore::new(
                dir.path().to_path_buf(),
            ),
        );
        let asr = WhisperSpeechToText::new(models, "tiny", "auto", None);
        assert!(asr
            .transcribe(&[], SpeechQuality::Preview)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn final_job_is_never_superseded() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let final_job = Job::Decode {
            audio: vec![0.0],
            quality: SpeechQuality::Final,
            deadline: Instant::now(),
            reply: tx,
        };
        assert!(final_job.is_final());

        let (tx, _rx) = mpsc::sync_channel(1);
        let preview_job = Job::Decode {
            audio: vec![0.0],
            quality: SpeechQuality::Preview,
            deadline: Instant::now(),
            reply: tx,
        };
        assert!(!preview_job.is_final());
    }

    #[test]
    fn superseded_job_notifies_the_caller() {
        let (tx, rx) = mpsc::sync_channel(1);
        Job::Decode {
            audio: vec![0.0],
            quality: SpeechQuality::Preview,
            deadline: Instant::now(),
            reply: tx,
        }
        .supersede();
        match rx.recv() {
            Ok(Err(GhostWhisperError::AsrBusy)) => {}
            other => panic!("esperava AsrBusy, veio {other:?}"),
        }
    }
}
