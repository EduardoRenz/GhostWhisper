//! Pipeline de transcrição: buffer de turno → ASR em streaming → eventos.
//!
//! Modelo de streaming transposto do **Perssua** (`electron/local-whisper`,
//! `WhisperSession.js` / `WhisperSessionStreaming.js`), que é a referência de
//! produto (BR-015):
//! - **um buffer de turno** acumula o áudio do turno atual (teto rígido de 30 s
//!   = `MAX_BUFFER_MS`, abaixo do penhasco de custo do whisper);
//! - **prévias frequentes** decodificam o turno que cresce (~a cada 0,75 s de
//!   áudio novo) e publicam um **texto rolante** revisável (`is_final = false`);
//! - a primeira prévia espera `INITIAL_PREVIEW_MS` (1,2 s), como no Perssua;
//! - o turno fecha após **1 s de silêncio** (`VAD_SILENCE_DURATION_MS`) com um
//!   passe **final** autoritativo (beam search) sobre o turno;
//! - **gate de fala sustentada** (RMS) evita decodificar ruído/clique;
//! - **filtro de alucinação** no texto decodificado;
//! - **telemetria de RTF** (tempo real) por passe.
//!
//! Arquitetura: a **decisão** é pura ([`turn_scheduler`]) e roda numa thread de
//! agendamento (a cada 50 ms); a **decodificação** roda numa thread de trabalho
//! própria e nunca bloqueia o agendamento. Sem isso, o fim do turno ficava preso
//! atrás de uma prévia em voo e o texto só parava de crescer quando a
//! decodificação terminava (o Perssua resolve o mesmo com `isProcessing` +
//! `pendingUtteranceComplete`).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::audio::{
    has_sustained_speech, AudioLevel, SPEECH_RMS_THRESHOLD, TARGET_SAMPLE_RATE,
};
use crate::domain::transcription::{
    is_likely_hallucination, strip_phrase_loop, Speaker, SpeechQuality, TranscribedSpan,
    TranscriptSegment,
};
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::repositories::TranscriptionsRepository;
use crate::ports::session_controller::AudioChannel;
use crate::ports::speech_to_text::SpeechToText;

use super::turn_scheduler::{self, Decision, TurnState, TurnTimings};

/// Tamanho mínimo do buffer para agir (ms).
const MIN_BUFFER_MS: i64 = 250;
/// Teto rígido do buffer de turno (backpressure).
///
/// Mantém o áudio vivo abaixo do penhasco de custo do whisper (`max_turn_ms`),
/// para que a próxima decodificação continue na faixa barata.
const MAX_BUFFER_MS: i64 = 30_000;
/// Intervalo de verificação da thread de agendamento.
const PROCESS_INTERVAL: Duration = Duration::from_millis(50);
/// Intervalo mínimo entre eventos de nível de áudio.
const LEVEL_INTERVAL: Duration = Duration::from_millis(100);
/// Sem áudio por este tempo ⇒ a fonte pode ter parado.
const AUDIO_STARVATION: Duration = Duration::from_secs(3);
/// Intervalo mínimo entre avisos de ausência de áudio.
const STARVATION_LOG_INTERVAL: Duration = Duration::from_secs(5);
/// Espera máxima pelo passe final em `stop`/`flush`.
const FLUSH_WAIT: Duration = Duration::from_secs(20);

/// Pedido de decodificação enviado à thread de trabalho.
struct DecodeRequest {
    /// Turno a que o áudio pertence (id estável do segmento).
    turn: u64,
    /// Base temporal do turno (ms) para timestamps monotônicos.
    base_ms: i64,
    quality: SpeechQuality,
    /// Áudio do turno (já recortado em `cut_ms`).
    audio: Vec<f32>,
    /// Milissegundos do turno cobertos por `audio`.
    cut_ms: i64,
}

/// Pedido concluído, com o contexto necessário para publicar.
struct DecodeOutcome {
    request: DecodeRequest,
    result: Result<Vec<TranscribedSpan>>,
    wall_ms: u128,
}

/// Como as decodificações são executadas.
enum DecodeMode {
    /// Testes: decodifica na própria thread (determinístico, sem corrida).
    Inline,
    /// Produção: thread de trabalho própria.
    Worker(mpsc::Sender<DecodeRequest>),
}

