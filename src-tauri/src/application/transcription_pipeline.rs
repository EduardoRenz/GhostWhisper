//! Pipeline de transcrição: buffer → VAD → ASR → eventos.
//!
//! Modelo de streaming (equivalente ao do `WhisperSession` do Perssua):
//! - **um buffer de turno** (`utterance`) acumula o áudio do turno atual, com
//!   teto rígido (descarta o mais antigo em caso de atraso);
//! - **duas passagens**: *preview* barata e **revisável** durante a fala
//!   (`is_final = false`) e um **passe final** autoritativo sobre o turno
//!   inteiro quando a fala termina (`is_final = true`);
//! - **gate de fala sustentada** (RMS) antes de chamar o Whisper — rejeita
//!   clique/ruído sem custo de decodificação;
//! - **filtro de alucinação** no texto decodificado;
//! - **telemetria de RTF** (tempo real) por passe, para diagnosticar lentidão.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::audio::{has_sustained_speech, AudioLevel, TARGET_SAMPLE_RATE};
use crate::domain::transcription::{
    is_likely_hallucination, Speaker, SpeechQuality, SpeechSpan, TranscriptSegment,
};
use crate::error::GhostWhisperError;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::repositories::TranscriptionsRepository;
use crate::ports::session_controller::AudioChannel;
use crate::ports::speech_to_text::SpeechToText;
use crate::ports::voice_activity::VoiceActivityDetector;

/// Tamanho mínimo do buffer para rodar o VAD (ms).
const MIN_BUFFER_MS: i64 = 250;
/// Intervalo entre avaliações do VAD (ms) — evita custo repetido.
const VAD_INTERVAL_MS: i64 = 250;
/// Passo mínimo de áudio novo entre previews (ms) — evita redecodear sempre.
const PREVIEW_STEP_MS: i64 = 900;
/// Janela decodificada no preview (ms). O preview só mostra o que está sendo
/// dito *agora*: decodificar o turno inteiro a cada passo custa cada vez mais e
/// satura a CPU (o turno completo é decodificado uma vez, no passe final).
const PREVIEW_WINDOW_MS: i64 = 2_000;
/// Fala mínima para virar segmento (ms).
const MIN_SEGMENT_MS: i64 = 300;
/// Silêncio necessário após a fala para fechar o turno (ms).
const SILENCE_MARGIN_MS: i64 = 200;
/// Teto do turno (ms) quando a fala é contínua sem pausas.
const MAX_TURN_MS: i64 = 10_000;
/// Teto rígido do buffer de turno (ms) — backpressure.
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
    vad: Arc<dyn VoiceActivityDetector>,
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
    /// Quantidade de amostras no buffer quando o VAD rodou pela última vez.
    /// A contagem é *por crescimento*, não absoluta: ao consumir um turno o
    /// buffer encolhe e o próximo VAD roda de imediato.
    last_vad_samples: Mutex<usize>,
    last_preview_ms: Mutex<i64>,
}

