//! Pipeline de transcrição: buffer de turno → ASR em streaming → eventos.
//!
//! Modelo de streaming transposto do **Perssua** (`electron/local-whisper`,
//! `WhisperSession.js` / `WhisperSessionStreaming.js`), que é a referência de
//! produto (BR-015):
//! - **um buffer de turno** acumula o áudio do turno atual (teto rígido de 60 s
//!   = `FINAL_UTTERANCE_MAX_SEC`);
//! - **prévias frequentes** decodificam o turno que cresce (~a cada 0,75 s de
//!   áudio novo) e publicam um **texto rolante** revisável (`is_final = false`);
//! - a primeira prévia espera `INITIAL_PREVIEW_MS` (1,2 s), como no Perssua;
//! - o turno fecha após **1 s de silêncio** (`VAD_SILENCE_DURATION_MS`) com um
//!   passe **final** autoritativo (beam search) sobre o turno;
//! - **gate de fala sustentada** (RMS) evita decodificar ruído/clique;
//! - **filtro de alucinação** no texto decodificado;
//! - **telemetria de RTF** (tempo real) por passe.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::audio::{has_sustained_speech, AudioLevel, TARGET_SAMPLE_RATE};
use crate::domain::transcription::{
    is_likely_hallucination, Speaker, SpeechQuality, TranscriptSegment,
};
use crate::error::GhostWhisperError;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::repositories::TranscriptionsRepository;
use crate::ports::session_controller::AudioChannel;
use crate::ports::speech_to_text::SpeechToText;

/// Tamanho mínimo do buffer para agir (ms).
const MIN_BUFFER_MS: i64 = 250;
/// Áudio mínimo do turno para valer uma decodificação (s).
/// Perssua: `SERVER_MIN_AUDIO_DURATION_SEC = 0.35`.
const MIN_WINDOW_MS: i64 = 350;
/// Passo mínimo de áudio novo entre prévias.
/// Perssua: `SERVER_WINDOW_DURATION_SEC = 0.75`.
const PREVIEW_STEP_MS: i64 = 750;
/// Primeira prévia só depois disto de turno.
/// Perssua: `INITIAL_PREVIEW_MS = 1200`.
const INITIAL_PREVIEW_MS: i64 = 1_200;
/// Força uma prévia quando nada é processado por este tempo.
/// Perssua: `SERVER_IDLE_FORCE_PROCESS_MS = 1000`.
const IDLE_FORCE_MS: u64 = 1_000;
/// Silêncio (ms) que encerra o turno.
/// Perssua: `VAD_SILENCE_DURATION_MS = 1000`.
const SILENCE_MS: i64 = 1_000;
/// Fala mínima do turno para virar segmento final.
/// Perssua: `FINAL_UTTERANCE_MIN_SEC = 0.8`.
const MIN_SEGMENT_MS: i64 = 800;
/// Teto do turno em fala contínua.
/// Perssua: `MAX_CONTEXT_MS = 55000`.
const MAX_TURN_MS: i64 = 55_000;
/// Teto rígido do buffer de turno (backpressure).
/// Perssua: `FINAL_UTTERANCE_MAX_SEC = 60`.
const MAX_BUFFER_MS: i64 = 60_000;
/// Limiar RMS do gate de fala.
const SPEECH_RMS_THRESHOLD: f32 = 0.008;
/// Intervalo de verificação da thread trabalhadora.
const PROCESS_INTERVAL: Duration = Duration::from_millis(50);
/// Intervalo mínimo entre eventos de nível de áudio.
const LEVEL_INTERVAL: Duration = Duration::from_millis(100);
/// Sem áudio por este tempo ⇒ a fonte pode ter parado.
const AUDIO_STARVATION: Duration = Duration::from_secs(3);
/// Intervalo mínimo entre avisos de ausência de áudio.
const STARVATION_LOG_INTERVAL: Duration = Duration::from_secs(5);