/// Consome áudio de uma fonte e publica segmentos transcritos.
pub struct TranscriptionPipeline {
    session_id: String,
    speaker: Speaker,
    channel: AudioChannel,
    asr: Arc<dyn SpeechToText>,
    publisher: Arc<dyn EventPublisher>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    /// Limiares de tempo do turno.
    timings: TurnTimings,
    /// Deslocamento do relógio da sessão (ms) no início desta captura.
    ///
    /// Os timestamps dos segmentos são relativos ao **início da sessão**, não ao
    /// início da captura: ao retomar/continuar uma sessão a linha temporal segue
    /// de onde estava em vez de reiniciar em 0 (BR-019).
    time_origin_ms: i64,
    /// Buffer do turno atual (teto rígido).
    buffer: Mutex<VecDeque<f32>>,
    stop: AtomicBool,
    /// Índice do turno (usado no id estável do segmento).
    turn: AtomicU64,
    /// Deslocamento temporal acumulado dos turnos (para timestamps monotônicos).
    turn_base_ms: AtomicU64,
    /// Decodificações em andamento.
    in_flight: AtomicUsize,
    dropped_samples: AtomicU64,
    last_level: Mutex<Instant>,
    last_audio: Mutex<Instant>,
    last_starvation_log: Mutex<Instant>,
    /// Comprimento do buffer (ms) na última decodificação **bem-sucedida**.
    last_decode_ms: Mutex<i64>,
    /// Instante da última tentativa de decodificação.
    last_attempt_at: Mutex<Instant>,
    /// A última tentativa falhou (transitório): só o relógio de ociosidade
    /// re-arma uma nova prévia (evita re-tentar a cada 50 ms).
    last_attempt_failed: AtomicBool,
    /// Fim (ms, relativo ao turno) da última janela com fala.
    last_voiced_end_ms: Mutex<i64>,
    /// Houve fala no turno atual (desde o último fechamento).
    had_speech: AtomicBool,
    /// Turnos com passe final pendente (aguardando o resultado).
    ///
    /// É um conjunto, não um único turno: o `tick` e o resultado da prévia podem
    /// fechar turnos diferentes em paralelo, e um final nunca pode ser descartado
    /// porque outro turno começou.
    pending_finals: Mutex<Vec<u64>>,
    /// Último texto de prévia publicado, por turno (fallback se o final falhar).
    last_preview: Mutex<Option<(u64, String)>>,
    /// Como executar as decodificações.
    mode: Mutex<DecodeMode>,
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
        Self::with_timings_and_all(
            session_id,
            speaker,
            channel,
            asr,
            publisher,
            transcriptions,
            TurnTimings::default(),
        )
    }

    /// Substitui os limiares de tempo do turno (produção usa
    /// [`TurnTimings::from_env`]).
    pub fn with_timings(mut self, timings: TurnTimings) -> Self {
        self.timings = timings;
        self
    }

    /// Define o deslocamento do relógio da sessão (ms) nesta captura.
    ///
    /// `0` = sessão nova; ao retomar/continuar, passe
    /// `agora_ms - session.started_at_ms` (BR-019).
    pub fn with_time_origin(mut self, origin_ms: i64) -> Self {
        self.time_origin_ms = origin_ms.max(0);
        self
    }

    /// Construtor completo.
    #[allow(clippy::too_many_arguments)]
    fn with_timings_and_all(
        session_id: String,
        speaker: Speaker,
        channel: AudioChannel,
        asr: Arc<dyn SpeechToText>,
        publisher: Arc<dyn EventPublisher>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        timings: TurnTimings,
    ) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            speaker,
            channel,
            asr,
            publisher,
            transcriptions,
            timings,
            time_origin_ms: 0,
            buffer: Mutex::new(VecDeque::new()),
            stop: AtomicBool::new(false),
            turn: AtomicU64::new(0),
            turn_base_ms: AtomicU64::new(0),
            in_flight: AtomicUsize::new(0),
            dropped_samples: AtomicU64::new(0),
            last_level: Mutex::new(now.checked_sub(LEVEL_INTERVAL).unwrap_or(now)),
            last_audio: Mutex::new(now),
            last_starvation_log: Mutex::new(
                now.checked_sub(STARVATION_LOG_INTERVAL).unwrap_or(now),
            ),
            last_decode_ms: Mutex::new(0),
            last_attempt_at: Mutex::new(now),
            last_attempt_failed: AtomicBool::new(false),
            last_voiced_end_ms: Mutex::new(0),
            had_speech: AtomicBool::new(false),
            pending_finals: Mutex::new(Vec::new()),
            last_preview: Mutex::new(None),
            mode: Mutex::new(DecodeMode::Inline),
        }
    }

    /// Inicia as threads (decodificação + agendamento).
    pub fn start(self: &Arc<Self>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<DecodeRequest>();
        if let Ok(mut guard) = self.mode.lock() {
            *guard = DecodeMode::Worker(tx);
        }

        {
            let this = Arc::clone(self);
            let asr = Arc::clone(&self.asr);
            let spawned = thread::Builder::new()
                .name("whisper-decode".into())
                .spawn(move || this.run_decode_worker(rx, asr));
            if let Err(error) = spawned {
                tracing::error!("falha ao iniciar thread de decodificação: {error}");
            }
        }

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

    /// Sinaliza a parada das threads.
    ///
    /// Antes de parar, fecha o turno em aberto: a última frase ficaria no
    /// buffer e seria descartada (o usuário perceberia "faltou o final").
    pub fn stop(&self) {
        self.flush();
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Publica o áudio que sobrou no buffer como último segmento final.
    fn flush(&self) {
        let (turn, base_ms) = self.current_turn_context();
        let buffered_ms = self.buffered_ms();
        if !self.had_speech() || buffered_ms < MIN_BUFFER_MS {
            // Nada será publicado: se uma prévia já foi mostrada, ela sai agora
            // (ISS-021) — era o caso da bolha "em andamento" pendurada.
            self.retract_turn(turn, base_ms, buffered_ms);
            return;
        }
        let audio = self.snapshot_upto(buffered_ms);
        self.begin_final(turn, base_ms, audio, buffered_ms);
        self.wait_for_final(FLUSH_WAIT);
    }

    /// Aguarda os passes finais pendentes (usado em `stop`/`flush`).
    fn wait_for_final(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if self.pending_finals_empty() {
                return;
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    "passe final não concluiu em {}s; seguindo",
                    timeout.as_secs()
                );
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Thread de trabalho: decodifica em série e publica o resultado.
    fn run_decode_worker(
        self: Arc<Self>,
        rx: mpsc::Receiver<DecodeRequest>,
        asr: Arc<dyn SpeechToText>,
    ) {
        while let Ok(request) = rx.recv() {
            let started = Instant::now();
            let result = asr.transcribe(&request.audio, request.quality);
            let wall_ms = started.elapsed().as_millis();
            // Libera o agendamento antes de publicar: `handle_outcome` pode
            // enfileirar o passe final.
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            self.handle_outcome(DecodeOutcome {
                request,
                result,
                wall_ms,
            });
        }
    }

    /// Enfileira uma decodificação (worker) ou a executa na hora (testes).
    fn dispatch(&self, request: DecodeRequest) {
        let sender = match self.mode.lock() {
            Ok(guard) => match &*guard {
                DecodeMode::Worker(tx) => Some(tx.clone()),
                DecodeMode::Inline => None,
            },
            Err(_) => None,
        };

        self.in_flight.fetch_add(1, Ordering::SeqCst);
        match sender {
            Some(tx) => {
                if tx.send(request).is_err() {
                    self.in_flight.fetch_sub(1, Ordering::SeqCst);
                    if let Ok(mut slot) = self.pending_finals.lock() {
                        slot.clear();
                    }
                    tracing::warn!("thread de decodificação encerrada; pedido descartado");
                }
            }
            None => {
                let started = Instant::now();
                let result = self.asr.transcribe(&request.audio, request.quality);
                let wall_ms = started.elapsed().as_millis();
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                self.handle_outcome(DecodeOutcome {
                    request,
                    result,
                    wall_ms,
                });
            }
        }
    }

    /// Ingere um bloco de áudio e processa o turno uma vez.
    ///
    /// Atalho equivalente a [`consume`](AudioConsumer::consume) + [`tick`];
    /// existe para testes determinísticos sem a thread de agendamento.
    pub fn tick(&self) {
        let buffered_ms = self.buffered_ms();
        if buffered_ms < MIN_BUFFER_MS {
            return;
        }

        let state = TurnState {
            buffered_ms,
            had_speech: self.had_speech(),
            last_voiced_end_ms: self.last_voiced_end_ms(),
            last_decode_ms: self.last_decode_ms(),
            ms_since_last_attempt: self.ms_since_last_attempt(),
            decode_in_flight: self.in_flight.load(Ordering::SeqCst) > 0,
            final_requested: self.final_pending_for_current_turn(),
            last_attempt_failed: self.last_attempt_failed.load(Ordering::Relaxed),
        };

        match turn_scheduler::decide(&state, &self.timings) {
            Decision::Wait => {}
            Decision::Preview { cut_ms } => {
                let (turn, base_ms) = self.current_turn_context();
                self.mark_attempt();
                let audio = self.snapshot_upto(cut_ms);
                self.dispatch(DecodeRequest {
                    turn,
                    base_ms,
                    quality: SpeechQuality::Preview,
                    audio,
                    cut_ms,
                });
            }
            Decision::Finalize { cut_ms } => {
                let (turn, base_ms) = self.current_turn_context();
                let audio = self.snapshot_upto(cut_ms);
                self.begin_final(turn, base_ms, audio, cut_ms);
            }
        }
    }

    /// Fecha o turno: consome o áudio, enfileira o passe final autoritativo.
    fn begin_final(&self, turn: u64, base_ms: i64, audio: Vec<f32>, cut_ms: i64) {
        self.finalize_turn(turn, base_ms, audio, cut_ms, None);
    }

    /// Fecha o turno **reusando** o texto da prévia (Perssua: `endedInNoise`).
    ///
    /// Quando o VAD nativo do whisper já marcou o fim da fala dentro da própria
    /// prévia, o texto decodificado cobre o turno inteiro: promovê-lo a final
    /// evita um segundo passe beam search no fechamento (BR-029).
    fn begin_final_reusing(
        &self,
        turn: u64,
        base_ms: i64,
        audio: Vec<f32>,
        cut_ms: i64,
        text: String,
    ) {
        self.finalize_turn(turn, base_ms, audio, cut_ms, Some(text));
    }

    /// Fecha o turno; `reuse` promove o texto já decodificado a final.
    fn finalize_turn(
        &self,
        turn: u64,
        base_ms: i64,
        audio: Vec<f32>,
        cut_ms: i64,
        reuse: Option<String>,
    ) {
        // O `tick` e o resultado da prévia podem disparar o fechamento do mesmo
        // turno em paralelo; a checagem+registro sob o mesmo lock garante que
        // só um fecha.
        match self.pending_finals.lock() {
            Ok(mut pending) => {
                if pending.contains(&turn) {
                    return;
                }
                pending.push(turn);
            }
            Err(_) => return,
        }
        self.mark_attempt();

        // Consome o turno imediatamente: o áudio já está no pedido e a captura
        // segue para o próximo turno (Perssua: `prepareNextUtterance`).
        let drained_ms = {
            let mut guard = match self.buffer.lock() {
                Ok(guard) => guard,
                // Lock envenenado: não deixa o turno marcado como pendente.
                Err(_) => {
                    if let Ok(mut pending) = self.pending_finals.lock() {
                        pending.retain(|pending_turn| *pending_turn != turn);
                    }
                    return;
                }
            };
            let take = ms_to_samples(cut_ms).min(guard.len());
            guard.drain(0..take);
            samples_to_ms(take)
        };
        let _ = self
            .turn_base_ms
            .fetch_add(drained_ms.max(0) as u64, Ordering::Relaxed);
        self.set_last_decode(0);
        self.last_attempt_failed.store(false, Ordering::Relaxed);

        let remaining = self.buffered_ms();
        let leftover = (self.last_voiced_end_ms() - drained_ms).max(0);
        if remaining == 0 {
            self.set_last_voiced_end_ms(0);
            self.had_speech.store(false, Ordering::Relaxed);
        } else {
            self.set_last_voiced_end_ms(leftover);
            self.had_speech.store(leftover > 0, Ordering::Relaxed);
        }
        self.turn.fetch_add(1, Ordering::Relaxed);

        // Gate: só gasta um passe final se há fala sustentada de verdade.
        let audio_ms = samples_to_ms(audio.len());
        let gate = audio_ms >= self.timings.min_segment_ms
            && has_sustained_speech(&audio, SPEECH_RMS_THRESHOLD);
        if !gate {
            // Nada será decodificado: o turno não fica pendente.
            if let Ok(mut pending) = self.pending_finals.lock() {
                pending.retain(|pending_turn| *pending_turn != turn);
            }
            tracing::debug!(
                channel = ?self.channel,
                audio_ms,
                cut_ms,
                "turno descartado pelo gate de fala"
            );
            // A prévia já publicada não pode ficar pendurada (ISS-021).
            self.retract_turn(turn, base_ms, cut_ms);
            return;
        }

        // O VAD já fechou o turno na prévia: promove o texto (sem novo passe).
        if let Some(text) = reuse {
            if let Ok(mut pending) = self.pending_finals.lock() {
                pending.retain(|pending_turn| *pending_turn != turn);
            }
            self.set_last_decode(0);
            self.publish_turn(turn, &text, base_ms, cut_ms, true);
            return;
        }

        self.dispatch(DecodeRequest {
            turn,
            base_ms,
            quality: SpeechQuality::Final,
            audio,
            cut_ms,
        });
    }

    /// Aplica o resultado de uma decodificação (publica/texto/finaliza).
    fn handle_outcome(&self, outcome: DecodeOutcome) {
        if let Ok(mut slot) = self.last_attempt_at.lock() {
            *slot = Instant::now();
        }
        let DecodeRequest {
            turn,
            base_ms,
            quality,
            audio,
            cut_ms,
        } = outcome.request;
        log_rtf(self.channel, quality, cut_ms, cut_ms, outcome.wall_ms);

        match quality {
            SpeechQuality::Preview => {
                if turn != self.turn.load(Ordering::Relaxed) {
                    tracing::trace!(turn, "prévia de turno já fechado; descartada");
                    return;
                }
                match outcome.result {
                    Ok(spans) => {
                        // Só avança a cadência com decodificação bem-sucedida.
                        self.set_last_decode(cut_ms);
                        self.last_attempt_failed.store(false, Ordering::Relaxed);
                        let Some(decoded) = self.clean(spans) else {
                            return;
                        };
                        self.publish_turn(turn, &decoded.text, base_ms, cut_ms, false);
                        if let Ok(mut slot) = self.last_preview.lock() {
                            *slot = Some((turn, decoded.text.clone()));
                        }
                        // Perssua (`applyRecognition`): se o VAD do whisper marcou
                        // o fim da fala há ≥ `silence_ms`, o turno terminou — mesmo
                        // com ruído de fundo mantendo o RMS alto.
                        if decoded
                            .speech_end_ms
                            .is_some_and(|end| cut_ms - end >= self.timings.silence_ms)
                        {
                            // O texto desta prévia já cobre o turno: vira final.
                            self.begin_final_reusing(turn, base_ms, audio, cut_ms, decoded.text);
                        }
                    }
                    Err(error) => self.note_failure(quality, error),
                }
            }
            SpeechQuality::Final => {
                let was_pending = self
                    .pending_finals
                    .lock()
                    .map(|mut pending| {
                        let found = pending.contains(&turn);
                        pending.retain(|pending_turn| *pending_turn != turn);
                        found
                    })
                    .unwrap_or(false);
                if !was_pending {
                    tracing::trace!(turn, "passe final superado; descartado");
                    return;
                }
                self.set_last_decode(0);
                match outcome.result {
                    Ok(spans) => {
                        self.last_attempt_failed.store(false, Ordering::Relaxed);
                        match self.clean(spans) {
                            Some(decoded) => {
                                self.publish_turn(turn, &decoded.text, base_ms, cut_ms, true)
                            }
                            // Final sem texto (alucinação/loop/silêncio): a prévia
                            // mostrada tem de sair da conversa (ISS-021).
                            None => self.retract_turn(turn, base_ms, cut_ms),
                        }
                    }
                    Err(error) => {
                        self.note_failure(quality, error);
                        self.persist_preview_fallback(turn, base_ms, cut_ms);
                    }
                }
            }
        }
    }

    /// Registra falha de decodificação sem consumir a cadência de prévias.
    fn note_failure(&self, quality: SpeechQuality, error: GhostWhisperError) {
        self.last_attempt_failed.store(true, Ordering::Relaxed);
        match error {
            // Modelo carregando ou prévia superada: transitório e esperado.
            GhostWhisperError::AsrNotReady | GhostWhisperError::AsrBusy => {
                tracing::trace!(?quality, "ASR indisponível; aguardando próxima janela")
            }
            other => tracing::warn!(?quality, "ASR falhou: {other}"),
        }
    }

    /// Se o passe final falhar, persiste o texto da prévia (não perde o turno).
    fn persist_preview_fallback(&self, turn: u64, base_ms: i64, cut_ms: i64) {
        let text = self.last_preview.lock().ok().and_then(|guard| {
            guard
                .as_ref()
                .filter(|(t, _)| *t == turn)
                .map(|(_, text)| text.clone())
        });
        let Some(text) = text else { return };
        tracing::warn!(turn, "passe final falhou; persistindo a última prévia");
        self.publish_turn(turn, &text, base_ms, cut_ms, true);
    }

    /// Junta os spans e filtra vazio/alucinação.
    fn clean(&self, spans: Vec<TranscribedSpan>) -> Option<DecodedTurn> {
        let joined = spans
            .iter()
            .map(|span| span.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if joined.is_empty() {
            return None;
        }
        // Loop do Whisper (ISS-019): o modelo repete a mesma frase sem parar e a
        // guarda do whisper.cpp (`result_len > 32 && entropy < entropy_thold`) não
        // pega a repetição com entropia acima do limiar. Corta o trecho repetido e
        // fica com a fala real; se tudo era loop, o turno morre aqui.
        let joined = strip_phrase_loop(&joined);
        if joined.is_empty() || is_likely_hallucination(&joined) {
            tracing::debug!("texto descartado (loop/alucinação)");
            return None;
        }
        let speech_end_ms = spans.iter().map(|span| span.end_ms).max();
        Some(DecodedTurn {
            text: joined,
            speech_end_ms,
        })
    }

    /// Id estável do turno: prévia, final e retratação compartilham o mesmo id.
    ///
    /// Carrega o **canal** e a **origem de tempo**: os dois canais da mesma sessão
    /// compartilham `session_id`, a origem e a numeração de turno (cada pipeline
    /// começa em 0), então sem o canal os turnos colidiriam (ISS-018). Continuar uma
    /// sessão cria pipelines novos com `turn` reiniciando em 0, e sem a origem dois
    /// turnos de capturas diferentes também colidiriam.
    fn segment_id(&self, turn: u64) -> String {
        format!(
            "{}-{}-{}-t{turn}",
            self.session_id,
            self.channel.as_str(),
            self.time_origin_ms
        )
    }

    /// Tira da conversa a prévia "em andamento" de um turno que terminou sem texto.
    ///
    /// Quando o passe final não produz texto — filtro de alucinação/loop, gate de
    /// fala sustentada ou buffer insuficiente no `flush` — nada era publicado para o
    /// id do turno e a prévia publicada antes ficava **pendurada** como bolha "em
    /// andamento" com o texto cru do parcial, para sempre (ISS-021). Publicamos um
    /// final de **texto vazio**, que a UI usa para retirar a bolha: o vazio não
    /// vira bolha nem é persistido (quem grava é [`Self::publish_turn`]).
    fn retract_turn(&self, turn: u64, base_ms: i64, cut_ms: i64) {
        let showed_preview = self
            .last_preview
            .lock()
            .map(|guard| {
                guard
                    .as_ref()
                    .is_some_and(|(preview_turn, _)| *preview_turn == turn)
            })
            .unwrap_or(false);
        if !showed_preview {
            // Nada foi mostrado: não poluir a conversa com um item vazio.
            return;
        }
        let start_ms = self.time_origin_ms + base_ms;
        tracing::debug!(turn, channel = ?self.channel, "prévia retratada (turno sem texto)");
        self.publisher.emit_segment(&TranscriptSegment {
            id: self.segment_id(turn),
            session_id: self.session_id.clone(),
            speaker: self.speaker,
            text: String::new(),
            start_ms,
            end_ms: start_ms + cut_ms,
            is_final: true,
        });
    }

    /// Publica o turno (prévia ou final) com id estável por turno.
    ///
    /// A prévia e o final compartilham o mesmo id, então a UI substitui o texto
    /// em vez de duplicar. Só o final é persistido.
    ///
    /// O id carrega o **canal** e a **origem de tempo** da captura: os dois
    /// canais da mesma sessão compartilham `session_id`, a origem e a numeração de
    /// turno (cada pipeline começa em 0), então sem o canal os turnos colidiriam —
    /// a UI substituiria a fala de um pelo texto do outro e o `INSERT` do segundo
    /// final falharia na chave primária (ISS-018). Continuar uma sessão cria
    /// pipelines novos com `turn` reiniciando em 0, e sem a origem dois turnos de
    /// capturas diferentes também colidiriam.
    fn publish_turn(&self, turn: u64, text: &str, base_ms: i64, cut_ms: i64, is_final: bool) {
        let start_ms = self.time_origin_ms + base_ms;
        let segment = TranscriptSegment {
            id: self.segment_id(turn),
            session_id: self.session_id.clone(),
            speaker: self.speaker,
            text: text.to_string(),
            start_ms,
            end_ms: start_ms + cut_ms,
            is_final,
        };
        if is_final {
            if let Err(error) = self.transcriptions.insert(&segment) {
                // Falha aqui significa segmento perdido (histórico/export): o id
                // carrega canal/origem/turno justamente para não haver colisão
                // (ISS-018), então isso é bug — nada de descer para `warn`.
                tracing::error!(
                    segment_id = %segment.id,
                    channel = ?self.channel,
                    "falha ao persistir segmento: {error}"
                );
            }
        }
        self.publisher.emit_segment(&segment);
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

    /// Ancoragem da cadência de prévias (usado em testes de regressão).
    #[cfg(test)]
    pub fn decode_anchor_ms(&self) -> i64 {
        self.last_decode_ms()
    }

    fn buffered_ms(&self) -> i64 {
        self.buffer
            .lock()
            .map(|buffer| samples_to_ms(buffer.len()))
            .unwrap_or(0)
    }

    fn snapshot_upto(&self, cut_ms: i64) -> Vec<f32> {
        match self.buffer.lock() {
            Ok(buffer) => {
                let take = ms_to_samples(cut_ms).min(buffer.len());
                buffer.iter().take(take).copied().collect()
            }
            Err(_) => Vec::new(),
        }
    }

    fn current_turn_context(&self) -> (u64, i64) {
        (
            self.turn.load(Ordering::Relaxed),
            self.turn_base_ms.load(Ordering::Relaxed) as i64,
        )
    }

    fn had_speech(&self) -> bool {
        self.had_speech.load(Ordering::Relaxed)
    }
    fn last_decode_ms(&self) -> i64 {
        self.last_decode_ms.lock().map(|v| *v).unwrap_or(0)
    }
    fn ms_since_last_attempt(&self) -> i64 {
        self.last_attempt_at
            .lock()
            .map(|last| last.elapsed().as_millis() as i64)
            .unwrap_or(i64::MAX)
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
    }
    /// Marca o instante da tentativa (re-arma o relógio de ociosidade).
    fn mark_attempt(&self) {
        if let Ok(mut slot) = self.last_attempt_at.lock() {
            *slot = Instant::now();
        }
    }
    /// Há um passe final pendente para o turno atual?
    ///
    /// Gate do scheduler: enquanto o final do turno atual está na fila, novas
    /// prévias do mesmo turno não fazem sentido.
    fn final_pending_for_current_turn(&self) -> bool {
        let current = self.turn.load(Ordering::Relaxed);
        self.pending_finals
            .lock()
            .map(|pending| pending.contains(&current))
            .unwrap_or(false)
    }

    /// Não há nenhum passe final pendente.
    fn pending_finals_empty(&self) -> bool {
        self.pending_finals
            .lock()
            .map(|pending| pending.is_empty())
            .unwrap_or(true)
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
///
/// Em `info` de propósito: é o número que diz se a transcrição acompanha a fala
/// (RTF < 1 = atrasando). Perssua registra o mesmo RTF
/// (`WhisperSession.js`: "completed in Xms (Yx realtime)").
fn log_rtf(
    channel: AudioChannel,
    quality: SpeechQuality,
    audio_ms: i64,
    buffered_ms: i64,
    wall_ms: u128,
) {
    let label = match quality {
        SpeechQuality::Preview => "preview",
        SpeechQuality::Final => "final",
    };
    let rtf = if wall_ms > 0 {
        audio_ms as f64 / wall_ms as f64
    } else {
        f64::INFINITY
    };
    tracing::info!(
        channel = ?channel,
        pass = label,
        audio_ms,
        buffered_ms,
        wall_ms = wall_ms as u64,
        rtf = format!("{rtf:.2}"),
        "passe de transcrição concluído"
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
    use crate::ports::model_store::ModelStore;

    /// Serializa os testes de integração pesados: cada um carrega o modelo e
    /// disputa os núcleos; dois em paralelo num 4-núcleos falseiam o tempo real
    /// e fazem o streaming parecer lento.
    static HEAVY_E2E: Mutex<()> = Mutex::new(());

    /// Trava dos testes pesados (tolera envenenamento por panic anterior).
    fn heavy_e2e_lock() -> std::sync::MutexGuard<'static, ()> {
        HEAVY_E2E
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

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

    /// ASR que conta chamadas e reporta o fim da fala cedo (VAD nativo do
    /// whisper), exercitando a promoção da prévia a final (BR-029).
    struct CountingEarlyEndAsr {
        calls: AtomicUsize,
    }
    impl CountingEarlyEndAsr {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }
    impl SpeechToText for CountingEarlyEndAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![TranscribedSpan {
                text: "bom dia".into(),
                start_ms: 0,
                end_ms: 500,
            }])
        }
    }

    /// ASR que sempre falha com um erro transitório.
    struct BusyAsr;
    impl SpeechToText for BusyAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            Err(GhostWhisperError::AsrBusy)
        }
    }

    /// ASR lento: simula uma decodificação que leva tempo real.
    struct SlowAsr {
        delay: Duration,
    }
    impl SpeechToText for SlowAsr {
        fn transcribe(
            &self,
            samples: &[f32],
            _quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            std::thread::sleep(self.delay);
            Ok(vec![TranscribedSpan {
                text: "fala".into(),
                start_ms: 0,
                end_ms: samples_to_ms(samples.len()),
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

    /// ASR que devolve textos diferentes para prévia e para o passe final.
    struct QualityAsr {
        preview: String,
        r#final: String,
    }
    impl SpeechToText for QualityAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            quality: SpeechQuality,
        ) -> CoreResult<Vec<TranscribedSpan>> {
            let text = match quality {
                SpeechQuality::Preview => &self.preview,
                SpeechQuality::Final => &self.r#final,
            };
            Ok(vec![TranscribedSpan {
                text: text.clone(),
                start_ms: 0,
                end_ms: 1_000,
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
        fn emit_insight(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &crate::ports::event_publisher::LlmSourceUsage) {}
    }

    type Fixture = (
        TranscriptionPipeline,
        Arc<RecordingPublisher>,
        Arc<InMemoryTranscriptionsRepository>,
    );

    fn pipeline(asr: Arc<dyn SpeechToText>) -> Fixture {
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

    /// Pipeline com a origem de tempo de uma sessão retomada (BR-019).
    fn pipeline_with_origin(
        text: &str,
        origin_ms: i64,
    ) -> (TranscriptionPipeline, Arc<RecordingPublisher>) {
        let (pipeline, publisher, _rows) = pipeline(Arc::new(FakeAsr {
            text: text.to_string(),
        }));
        (pipeline.with_time_origin(origin_ms), publisher)
    }

    fn timings() -> TurnTimings {
        TurnTimings::default()
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
        assert_eq!(segments[0].id, "s1-microphone-0-t0");
    }

    #[test]
    fn time_origin_shifts_timestamps_and_keeps_ids_unique() {
        // Duas capturas da MESMA sessão (BR-019): a segunda continua o relógio de
        // onde a primeira parou, e os ids de turno não colidem entre capturas.
        let (first, first_publisher) = pipeline_with_origin("fala", 0);
        first.consume(&speech(1_500));
        first.tick();
        let first_segments = first_publisher.segments();

        let (second, second_publisher) = pipeline_with_origin("fala", 60_000);
        second.consume(&speech(1_500));
        second.tick();
        let second_segments = second_publisher.segments();

        assert_eq!(first_segments[0].start_ms, 0);
        assert_eq!(second_segments[0].start_ms, 60_000, "linha temporal segue");
        assert_eq!(second_segments[0].end_ms, 61_500);
        assert_ne!(
            first_segments[0].id, second_segments[0].id,
            "ids de capturas diferentes não podem colidir"
        );
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
    fn vad_end_promotes_the_preview_to_final_without_a_second_decode() {
        // BR-029 (Perssua `endedInNoise`): o VAD nativo fecha o turno dentro da
        // prévia ⇒ o texto já decodificado vira final, sem um segundo passe.
        let asr = Arc::new(CountingEarlyEndAsr::new());
        let (pipeline, publisher, rows) = pipeline(asr.clone());

        pipeline.consume(&speech(1_500));
        pipeline.tick();

        let segments = publisher.segments();
        assert_eq!(asr.calls(), 1, "só a prévia foi decodificada");
        assert_eq!(segments.len(), 2, "prévia + final promovido");
        assert!(!segments[0].is_final);
        assert!(segments[1].is_final);
        assert_eq!(segments[0].id, segments[1].id, "mesmo id de turno");
        assert_eq!(segments[1].text, "bom dia");
        assert_eq!(rows.all().len(), 1, "final persistido");
        assert_eq!(pipeline.buffered_samples(), 0, "buffer consumido");
    }

    #[test]
    fn the_two_channels_of_a_session_never_share_a_turn_id() {
        // ISS-018: microfone e sistema compartilham sessão, origem de tempo e
        // numeração de turno (cada pipeline começa em 0). Sem o canal no id, o
        // texto de um substituiria o do outro na UI e o segundo final falharia na
        // chave primária do banco — os dois finais têm de conviver.
        let transcripts = Arc::new(InMemoryTranscriptionsRepository::new());
        let mic_publisher = Arc::new(RecordingPublisher::new());
        let system_publisher = Arc::new(RecordingPublisher::new());

        let mic = TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            Arc::new(FakeAsr {
                text: "eu falo".into(),
            }),
            mic_publisher.clone(),
            transcripts.clone(),
        );
        let system = TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::They,
            AudioChannel::System,
            Arc::new(FakeAsr {
                text: "ele fala".into(),
            }),
            system_publisher.clone(),
            transcripts.clone(),
        );

        // O mesmo turno (primeiro de cada canal) fecha e persiste nos dois.
        for pipeline in [&mic, &system] {
            pipeline.consume(&speech(1_500));
            pipeline.tick();
            pipeline.consume(&silence(1_000));
            pipeline.tick();
        }

        let mic_segments = mic_publisher.segments();
        let system_segments = system_publisher.segments();
        assert_eq!(mic_segments[0].id, "s1-microphone-0-t0");
        assert_eq!(system_segments[0].id, "s1-system-0-t0");

        let stored = transcripts.all();
        assert_eq!(stored.len(), 2, "os dois canais persistem: {stored:?}");
        assert_eq!(
            stored
                .iter()
                .filter(|segment| segment.speaker == Speaker::Me)
                .count(),
            1
        );
        assert_eq!(
            stored
                .iter()
                .filter(|segment| segment.speaker == Speaker::They)
                .count(),
            1
        );
    }

    #[test]
    fn continuous_speech_is_cut_at_max_turn() {
        let (pipeline, publisher) = fake_pipeline("longa");
        let cap = timings().max_turn_ms;
        // Fala contínua no teto: fecha pela teto, cortando exatamente o teto.
        pipeline.consume(&speech(cap));
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
    fn repeated_phrase_loop_is_collapsed_to_one_occurrence() {
        // ISS-019: o modelo repete a mesma frase e a guarda do whisper.cpp não
        // pega; a conversa fica com **uma** ocorrência, nunca com o loop.
        let looped = "O que eu quero fazer? ".repeat(15);
        let (pipeline, publisher) = fake_pipeline(looped.trim());
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        pipeline.consume(&silence(1_000));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 2, "prévia + final");
        for segment in &segments {
            assert_eq!(
                segment.text, "O que eu quero fazer?",
                "loop encolhido para uma ocorrência"
            );
        }
    }

    #[test]
    fn repeated_phrase_loop_is_cut_keeping_the_speech() {
        // Fala real seguida do loop: a fala fica e a frase repetida uma vez.
        let (pipeline, publisher) = fake_pipeline(
            "bom dia pessoal O que eu quero fazer? O que eu quero fazer? O que eu quero fazer?",
        );
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        let preview = publisher.segments();
        assert_eq!(preview.len(), 1);
        assert_eq!(
            preview[0].text, "bom dia pessoal O que eu quero fazer?",
            "loop cortado na prévia"
        );

        pipeline.consume(&silence(1_000));
        pipeline.tick();
        let segments = publisher.segments();
        assert_eq!(segments.len(), 2);
        assert!(segments[1].is_final);
        assert_eq!(
            segments[1].text, "bom dia pessoal O que eu quero fazer?",
            "e no final"
        );
    }

    #[test]
    fn discarded_final_retracts_the_pending_preview() {
        // ISS-021: a prévia aparece "em andamento" com o texto cru; se o passe final
        // for descartado (alucinação), a bolha **não** pode ficar pendurada.
        let (pipeline, publisher, rows) = pipeline(Arc::new(QualityAsr {
            preview: "bom dia pessoal".into(),
            r#final: "Thanks for watching!".into(),
        }));

        pipeline.consume(&speech(1_500));
        pipeline.tick();
        let preview = publisher.segments();
        assert_eq!(preview.len(), 1);
        assert_eq!(preview[0].text, "bom dia pessoal");
        assert!(!preview[0].is_final, "a prévia fica em andamento");

        pipeline.consume(&silence(1_000));
        pipeline.tick();

        let segments = publisher.segments();
        assert_eq!(segments.len(), 2, "prévia + retratação");
        assert!(segments[1].is_final);
        assert_eq!(segments[1].text, "", "texto vazio retira a bolha");
        assert_eq!(
            segments[1].id, segments[0].id,
            "mesmo id: substitui a prévia na conversa"
        );
        assert!(rows.all().is_empty(), "nada é persistido");
    }

    #[test]
    fn discarded_final_without_preview_does_not_publish_anything() {
        // O texto já nasce descartável: nenhuma prévia foi mostrada, então não há o
        // que retratar (nada de item vazio na conversa).
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

    // --- Regressões do streaming em tempo real -------------------------------

    #[test]
    fn finals_of_distinct_turns_are_tracked_independently() {
        // Cada turno tem o seu passe final: um final não pode sumir porque o
        // turno seguinte já começou.
        let (pipeline, _publisher, rows) = pipeline(Arc::new(FakeAsr {
            text: "fala".into(),
        }));
        for _ in 0..2 {
            pipeline.consume(&speech(1_500));
            pipeline.tick();
            pipeline.consume(&silence(1_000));
            pipeline.tick();
        }
        assert_eq!(rows.all().len(), 2, "cada turno persiste o seu final");
    }

    #[test]
    fn failed_preview_does_not_consume_the_preview_step() {
        // A2: uma falha transitória não pode ancorar a cadência — senão o
        // próximo passe só acontece depois de mais 750 ms de áudio.
        let (pipeline, publisher, _rows) = pipeline(Arc::new(BusyAsr));
        pipeline.consume(&speech(1_500));
        pipeline.tick();
        assert!(publisher.segments().is_empty(), "AsrBusy não publica nada");
        assert_eq!(
            pipeline.decode_anchor_ms(),
            0,
            "a cadência não devia avançar numa falha"
        );

        // Com decodificação bem-sucedida, a âncora acompanha o buffer.
        let (ok_pipeline, _publisher) = fake_pipeline("ok");
        ok_pipeline.consume(&speech(1_500));
        ok_pipeline.tick();
        assert_eq!(ok_pipeline.decode_anchor_ms(), 1_500);
    }

    #[test]
    fn tick_never_blocks_on_a_slow_decode() {
        // A1: o agendamento não pode ficar preso atrás da decodificação.
        let publisher = Arc::new(RecordingPublisher::new());
        let pipeline = Arc::new(TranscriptionPipeline::new(
            "s1".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            Arc::new(SlowAsr {
                delay: Duration::from_millis(400),
            }),
            publisher.clone(),
            Arc::new(InMemoryTranscriptionsRepository::new()),
        ));
        pipeline.start();

        pipeline.consume(&speech(1_500));
        let started = Instant::now();
        pipeline.tick();
        assert!(
            started.elapsed() < Duration::from_millis(150),
            "tick devia apenas enfileirar, levou {:?}",
            started.elapsed()
        );

        // O texto ainda chega (na thread de trabalho).
        let deadline = Instant::now() + Duration::from_secs(3);
        while publisher.segments().is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            !publisher.segments().is_empty(),
            "a prévia devia acabar sendo publicada"
        );
        pipeline.stop();
    }

    // --- Integração real (baixa modelos + fala). Rode com `--ignored`. ---

    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn end_to_end_transcribes_jfk_speech() {
        let _heavy = heavy_e2e_lock();
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
            "",
            false,
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

    /// Percorre o caminho de **produção** (threads + worker de decodificação):
    /// alimenta o áudio em blocos e exige que a prévia apareça antes do final.
    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn end_to_end_streams_preview_before_final() {
        let _heavy = heavy_e2e_lock();
        use crate::adapters::speech::whisper_model_store::FileModelStore;
        use crate::adapters::speech::whisper_speech_to_text::WhisperSpeechToText;
        use crate::ports::model_store::ModelStore;

        let samples = download_jfk_wav();
        assert!(samples.len() > 3 * TARGET_SAMPLE_RATE as usize);

        let models_dir = std::env::temp_dir().join("ghostwhisper-models");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(models_dir));
        models.vad_model_path().expect("vad model");
        models.whisper_model_path("tiny").expect("whisper model");

        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&models),
            "tiny",
            "auto",
            "",
            false,
            None,
        ));
        whisper.preload();
        wait_until_asr_ready(&whisper);

        let publisher = Arc::new(RecordingPublisher::new());
        let pipeline = Arc::new(TranscriptionPipeline::new(
            "jfk-stream".to_string(),
            Speaker::Me,
            AudioChannel::Microphone,
            whisper,
            publisher.clone(),
            Arc::new(InMemoryTranscriptionsRepository::new()),
        ))
        .start();

        // Fala completa em blocos de 1 s, como chega da captura.
        let block = TARGET_SAMPLE_RATE as usize;
        for chunk in samples.chunks(block) {
            pipeline.consume(chunk);
            std::thread::sleep(Duration::from_millis(250));
        }

        // A prévia precisa ter sido publicada **durante** a fala.
        let deadline = Instant::now() + Duration::from_secs(20);
        while publisher.segments().iter().all(|s| s.is_final) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        let preview_seen = publisher.segments().iter().any(|s| !s.is_final);
        assert!(preview_seen, "nenhuma prévia publicada durante a fala");

        // `stop` fecha o turno e o final precisa sair.
        pipeline.stop();
        let deadline = Instant::now() + Duration::from_secs(20);
        while publisher.segments().iter().all(|s| !s.is_final) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        let text = publisher
            .segments()
            .iter()
            .filter(|s| s.is_final)
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        assert!(text.contains("country"), "final inesperado: {text:?}");
    }

    /// Curva de RTF: parede por segundo de áudio em fatias de duração crescente.
    ///
    /// É o dado que decide se a prévia pode decodificar o turno inteiro (como o
    /// Perssua) ou precisa de janela deslizante. Rode com
    /// `cargo test --release --lib -- --ignored --nocapture benchmark_rtf`.
    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn benchmark_rtf_by_audio_duration() {
        let _heavy = heavy_e2e_lock();
        use crate::adapters::speech::whisper_model_store::FileModelStore;
        use crate::adapters::speech::whisper_speech_to_text::WhisperSpeechToText;
        use crate::ports::model_store::ModelStore;

        let jfk = download_jfk_wav();
        assert!(jfk.len() > 16_000, "amostra de fala muito curta");

        let models_dir = std::env::temp_dir().join("ghostwhisper-models");
        let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(models_dir));
        models.vad_model_path().expect("vad model");
        let model = std::env::var("GHOSTWHISPER_BENCH_MODEL").unwrap_or_else(|_| "tiny".into());
        models.whisper_model_path(&model).expect("whisper model");

        let whisper = Arc::new(WhisperSpeechToText::new(
            Arc::clone(&models),
            &model,
            "auto",
            "",
            false,
            None,
        ));
        whisper.preload();
        wait_until_asr_ready(&whisper);

        // Aquecimento: a primeira chamada paga alocação de buffers do ggml.
        let _ = whisper.transcribe(&jfk[..TARGET_SAMPLE_RATE as usize], SpeechQuality::Preview);

        println!("\n--- RTF por duracao (modelo {model}) ---");
        println!(
            "{:>7}  {:>8}  {:>8}  {:>8}  {:>8}  texto",
            "audio_s", "prev_ms", "prev_rtf", "fin_ms", "fin_rtf"
        );
        for seconds in [1usize, 2, 4, 8, 11, 20, 30] {
            let samples = tile(&jfk, seconds * TARGET_SAMPLE_RATE as usize);
            let audio_ms = samples_to_ms(samples.len()) as f64;

            // Mínimo de N repetições: sob ruído de máquina compartilhada o menor
            // tempo é o estimador mais próximo do custo real da decodificação.
            let repeats = if seconds <= 8 { 3 } else { 1 };
            let mut preview_ms = f64::MAX;
            let mut final_ms = f64::MAX;
            let mut text = String::new();
            for _ in 0..repeats {
                let started = Instant::now();
                let _ = whisper
                    .transcribe(&samples, SpeechQuality::Preview)
                    .expect("transcrever preview");
                preview_ms = preview_ms.min(started.elapsed().as_millis().max(1) as f64);

                let started = Instant::now();
                let final_pass = whisper
                    .transcribe(&samples, SpeechQuality::Final)
                    .expect("transcrever final");
                final_ms = final_ms.min(started.elapsed().as_millis().max(1) as f64);
                text = final_pass
                    .iter()
                    .map(|span| span.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ");
            }

            println!(
                "{:>7.1}  {:>8.0}  {:>8.2}  {:>8.0}  {:>8.2}  {}",
                audio_ms / 1000.0,
                preview_ms,
                audio_ms / preview_ms,
                final_ms,
                audio_ms / final_ms,
                text.chars().take(40).collect::<String>()
            );
        }
    }

    /// Repete o áudio até atingir `total` amostras (mantém a duração pedida).
    fn tile(samples: &[f32], total: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(total);
        while out.len() < total {
            let remaining = total - out.len();
            let take = remaining.min(samples.len());
            out.extend_from_slice(&samples[..take]);
        }
        out
    }

    /// Store que delega o whisper mas nega o VAD (isola o custo do Silero).
    struct NoVadStore {
        inner: crate::adapters::speech::whisper_model_store::FileModelStore,
    }
    impl ModelStore for NoVadStore {
        fn vad_model_path(&self) -> CoreResult<std::path::PathBuf> {
            Err(GhostWhisperError::Other(
                "VAD desabilitado (diagnóstico)".into(),
            ))
        }
        fn whisper_model_path(&self, name: &str) -> CoreResult<std::path::PathBuf> {
            self.inner.whisper_model_path(name)
        }
    }

    /// Diagnóstico: separa o custo fixo por chamada (VAD × encode) do custo
    /// proporcional ao áudio. Rode com `--ignored --nocapture`.
    #[test]
    #[ignore = "baixa o modelo whisper tiny e a fala de exemplo (requer rede)"]
    fn benchmark_vad_overhead() {
        let _heavy = heavy_e2e_lock();
        use crate::adapters::speech::whisper_model_store::FileModelStore;
        use crate::adapters::speech::whisper_speech_to_text::WhisperSpeechToText;

        let jfk = download_jfk_wav();
        let one_second = &jfk[..TARGET_SAMPLE_RATE as usize];

        println!(
            "\navailable_parallelism = {:?}",
            std::thread::available_parallelism().map(|n| n.get())
        );

        for with_vad in [true, false] {
            let models_dir = std::env::temp_dir().join("ghostwhisper-models");
            let inner = FileModelStore::new(models_dir);
            let models: Arc<dyn ModelStore> = if with_vad {
                Arc::new(inner)
            } else {
                Arc::new(NoVadStore { inner })
            };
            let whisper = Arc::new(WhisperSpeechToText::new(
                Arc::clone(&models),
                "tiny",
                "auto",
                "",
                false,
                None,
            ));
            whisper.preload();
            wait_until_asr_ready(&whisper);

            // Aquecimento (compila/aloja buffers) para medir o regime.
            let _ = whisper.transcribe(one_second, SpeechQuality::Preview);

            let started = Instant::now();
            let spans = whisper
                .transcribe(one_second, SpeechQuality::Preview)
                .expect("decodificar 1s");
            let wall = started.elapsed().as_millis();
            println!("vad={with_vad} 1s -> {wall}ms ({} spans)", spans.len());
        }
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