impl TranscriptionPipeline {
    /// Monta o pipeline para uma sessão/falante/canal.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        speaker: Speaker,
        channel: AudioChannel,
        vad: Arc<dyn VoiceActivityDetector>,
        asr: Arc<dyn SpeechToText>,
        publisher: Arc<dyn EventPublisher>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
    ) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            speaker,
            channel,
            vad,
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
            last_vad_samples: Mutex::new(0),
            last_preview_ms: Mutex::new(-PREVIEW_STEP_MS),
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
    /// buffer e seria descartada (o usuário perceivedia "faltou o final").
    pub fn stop(&self) {
        self.flush();
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Publica o áudio que sobrou no buffer como último segmento final.
    fn flush(&self) {
        let snapshot: Vec<f32> = match self.buffer.lock() {
            Ok(buffer) => buffer.iter().copied().collect(),
            Err(_) => return,
        };
        let audio_ms = samples_to_ms(snapshot.len());
        if audio_ms < MIN_SEGMENT_MS || !has_sustained_speech(&snapshot, SPEECH_RMS_THRESHOLD) {
            return;
        }
        let started = Instant::now();
        let text = self.decode_turn(&snapshot, SpeechQuality::Final);
        log_rtf(
            self.channel,
            SpeechQuality::Final,
            audio_ms,
            started.elapsed().as_millis(),
        );
        if let Some(text) = text {
            self.publish_turn(&text, audio_ms, true);
        }
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
    }

    /// Ingere um bloco de áudio e processa o turno uma vez.
    ///
    /// Atalho equivalente a [`consume`](AudioConsumer::consume) + [`tick`];
    /// existe para testes determinísticos sem a thread trabalhadora.
    ///
    /// Público para permitir testes determinísticos sem a thread.
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

        // VAD com throttle por *crescimento* do buffer: roda quando ganhou
        // VAD_STEP_SAMPLES, e também quando o buffer encolheu (um turno foi
        // consumido). Comparar milissegundos absolutos deixava o VAD bloqueado
        // até o buffer passar o pico do turno anterior.
        let buffered_samples = snapshot.len();
        let last_vad = self.last_vad_samples();
        let grew = buffered_samples.saturating_sub(last_vad);
        if buffered_samples >= last_vad && grew < ms_to_samples(VAD_INTERVAL_MS) {
            return;
        }
        self.set_last_vad_samples(buffered_samples);
        let vad_started = Instant::now();
        let spans = match self.vad.speech_segments(&snapshot) {
            Ok(spans) => spans,
            Err(error) => {
                tracing::warn!("VAD falhou: {error}");
                return;
            }
        };
        let vad_ms = vad_started.elapsed().as_millis();

        let Some(cut_ms) = turn_cut_ms(&spans, buffered_ms, SILENCE_MARGIN_MS, MAX_TURN_MS) else {
            // Turno em andamento: tenta um preview barato.
            self.maybe_preview(&snapshot, buffered_ms);
            return;
        };

        // Turno terminou: passe final autoritativo sobre o trecho [0..cut].
        let end = ms_to_samples(cut_ms).min(snapshot.len());
        let gate = cut_ms >= MIN_SEGMENT_MS
            && has_sustained_speech(&snapshot[..end], SPEECH_RMS_THRESHOLD);
        let audio_ms = samples_to_ms(end);
        let produced = if gate {
            let quality = SpeechQuality::Final;
            let started = Instant::now();
            let text = self.decode_turn(&snapshot[..end], quality);
            let wall = started.elapsed().as_millis();
            log_rtf(self.channel, quality, audio_ms, wall);
            match text {
                Some(text) => self.publish_turn(&text, audio_ms, true),
                None => 0,
            }
        } else {
            0
        };

        // Consome o turno e avança a base temporal.
        let base = self.turn_base_ms.load(Ordering::Relaxed);
        self.turn_base_ms
            .store(base + cut_ms as u64, Ordering::Relaxed);
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.drain(0..end);
        }
        self.turn.fetch_add(1, Ordering::Relaxed);
        self.set_last_preview_ms(0);

        tracing::debug!(
            channel = ?self.channel,
            buffered_ms,
            cut_ms,
            gate,
            produced,
            vad_ms,
            dropped = self.dropped_samples.load(Ordering::Relaxed),
            "turno finalizado"
        );
    }

    /// Publica (ou atualiza) um preview do turno em andamento.
    fn maybe_preview(&self, snapshot: &[f32], buffered_ms: i64) {
        // Gate: fala sustentada e passo mínimo desde o último preview.
        if buffered_ms - self.last_preview_ms() < PREVIEW_STEP_MS {
            return;
        }
        // O preview cobre só a janela recente (o que está sendo dito agora):
        // o custo fica constante mesmo com o turno crescendo.
        let window_samples = ms_to_samples(PREVIEW_WINDOW_MS);
        let start = snapshot.len().saturating_sub(window_samples);
        let window = &snapshot[start..];
        if !has_sustained_speech(window, SPEECH_RMS_THRESHOLD) {
            return;
        }
        self.set_last_preview_ms(buffered_ms);
        let quality = SpeechQuality::Preview;
        let started = Instant::now();
        let text = self.decode_turn(window, quality);
        let wall = started.elapsed().as_millis();
        log_rtf(self.channel, quality, samples_to_ms(window.len()), wall);
        if let Some(text) = text {
            self.publish_turn(&text, samples_to_ms(window.len()), false);
        }
    }

    /// Decodifica um trecho como um turno único; filtra alucinação/vazio.
    fn decode_turn(&self, audio: &[f32], quality: SpeechQuality) -> Option<String> {
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
                Some(joined)
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
    /// O preview e o final compartilham o mesmo id, então a UI substitui o texto
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

    fn last_vad_samples(&self) -> usize {
        self.last_vad_samples.lock().map(|v| *v).unwrap_or(0)
    }
    fn set_last_vad_samples(&self, samples: usize) {
        if let Ok(mut slot) = self.last_vad_samples.lock() {
            *slot = samples;
        }
    }
    fn last_preview_ms(&self) -> i64 {
        self.last_preview_ms
            .lock()
            .map(|v| *v)
            .unwrap_or(-PREVIEW_STEP_MS)
    }
    fn set_last_preview_ms(&self, v: i64) {
        if let Ok(mut slot) = self.last_preview_ms.lock() {
            *slot = v;
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
        if let Ok(mut buffer) = self.buffer.lock() {
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
        }
        if let Ok(mut last) = self.last_audio.lock() {
            *last = Instant::now();
        }
        self.emit_level(samples);
    }
}

/// Decide o ponto de corte do turno (ms) a partir dos trechos de fala.
///
/// `None` = turno ainda em andamento. Corta no fim da última fala seguida de
/// `silence_margin_ms` de silêncio; se a fala for contínua, força em `max_ms`.
fn turn_cut_ms(
    spans: &[SpeechSpan],
    buffered_ms: i64,
    silence_margin_ms: i64,
    max_ms: i64,
) -> Option<i64> {
    if buffered_ms >= max_ms {
        // Corta *no* teto, nunca além: o passe final fica com custo limitado.
        // Antes devolvia o buffer inteiro, que podia ter crescido muito entre
        // dois ticks (o VAD é throttled) e travava o ASR.
        return Some(max_ms);
    }
    spans
        .iter()
        .rev()
        .find(|span| buffered_ms - span.end_ms >= silence_margin_ms)
        .map(|span| span.end_ms)
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
    use crate::domain::transcription::TranscribedSpan;
    use crate::error::Result as CoreResult;

    struct FakeVad {
        spans: Vec<SpeechSpan>,
    }
    impl VoiceActivityDetector for FakeVad {
        fn speech_segments(&self, _samples: &[f32]) -> CoreResult<Vec<SpeechSpan>> {
            Ok(self.spans.clone())
        }
    }

    struct FakeAsr {
        text: String,
    }
    impl SpeechToText for FakeAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            Ok(vec![TranscribedSpan {
                text: self.text.clone(),
                start_ms: 0,
                end_ms: 500,
            }])
        }
    }

    /// ASR que registra quantos samples cada pedido levou (e o texto pedido).
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
                end_ms: 500,
            }])
        }
    }

    struct RecordingPublisher {
        segments: Mutex<Vec<TranscriptSegment>>,
        levels: Mutex<Vec<AudioLevel>>,
        states: Mutex<Vec<crate::ports::event_publisher::SessionStateEvent>>,
    }
    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                segments: Mutex::new(Vec::new()),
                levels: Mutex::new(Vec::new()),
                states: Mutex::new(Vec::new()),
            }
        }
    }
    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, segment: &TranscriptSegment) {
            self.segments.lock().unwrap().push(segment.clone());
        }
        fn emit_audio_level(&self, _channel: AudioChannel, level: AudioLevel) {
            self.levels.lock().unwrap().push(level);
        }
        fn emit_session_state(&self, state: &crate::ports::event_publisher::SessionStateEvent) {
            self.states.lock().unwrap().push(state.clone());
        }
        fn emit_insight(&self, _text: &str) {}
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
    }

    fn pipeline(
        spans: Vec<SpeechSpan>,
        text: &str,
    ) -> (TranscriptionPipeline, Arc<RecordingPublisher>) {
        use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;

        let publisher = Arc::new(RecordingPublisher::new());
        let pipeline = TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            Arc::new(FakeVad { spans }),
            Arc::new(FakeAsr {
                text: text.to_string(),
            }),
            publisher.clone(),
            Arc::new(InMemoryTranscriptionsRepository::new()),
        );
        (pipeline, publisher)
    }

    /// Monta um pipeline com ASR/VAD injetados (para testes de regressão).
    fn pipeline_with(
        asr: Arc<dyn SpeechToText>,
        vad: Arc<dyn VoiceActivityDetector>,
    ) -> TranscriptionPipeline {
        use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;

        TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            vad,
            asr,
            Arc::new(RecordingPublisher::new()),
            Arc::new(InMemoryTranscriptionsRepository::new()),
        )
    }

    // --- Corte / streaming ---------------------------------------------------

    fn span(start_ms: i64, end_ms: i64) -> SpeechSpan {
        SpeechSpan { start_ms, end_ms }
    }

    #[test]
    fn cut_waits_on_short_buffer() {
        // 1 s com fala até 950 ms: só 50 ms de silêncio (< margem) ⇒ espera.
        assert_eq!(turn_cut_ms(&[span(0, 950)], 1000, 200, 10_000), None);
    }

    #[test]
    fn cut_at_completed_span_end() {
        assert_eq!(
            turn_cut_ms(&[span(0, 500), span(900, 1500)], 2000, 200, 10_000),
            Some(1500)
        );
    }

    #[test]
    fn cut_waits_when_speech_at_end() {
        assert_eq!(turn_cut_ms(&[span(0, 2000)], 2000, 200, 10_000), None);
    }

    #[test]
    fn cut_forced_at_max() {
        assert_eq!(
            turn_cut_ms(&[span(0, 10_000)], 10_000, 200, 10_000),
            Some(10_000)
        );
    }

    #[test]
    fn cut_waits_on_pure_silence_until_max() {
        assert_eq!(turn_cut_ms(&[], 2000, 200, 10_000), None);
        assert_eq!(turn_cut_ms(&[], 10_000, 200, 10_000), Some(10_000));
        // Teto é limite: nunca devolve o buffer inteiro.
        assert_eq!(turn_cut_ms(&[], 45_000, 200, 10_000), Some(10_000));
    }

    #[test]
    fn preview_publishes_interim_with_stable_id() {
        // Fala chegando (span toca o fim) + fala sustentada ⇒ preview.
        let (pipeline, publisher) = pipeline(vec![span(0, 3000)], "olá");
        // 3 s de tom forte (gate passa), span no fim ⇒ sem corte.
        pipeline.consume(&vec![0.4f32; 48_000]);
        pipeline.tick();

        let segments = publisher.segments.lock().unwrap().clone();
        assert_eq!(segments.len(), 1, "esperava um preview");
        assert!(!segments[0].is_final, "preview não é final");
        assert_eq!(segments[0].id, "s1-t0");
    }

    #[test]
    fn final_replaces_preview_and_persists() {
        // Fala concluída (span 0..1500, buffer 2 s ⇒ margem 500) ⇒ final.
        let (pipeline, publisher) = pipeline(vec![span(0, 1500)], "bom dia");
        pipeline.consume(&vec![0.4f32; 32_000]);
        pipeline.tick();

        let segments = publisher.segments.lock().unwrap().clone();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].is_final, "esperava final");
        assert_eq!(segments[0].text, "bom dia");
        // Buffer consumido até o corte.
        assert_eq!(pipeline.buffer.lock().unwrap().len(), 8_000);
    }

    #[test]
    fn buffer_is_capped_with_drop_count() {
        let (pipeline, _publisher) = pipeline(vec![], "");
        pipeline.consume(&vec![0.4f32; 80 * TARGET_SAMPLE_RATE as usize]);
        let remaining_ms = samples_to_ms(pipeline.buffer.lock().unwrap().len());
        assert!(
            remaining_ms <= MAX_BUFFER_MS,
            "teto excedido: {remaining_ms}ms"
        );
        assert!(pipeline.dropped_samples.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn idle_silence_emits_nothing() {
        let (pipeline, publisher) = pipeline(vec![], "");
        pipeline.consume(&vec![0.0f32; 32_000]);
        pipeline.tick();
        assert!(publisher.segments.lock().unwrap().is_empty());
    }

    #[test]
    fn hallucination_is_dropped() {
        let (pipeline, publisher) = pipeline(vec![span(0, 1500)], "Thanks for watching!");
        pipeline.consume(&vec![0.4f32; 32_000]);
        pipeline.tick();
        assert!(publisher.segments.lock().unwrap().is_empty());
    }

    #[test]
    fn turn_emits_preview_then_final_with_the_same_id() {
        // Fluxo completo: durante a fala sai um preview (revisável); ao
        // terminar o turno sai o final, que substitui o preview pelo mesmo id.
        let (pipeline, publisher) = pipeline(vec![span(0, 1000)], "olá mundo");
        pipeline.consume(&vec![0.4f32; 16_000]);
        pipeline.tick();
        let segments = publisher.segments.lock().unwrap();
        assert_eq!(segments.len(), 1, "esperava o preview");
        assert!(!segments[0].is_final, "o primeiro passe é o preview");
        let turn_id = segments[0].id.clone();
        drop(segments);

        // Silêncio fecha o turno.
        pipeline.consume(&vec![0.0f32; 8_000]);
        pipeline.tick();
        let segments = publisher.segments.lock().unwrap();
        assert_eq!(segments.len(), 2, "esperava o final");
        assert!(segments[1].is_final, "o segundo passe é o final");
        assert_eq!(segments[1].id, turn_id, "mesmo id de turno");
        assert_eq!(segments[1].text, segments[0].text);
    }

    #[test]
    fn ignores_short_spans_in_process() {
        let (pipeline, publisher) = pipeline(vec![span(0, 100)], "ruído");
        pipeline.process(&vec![0.4f32; 16_000]);
        assert!(publisher.segments.lock().unwrap().is_empty());
    }

    // --- Integração real (baixa modelos + fala). Rode com `--ignored`. ---

    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn end_to_end_transcribes_jfk_speech() {
        use crate::adapters::speech::silero_voice_activity::SileroVoiceActivityDetector;
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
            Arc::new(SileroVoiceActivityDetector::new(Arc::clone(&models))),
            whisper,
            publisher.clone(),
            Arc::new(
                crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository::new(),
            ),
        );

        for _ in 0..4 {
            pipeline.process(&samples);
        }

        let segments = publisher.segments.lock().unwrap();
        let text = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        assert!(text.contains("country"), "transcrição inesperada: {text:?}");
    }

    /// Regressão: o preview nunca decodifica o turno inteiro — só a janela
    /// recente. Decodificar o buffer completo a cada passo saturava a CPU e a
    /// transcrição parava depois da primeira linha.
    #[test]
    fn preview_decodes_only_the_recent_window() {
        let asr = Arc::new(RecordingAsr::new());
        // VAD sem fala (turno nunca fecha) => só préviews.
        // Fala em andamento (span aberto): o turno não fecha, só há preview.
        let ongoing = Arc::new(FakeVad {
            spans: vec![span(0, 600_000)],
        });
        let pipeline = pipeline_with(asr.clone(), ongoing);
        let window = ms_to_samples(PREVIEW_WINDOW_MS);

        // Fala contínua bem acima da janela do preview.
        let speech = vec![0.2_f32; ms_to_samples(8_000)];
        for _ in 0..8 {
            pipeline.process(&speech);
        }

        let previews: Vec<usize> = asr
            .sizes()
            .into_iter()
            .filter(|size| *size <= window + TARGET_SAMPLE_RATE as usize)
            .collect();
        assert!(!previews.is_empty(), "esperava ao menos um preview");
        for size in previews {
            assert!(
                size <= window + TARGET_SAMPLE_RATE as usize,
                "preview decodificou {size} amostras, acima da janela de {window}"
            );
        }
    }

    /// Regressão: após consumir um turno o VAD volta a rodar imediatamente.
    /// Com a contagem antiga (milissegundos absolutos) o VAD ficava travado
    /// até o buffer ultrapassar o pico do turno anterior — a transcrição
    /// parecia parar depois da primeira linha.
    #[test]
    fn vad_resumes_right_after_a_turn_is_consumed() {
        let spans = vec![SpeechSpan {
            start_ms: 0,
            end_ms: 1_500,
        }];
        let pipeline = pipeline_with(
            Arc::new(FakeAsr {
                text: "fala".into(),
            }),
            Arc::new(FakeVad {
                spans: spans.clone(),
            }),
        );

        let speech = vec![0.2_f32; ms_to_samples(1_600)];
        // Primeiro turno fecha (span + 200 ms de silêncio).
        pipeline.process(&speech);
        pipeline.process(&vec![0.0_f32; ms_to_samples(400)]);
        assert!(
            pipeline.buffered_samples() < speech.len(),
            "turno não consumido"
        );

        // Após consumir, um turno NOVO com pouco áudio deve ser processado.
        pipeline.process(&speech);
        pipeline.process(&vec![0.0_f32; ms_to_samples(400)]);
        assert!(
            pipeline.buffered_samples() < speech.len(),
            "segundo turno não fechado"
        );
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