/// Consome áudio de uma fonte e publica segmentos transcritos.
pub struct TranscriptionPipeline {
    session_id: String,
    speaker: Speaker,
    channel: AudioChannel,
    asr: Arc<dyn SpeechToText>,
    publisher: Arc<dyn EventPublisher>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    /// Buffer do turno atual (teto rígido).
    buffer: Mutex<VecDeque<f32>>,
    stop: AtomicBool,
    /// Índice do turno (usado no id estável do segmento).
    turn: AtomicU64,
    /// Deslocamento temporal acumulado dos turnos (para timestamps monotônicos).
    turn_base_ms: AtomicU64,
    dropped_samples: AtomicU64,
    last_level: Mutex<Instant>,
    last_audio: Mutex<Instant>,
    last_starvation_log: Mutex<Instant>,
    /// Comprimento do buffer (ms) na última decodificação — garante o passo.
    last_decode_ms: Mutex<i64>,
    /// Instante da última decodificação (dispara a prévia por ociosidade).
    last_decode_at: Mutex<Instant>,
    /// Fim (ms, relativo ao turno) da última janela com fala.
    last_voiced_end_ms: Mutex<i64>,
    /// Houve fala no turno atual (desde o último fechamento).
    had_speech: AtomicBool,
}

impl TranscriptionPipeline {
    /// Monta o pipeline para uma sessão/falante/canal.
    pub fn new(
        session_id: String,
        speaker: Speaker,
        channel: AudioChannel,
        asr: Arc<dyn SpeechToText>,
        publisher: Arc<dyn EventPublisher>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
    ) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            speaker,
            channel,
            asr,
            publisher,
            transcriptions,
            buffer: Mutex::new(VecDeque::new()),
            stop: AtomicBool::new(false),
            turn: AtomicU64::new(0),
            turn_base_ms: AtomicU64::new(0),
            dropped_samples: AtomicU64::new(0),
            last_level: Mutex::new(now.checked_sub(LEVEL_INTERVAL).unwrap_or(now)),
            last_audio: Mutex::new(now),
            last_starvation_log: Mutex::new(
                now.checked_sub(STARVATION_LOG_INTERVAL).unwrap_or(now),
            ),
            last_decode_ms: Mutex::new(0),
            last_decode_at: Mutex::new(now),
            last_voiced_end_ms: Mutex::new(0),
            had_speech: AtomicBool::new(false),
        }
    }

    /// Inicia a thread trabalhadora.
    pub fn start(self: &Arc<Self>) -> Arc<Self> {
        let this = Arc::clone(self);
        thread::spawn(move || {
            while !this.stop.load(Ordering::Relaxed) {
                thread::sleep(PROCESS_INTERVAL);
                this.warn_if_starved();
                this.tick();
            }
        });
        Arc::clone(self)
    }

    /// Sinaliza a parada da thread trabalhadora.
    ///
    /// Antes de parar, fecha o turno em aberto: a última frase ficaria no
    /// buffer e seria descartada (o usuário perceberia "faltou o final").
    pub fn stop(&self) {
        self.flush();
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Publica o áudio que sobrou no buffer como último segmento final.
    fn flush(&self) {
        if !self.had_speech() {
            return;
        }
        let snapshot: Vec<f32> = match self.buffer.lock() {
            Ok(buffer) => buffer.iter().copied().collect(),
            Err(_) => return,
        };
        self.finalize_turn(&snapshot, samples_to_ms(snapshot.len()));
    }

    /// Ingere um bloco de áudio e processa o turno uma vez.
    ///
    /// Atalho equivalente a [`consume`](AudioConsumer::consume) + [`tick`];
    /// existe para testes determinísticos sem a thread trabalhadora.
    pub fn tick(&self) {
        let snapshot: Vec<f32> = match self.buffer.lock() {
            Ok(buffer) => {
                if samples_to_ms(buffer.len()) < MIN_BUFFER_MS {
                    return;
                }
                buffer.iter().copied().collect()
            }
            Err(_) => return,
        };
        let buffered_ms = samples_to_ms(snapshot.len());
        let had_speech = self.had_speech();
        let silence_ms = buffered_ms - self.last_voiced_end_ms();

        // 1) Encerra o turno: teto de fala contínua ou silêncio suficiente.
        let forced = buffered_ms >= MAX_TURN_MS;
        let ended = had_speech && silence_ms >= SILENCE_MS && buffered_ms >= MIN_SEGMENT_MS;
        if forced || ended {
            let cut_ms = if forced { MAX_TURN_MS } else { buffered_ms };
            self.finalize_turn(&snapshot, cut_ms);
            return;
        }

        // 2) Prévias durante a fala (texto rolante do turno).
        self.maybe_preview(&snapshot, buffered_ms, had_speech);
    }

    /// Decodifica o turno crescente e publica a prévia revisável.
    fn maybe_preview(&self, snapshot: &[f32], buffered_ms: i64, had_speech: bool) {
        if !had_speech || buffered_ms < INITIAL_PREVIEW_MS || buffered_ms < MIN_WINDOW_MS {
            return;
        }
        let step_ready = buffered_ms - self.last_decode_ms() >= PREVIEW_STEP_MS;
        let idle_ready = self
            .last_decode_at
            .lock()
            .map(|last| last.elapsed() >= Duration::from_millis(IDLE_FORCE_MS))
            .unwrap_or(false);
        if !step_ready && !idle_ready {
            return;
        }

        let quality = SpeechQuality::Preview;
        let started = Instant::now();
        let decoded = self.decode_turn(snapshot, quality);
        let wall = started.elapsed().as_millis();
        log_rtf(self.channel, quality, buffered_ms, wall);

        let Some(decoded) = decoded else {
            self.set_last_decode(buffered_ms);
            return;
        };
        self.set_last_decode(buffered_ms);
        self.publish_turn(&decoded.text, buffered_ms, false);

        // Perssua (`applyRecognition`): se o VAD do whisper marcou o fim da
        // fala há ≥ SILENCE_MS, o turno terminou — mesmo com ruído de fundo
        // mantendo o RMS alto sem gerar silêncio real no buffer.
        if decoded
            .speech_end_ms
            .is_some_and(|end| buffered_ms - end >= SILENCE_MS)
        {
            self.finalize_turn(snapshot, buffered_ms);
        }
    }

    /// Fecha o turno: passe final autoritativo, persistência e limpeza.
    fn finalize_turn(&self, snapshot: &[f32], cut_ms: i64) {
        let end = ms_to_samples(cut_ms).min(snapshot.len());
        let audio_ms = samples_to_ms(end);
        let gate = audio_ms >= MIN_SEGMENT_MS
            && has_sustained_speech(&snapshot[..end], SPEECH_RMS_THRESHOLD);

        let produced = if gate {
            let quality = SpeechQuality::Final;
            let started = Instant::now();
            let text = self.decode_turn(&snapshot[..end], quality);
            let wall = started.elapsed().as_millis();
            log_rtf(self.channel, quality, audio_ms, wall);
            match text {
                Some(decoded) => self.publish_turn(&decoded.text, audio_ms, true),
                None => 0,
            }
        } else {
            0
        };

        // Consome o turno (ou o trecho até o teto) e avança a base temporal.
        let base = self.turn_base_ms.load(Ordering::Relaxed);
        self.turn_base_ms
            .store(base + audio_ms.max(0) as u64, Ordering::Relaxed);
        let remaining = {
            let mut guard = match self.buffer.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let take = end.min(guard.len());
            guard.drain(0..take);
            guard.len()
        };
        // Reposiciona o marcador de fala restante (caso o teto tenha cortado
        // no meio de uma fala contínua).
        let leftover = (self.last_voiced_end_ms() - audio_ms).max(0);
        self.set_last_decode(0);
        if remaining == 0 {
            self.set_last_voiced_end_ms(0);
            self.had_speech.store(false, Ordering::Relaxed);
        } else {
            self.set_last_voiced_end_ms(leftover);
            self.had_speech.store(leftover > 0, Ordering::Relaxed);
        }
        self.turn.fetch_add(1, Ordering::Relaxed);

        tracing::debug!(
            channel = ?self.channel,
            buffered_ms = samples_to_ms(snapshot.len()),
            cut_ms,
            gate,
            produced,
            dropped = self.dropped_samples.load(Ordering::Relaxed),
            "turno finalizado"
        );
    }

    /// Decodifica um trecho como um turno único; filtra alucinação/vazio.
    ///
    /// Retorna o texto e o fim da última fala reportado pelo VAD (ms, relativo
    /// ao trecho) — usado para encerrar o turno quando a fala acaba antes do
    /// buffer (Perssua: `applyRecognition`/`endedInNoise`).
    fn decode_turn(&self, audio: &[f32], quality: SpeechQuality) -> Option<DecodedTurn> {
        match self.asr.transcribe(audio, quality) {
            Ok(spans) => {
                let joined = spans
                    .iter()
                    .map(|span| span.text.trim())
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if joined.is_empty() || is_likely_hallucination(&joined) {
                    if !joined.is_empty() {
                        tracing::debug!("texto descartado como alucinação: {joined:?}");
                    }
                    return None;
                }
                let speech_end_ms = spans.iter().map(|span| span.end_ms).max();
                Some(DecodedTurn {
                    text: joined,
                    speech_end_ms,
                })
            }
            // Modelo ainda carregando ou decodificação anterior ainda em curso:
            // estado normal e transitório (o próximo tick tenta de novo).
            Err(GhostWhisperError::AsrNotReady) | Err(GhostWhisperError::AsrBusy) => {
                tracing::trace!("ASR ainda indisponível; aguardando próxima janela");
                None
            }
            Err(error) => {
                tracing::warn!("ASR falhou: {error}");
                None
            }
        }
    }

    /// Publica o turno (interim ou final) com id estável por turno.
    ///
    /// A prévia e o final compartilham o mesmo id, então a UI substitui o texto
    /// em vez de duplicar. Só o final é persistido.
    fn publish_turn(&self, text: &str, audio_ms: i64, is_final: bool) -> usize {
        let turn = self.turn.load(Ordering::Relaxed);
        let base = self.turn_base_ms.load(Ordering::Relaxed) as i64;
        let segment = TranscriptSegment {
            id: format!("{}-t{turn}", self.session_id),
            session_id: self.session_id.clone(),
            speaker: self.speaker,
            text: text.to_string(),
            start_ms: base,
            end_ms: base + audio_ms,
            is_final,
        };
        if is_final {
            if let Err(error) = self.transcriptions.insert(&segment) {
                tracing::warn!("falha ao persistir segmento: {error}");
            }
        }
        self.publisher.emit_segment(&segment);
        1
    }

    /// Transcreve um buffer completo de uma vez (usado em testes/offline).
    #[cfg(test)]
    pub fn process(&self, samples: &[f32]) {
        self.consume(samples);
        self.tick();
    }

    /// Quantidade de amostras no buffer de turno (usado em testes).
    #[cfg(test)]
    pub fn buffered_samples(&self) -> usize {
        self.buffer.lock().map(|buffer| buffer.len()).unwrap_or(0)
    }

    fn had_speech(&self) -> bool {
        self.had_speech.load(Ordering::Relaxed)
    }
    fn last_decode_ms(&self) -> i64 {
        self.last_decode_ms.lock().map(|v| *v).unwrap_or(0)
    }
    fn last_voiced_end_ms(&self) -> i64 {
        self.last_voiced_end_ms.lock().map(|v| *v).unwrap_or(0)
    }
    fn set_last_voiced_end_ms(&self, v: i64) {
        if let Ok(mut slot) = self.last_voiced_end_ms.lock() {
            *slot = v;
        }
    }
    fn set_last_decode(&self, buffered_ms: i64) {
        if let Ok(mut slot) = self.last_decode_ms.lock() {
            *slot = buffered_ms;
        }
        if let Ok(mut slot) = self.last_decode_at.lock() {
            *slot = Instant::now();
        }
    }

    /// Emite o nível de áudio (throttle por tempo).
    fn emit_level(&self, samples: &[f32]) {
        let Some(level) = AudioLevel::from_samples(samples) else {
            return;
        };
        let should_emit = self
            .last_level
            .lock()
            .map(|mut last| {
                if last.elapsed() >= LEVEL_INTERVAL {
                    *last = Instant::now();
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if should_emit {
            self.publisher.emit_audio_level(self.channel, level);
        }
    }

    /// Avisa (com throttle) quando não chega áudio há muito tempo.
    fn warn_if_starved(&self) {
        let starved = self
            .last_audio
            .lock()
            .map(|last| last.elapsed() >= AUDIO_STARVATION)
            .unwrap_or(false);
        if !starved {
            return;
        }
        let should_log = self
            .last_starvation_log
            .lock()
            .map(|mut last| {
                if last.elapsed() >= STARVATION_LOG_INTERVAL {
                    *last = Instant::now();
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if should_log {
            tracing::warn!(
                channel = ?self.channel,
                "sem áudio há mais de {}s: a fonte de captura pode ter parado",
                AUDIO_STARVATION.as_secs()
            );
        }
    }
}

impl AudioConsumer for TranscriptionPipeline {
    fn consume(&self, samples: &[f32]) {
        let buffered_ms = {
            let Ok(mut buffer) = self.buffer.lock() else {
                return;
            };
            buffer.extend(samples.iter().copied());
            if samples_to_ms(buffer.len()) > MAX_BUFFER_MS {
                let cap = ms_to_samples(MAX_BUFFER_MS);
                let excess = buffer.len().saturating_sub(cap);
                buffer.drain(0..excess);
                let total = self
                    .dropped_samples
                    .fetch_add(excess as u64, Ordering::Relaxed)
                    + excess as u64;
                if total % (TARGET_SAMPLE_RATE as u64 * 5) < excess as u64 {
                    tracing::warn!(
                        channel = ?self.channel,
                        dropped_seconds = total / TARGET_SAMPLE_RATE as u64,
                        "backlog de áudio: descartando o mais antigo"
                    );
                }
            }
            samples_to_ms(buffer.len())
        };

        // VAD de amplitude (Perssua: `appendAudio` + RMS): marca fala e o fim
        // da última janela falada, base do corte por silêncio.
        if let Some(level) = AudioLevel::from_samples(samples) {
            if level.rms > SPEECH_RMS_THRESHOLD {
                self.had_speech.store(true, Ordering::Relaxed);
                self.set_last_voiced_end_ms(buffered_ms);
            }
        }

        if let Ok(mut last) = self.last_audio.lock() {
            *last = Instant::now();
        }
        self.emit_level(samples);
    }
}

/// Resultado de uma decodificação de turno.
struct DecodedTurn {
    /// Texto já filtrado (sem alucinação/vazio).
    text: String,
    /// Fim da última fala reportado pelo VAD do whisper (ms, relativo ao trecho).
    speech_end_ms: Option<i64>,
}

/// Registra telemetria de tempo real por passe.
fn log_rtf(channel: AudioChannel, quality: SpeechQuality, audio_ms: i64, wall_ms: u128) {
    let label = match quality {
        SpeechQuality::Preview => "preview",
        SpeechQuality::Final => "final",
    };
    let rtf = if wall_ms > 0 {
        format!("{:.2}x", audio_ms as f64 / wall_ms as f64)
    } else {
        "—".to_string()
    };
    tracing::debug!(
        channel = ?channel,
        audio_ms,
        wall_ms,
        rtf,
        "passe de transcrição concluído ({label})"
    );
}

fn ms_to_samples(ms: i64) -> usize {
    if ms <= 0 {
        return 0;
    }
    (ms as usize * TARGET_SAMPLE_RATE as usize) / 1000
}

fn samples_to_ms(samples: usize) -> i64 {
    (samples as i64 * 1000) / TARGET_SAMPLE_RATE as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::domain::transcription::TranscribedSpan;
    use crate::error::Result as CoreResult;

    struct FakeAsr {
        text: String,
    }
    impl SpeechToText for FakeAsr {
        fn transcribe(
            &self,
            samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            Ok(vec![TranscribedSpan {
                text: self.text.clone(),
                start_ms: 0,
                end_ms: samples_to_ms(samples.len()),
            }])
        }
    }

    /// ASR que reporta o fim da fala cedo (como o VAD do whisper sob ruído de
    /// fundo), exercitando o fechamento por `endedInNoise`.
    struct EarlyEndAsr;
    impl SpeechToText for EarlyEndAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            Ok(vec![TranscribedSpan {
                text: "fala".into(),
                start_ms: 0,
                end_ms: 500,
            }])
        }
    }

    /// ASR que registra o tamanho de cada pedido e a qualidade.
    struct RecordingAsr {
        audio_sizes: Mutex<Vec<usize>>,
        qualities: Mutex<Vec<SpeechQuality>>,
    }
    impl RecordingAsr {
        fn new() -> Self {
            Self {
                audio_sizes: Mutex::new(Vec::new()),
                qualities: Mutex::new(Vec::new()),
            }
        }
        fn sizes(&self) -> Vec<usize> {
            self.audio_sizes.lock().expect("sizes").clone()
        }
        fn qualities(&self) -> Vec<SpeechQuality> {
            self.qualities.lock().expect("qualities").clone()
        }
    }
    impl SpeechToText for RecordingAsr {
        fn transcribe(
            &self,
            samples: &[f32],
            quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            self.audio_sizes.lock().expect("sizes").push(samples.len());
            self.qualities.lock().expect("qualities").push(quality);
            Ok(vec![TranscribedSpan {
                text: "fala".into(),
                start_ms: 0,
                end_ms: samples_to_ms(samples.len()),
            }])
        }
    }

    struct RecordingPublisher {
        segments: Mutex<Vec<TranscriptSegment>>,
    }
    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                segments: Mutex::new(Vec::new()),
            }
        }
        fn segments(&self) -> Vec<TranscriptSegment> {
            self.segments.lock().unwrap().clone()
        }
    }
    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, segment: &TranscriptSegment) {
            self.segments.lock().unwrap().push(segment.clone());
        }
        fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _text: &str) {}
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
    }

    fn pipeline(
        asr: Arc<dyn SpeechToText>,
    ) -> (
        TranscriptionPipeline,
        Arc<RecordingPublisher>,
        Arc<InMemoryTranscriptionsRepository>,
    ) {
        let publisher = Arc::new(RecordingPublisher::new());
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let pipeline = TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            asr,
            publisher.clone(),
            transcriptions.clone(),
        );
        (pipeline, publisher, transcriptions)
    }

    fn fake_pipeline(text: &str) -> (TranscriptionPipeline, Arc<RecordingPublisher>) {
        let (pipeline, publisher, _rows) = pipeline(Arc::new(FakeAsr {
            text: text.to_string(),
        }));
        (pipeline, publisher)
    }

    fn speech(ms: i64) -> Vec<f32> {
        vec![0.4f32; ms_to_samples(ms)]
    }
    fn silence(ms: i64) -> Vec<f32> {
        vec![0.0f32; ms_to_samples(ms)]
    }

    // --- Streaming: primeiro preview, passo e texto rolante ------------------

    #[test]
    fn first_preview_waits_for_initial_window() {
        let (pipeline, publisher) = fake_pipeline("olá");
        // 1,0 s de fala: abaixo de INITIAL_PREVIEW_MS (1,2 s) ⇒ nada.
        pipeline.consume(&speech(1_000));
        pipeline.tick();
        assert!(publisher.segments().is_empty(), "não devia pré-visualizar");

        // +0,3 s cruza o limiar ⇒ primeira prévia.
        pipeline.consume(&speech(300));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 1, "esperava a primeira prévia");
        assert!(!segments[0].is_final);
        assert_eq!(segments[0].id, "s1-t0");
    }

    #[test]
    fn preview_respects_step_and_decodes_growing_utterance() {
        let asr = Arc::new(RecordingAsr::new());
        let (pipeline, publisher, _rows) = pipeline(asr.clone());

        // 1,5 s de fala ⇒ prévia; depois 0,5 s (abaixo do passo) nada; +0,3 s ⇒ prévia.
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        pipeline.consume(&speech(500));
        pipeline.tick();
        pipeline.consume(&speech(300));
        pipeline.tick();

        let segments = publisher.segments();
        assert_eq!(segments.len(), 2, "esperava duas prévias");
        assert_eq!(segments[0].id, segments[1].id, "mesmo id de turno");

        // As prévias decodificam o turno que cresce (não uma janela fixa).
        let sizes = asr.sizes();
        assert!(sizes.len() >= 2);
        assert!(
            sizes[1] > sizes[0],
            "a segunda prévia devia ver mais áudio: {sizes:?}"
        );
    }

    #[test]
    fn silence_finalizes_the_turn_and_persists() {
        let (pipeline, publisher, rows) = pipeline(Arc::new(FakeAsr {
            text: "bom dia".into(),
        }));
        // Fala suficiente para uma prévia.
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        let turn_id = publisher.segments()[0].id.clone();
        assert!(!publisher.segments()[0].is_final);

        // 1 s de silêncio fecha o turno (mesmo id, agora final e persistido).
        pipeline.consume(&silence(1_000));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 2);
        assert!(segments[1].is_final);
        assert_eq!(segments[1].id, turn_id);
        assert_eq!(segments[1].text, "bom dia");
        assert_eq!(rows.all().len(), 1, "final persistido");
        assert_eq!(pipeline.buffered_samples(), 0, "buffer consumido");
    }

    #[test]
    fn continuous_speech_is_cut_at_max_turn() {
        let (pipeline, publisher) = fake_pipeline("longa");
        // 55 s de fala contínua: fecha pela teto, cortando exatamente MAX_TURN_MS.
        pipeline.consume(&speech(MAX_TURN_MS));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].is_final);
        assert_eq!(pipeline.buffered_samples(), 0);
    }

    #[test]
    fn idle_silence_emits_nothing() {
        let (pipeline, publisher) = fake_pipeline("x");
        pipeline.consume(&silence(2_000));
        pipeline.tick();
        assert!(publisher.segments().is_empty());
    }

    #[test]
    fn hallucination_is_dropped() {
        let (pipeline, publisher) = fake_pipeline("Thanks for watching!");
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        pipeline.consume(&silence(1_000));
        pipeline.tick();
        assert!(publisher.segments().is_empty());
    }

    #[test]
    fn short_blip_is_not_finalized() {
        let (pipeline, publisher) = fake_pipeline("oi");
        // 100 ms de fala (sem fala sustentada de 160 ms) + silêncio.
        pipeline.consume(&speech(100));
        pipeline.consume(&silence(1_000));
        pipeline.tick();
        assert!(publisher.segments().is_empty(), "ruído curto não vira fala");
    }

    #[test]
    fn trailing_noise_finalizes_via_vad_timestamps() {
        // RMS alto (ruído) por > 1 s sem silêncio real, mas o VAD do whisper diz
        // que a fala acabou em 500 ms ⇒ o turno fecha mesmo assim.
        let (pipeline, publisher, _rows) = pipeline(Arc::new(EarlyEndAsr));
        pipeline.consume(&speech(2_000));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 2, "prévia + final");
        assert!(!segments[0].is_final);
        assert!(segments[1].is_final);
        assert_eq!(segments[0].id, segments[1].id);
    }

    #[test]
    fn buffer_is_capped_with_drop_count() {
        let (pipeline, _publisher) = fake_pipeline("x");
        pipeline.consume(&vec![0.4f32; 80 * TARGET_SAMPLE_RATE as usize]);
        let remaining_ms = samples_to_ms(pipeline.buffer.lock().unwrap().len());
        assert!(
            remaining_ms <= MAX_BUFFER_MS,
            "teto excedido: {remaining_ms}ms"
        );
        assert!(pipeline.dropped_samples.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn stop_flushes_the_open_turn() {
        let (pipeline, publisher) = fake_pipeline("final");
        pipeline.consume(&speech(1_000));
        pipeline.stop();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 1, "o turno em aberto devia ser publicado");
        assert!(segments[0].is_final);
    }

    #[test]
    fn preview_is_greedy_and_final_is_beam() {
        let asr = Arc::new(RecordingAsr::new());
        let (pipeline, _publisher, _rows) = pipeline(asr.clone());
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        pipeline.consume(&silence(1_000));
        pipeline.tick();
        assert_eq!(
            asr.qualities(),
            vec![SpeechQuality::Preview, SpeechQuality::Final]
        );
    }

    // --- Integração real (baixa modelos + fala). Rode com `--ignored`. ---

    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn end_to_end_transcribes_jfk_speech() {
        use crate::adapters::speech::whisper_model_store::FileModelStore;
        use crate::adapters::speech::whisper_speech_to_text::WhisperSpeechToText;
        use crate::ports::model_store::ModelStore;

        let samples = download_jfk_wav();
        assert!(samples.len() > 16_000, "amostra de fala muito curta");

        let models_dir = std::env::temp_dir().join("ghostwhisper-models");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(models_dir));
        models.vad_model_path().expect("vad model");
        models.whisper_model_path("tiny").expect("whisper model");

        // O modelo agora é pré-carregado em background (ver `preload`).
        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&models),
            "tiny",
            "auto",
            None,
        ));
        whisper.preload();
        wait_until_asr_ready(&whisper);

        let publisher = Arc::new(RecordingPublisher::new());
        let pipeline = TranscriptionPipeline::new(
            "jfk".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            whisper,
            publisher.clone(),
            Arc::new(InMemoryTranscriptionsRepository::new()),
        );

        for _ in 0..4 {
            pipeline.process(&samples);
        }

        let segments = publisher.segments();
        let text = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        assert!(text.contains("country"), "transcrição inesperada: {text:?}");
    }

    /// Aguarda o pré-carregamento do modelo concluir (thread de background).
    fn wait_until_asr_ready(
        whisper: &Arc<crate::adapters::speech::whisper_speech_to_text::WhisperSpeechToText>,
    ) {
        for _ in 0..1_200 {
            if whisper.is_ready() {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!("tempo esgotado esperando o modelo whisper ficar pronto");
    }

    fn download_jfk_wav() -> Vec<f32> {
        let url = "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav";
        let mut reader = ureq::get(url).call().expect("baixar jfk.wav").into_reader();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut bytes).expect("ler wav");
        parse_wav_pcm16_mono(&bytes)
    }

    fn parse_wav_pcm16_mono(bytes: &[u8]) -> Vec<f32> {
        assert!(
            &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
            "WAV inválido"
        );
        let mut pos = 12;
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            let body = pos + 8;
            if id == &b"data"[..] {
                let end = (body + size).min(bytes.len());
                return bytes[body..end]
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                    .collect();
            }
            pos = body + size + (size & 1);
        }
        panic!("chunk 'data' não encontrado no WAV");
    }
}
