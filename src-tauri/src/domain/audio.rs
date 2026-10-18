//! Tipos de domínio para áudio.

use serde::Serialize;

use std::collections::VecDeque;

/// Taxa de amostragem de trabalho (esperada pelo whisper.cpp).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Nível de áudio de um bloco (RMS e pico).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
}

impl AudioLevel {
    /// Calcula o nível de um bloco de amostras normalizadas.
    pub fn from_samples(samples: &[f32]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sum_squares = 0.0f32;
        let mut peak = 0.0f32;
        for &sample in samples {
            sum_squares += sample * sample;
            let abs = sample.abs();
            if abs > peak {
                peak = abs;
            }
        }
        Some(Self {
            rms: (sum_squares / samples.len() as f32).sqrt(),
            peak,
        })
    }
}

/// Duração mínima de fala (RMS acima de `threshold`) para considerar o áudio como
/// fala contínua, em vez de clique/ruído.
pub const MIN_SPEECH_MS: i64 = 160;

/// Ganho de microfone padrão (Perssua: `local_whisper_user_mic_gain = 8`).
///
/// Microfones de notebook costumam entregar fala bem abaixo de 1.0 de amplitude;
/// sem este estágio o gate de fala quase nunca abre (BR-018).
pub const DEFAULT_MIC_GAIN: f32 = 8.0;
/// Menor ganho permitido (1× = sem amplificação).
pub const MIN_MIC_GAIN: f32 = 1.0;
/// Maior ganho permitido (o Perssua limita a 12×).
pub const MAX_MIC_GAIN: f32 = 12.0;

/// Limiar RMS que separa fala de ruído (sinal de trabalho, pós-ganho).
///
/// É a única fonte do valor: usado pelo gate de entrada do microfone e pelo
/// pipeline (disparo de prévias e passe final), para que os dois nunca se
/// contradigam.
pub const SPEECH_RMS_THRESHOLD: f32 = 0.008;

/// Silêncio mantido depois da última fala antes de silenciar (Perssua:
/// `local_whisper_user_mic_noise_gate_hangover_ms = 600`).
pub const DEFAULT_NOISE_GATE_HANGOVER_MS: u64 = 600;

/// Aplica ganho ao sinal, saturando em `[-1, 1]`.
///
/// É o estágio de entrada do microfone (Perssua: `micGainNode` + `micGainBuffer`,
/// aplicado **antes** do gate de fala). A saturação evita amostras fora de faixa
/// no whisper quando o ganho é alto; `gain = 1` é identidade (e não satura nada).
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    if !gain.is_finite() || gain <= 0.0 || (gain - 1.0).abs() < f32::EPSILON {
        return;
    }
    for sample in samples.iter_mut() {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

/// Normaliza um ganho vindo da configuração para a faixa suportada.
pub fn clamp_mic_gain(gain: f32) -> f32 {
    if !gain.is_finite() {
        return DEFAULT_MIC_GAIN;
    }
    gain.clamp(MIN_MIC_GAIN, MAX_MIC_GAIN)
}

/// Não há ruído abaixo deste RMS para silenciar (desliga o gate).
pub const NOISE_GATE_DISABLED: f32 = 0.0;

/// Noise gate do microfone: **silencia** (em vez de descartar) o que está abaixo
/// do limiar, com *hangover*.
///
/// Transposto do Perssua (`Kv` em `main.f180d025.js`, opção `muteInsteadOfDrop`):
/// - `rms >= threshold` ⇒ passa e marca fala;
/// - ainda em fala até `hangover_ms` desde a última fala ⇒ passa (pausas curtas
///   dentro da mesma frase não são cortadas);
/// - caso contrário ⇒ zera o bloco.
///
/// **Diferença deliberada:** o Perssua, depois de um tempo mudo, *descarta* os
/// blocos. Aqui nunca descartamos — o pipeline mede o silêncio pelo comprimento
/// do buffer, então remover amostras impediria o turno de fechar. Zerar preserva
/// a linha do tempo e tem o mesmo efeito para o ASR.
///
/// **Limiar padrão:** alinhado ao gate de fala do app
/// ([`SPEECH_RMS_THRESHOLD`]), não aos 0,025 do Perssua — aquele valor é mais
/// restritivo que o nosso gate e silenciaria fala que hoje é transcrita
/// (justamente a sensibilidade que queremos preservar). Ajustável por ambiente.
#[derive(Debug, Clone)]
pub struct NoiseGate {
    threshold: f32,
    hangover_ms: u64,
    speaking: bool,
    last_loud_ms: u64,
}

impl NoiseGate {
    /// Cria o gate; `threshold <= 0` o desliga.
    pub fn new(threshold: f32, hangover_ms: u64) -> Self {
        Self {
            threshold: if threshold.is_finite() {
                threshold.max(NOISE_GATE_DISABLED)
            } else {
                SPEECH_RMS_THRESHOLD
            },
            hangover_ms,
            speaking: false,
            last_loud_ms: 0,
        }
    }

    /// Limiar configurado.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Processa um bloco **in place**: `true` quando passou, `false` quando foi
    /// silenciado. `now_ms` vem de um relógio monotônico do chamador.
    pub fn process(&mut self, samples: &mut [f32], now_ms: u64) -> bool {
        if self.threshold <= NOISE_GATE_DISABLED {
            return true;
        }
        let loud =
            AudioLevel::from_samples(samples).is_some_and(|level| level.rms >= self.threshold);
        if loud {
            self.speaking = true;
            self.last_loud_ms = now_ms;
            return true;
        }
        if self.speaking && now_ms.saturating_sub(self.last_loud_ms) <= self.hangover_ms {
            return true;
        }
        self.speaking = false;
        samples.fill(0.0);
        false
    }
}

/// Janela de análise do gate de fala, em ms.
const GATE_FRAME_MS: i64 = 20;

/// Indica se o áudio contém fala **sustentada** (RMS > `threshold` por pelo menos
/// `MIN_SPEECH_MS` contínuos), rejeitando ruído e cliques antes do ASR.
pub fn has_sustained_speech(samples: &[f32], threshold: f32) -> bool {
    let frame = ((TARGET_SAMPLE_RATE as i64 * GATE_FRAME_MS / 1000) as usize).max(1);
    let mut voiced_ms = 0i64;
    for chunk in samples.chunks(frame) {
        let level = AudioLevel::from_samples(chunk);
        let voiced = level.is_some_and(|l| l.rms > threshold);
        if voiced {
            voiced_ms += (chunk.len() * 1000 / TARGET_SAMPLE_RATE as usize) as i64;
            if voiced_ms >= MIN_SPEECH_MS {
                return true;
            }
        } else {
            voiced_ms = 0;
        }
    }
    false
}

/// Um dispositivo de entrada de áudio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    /// Identificador estável usado para seleção (nome do dispositivo).
    pub id: String,
    /// Nome legível.
    pub name: String,
    /// Se é o dispositivo padrão do sistema.
    pub is_default: bool,
}

// --- Monitor atrasado do microfone ("ouvir minha voz", task 15) -------------

/// Atraso padrão do monitor, em ms.
///
/// O atraso é o que evita a realimentação (microfonia) quando a voz sai pela
/// caixa de som: sem ele, o microfone reouve a própria saída e entra em loop.
pub const DEFAULT_MONITOR_DELAY_MS: u64 = 400;

/// Folga máxima além do atraso, em ms de áudio (protege contra atraso crescente).
const MONITOR_MAX_LAG_MS: u64 = 500;

/// Bloco de áudio agendado para reprodução.
#[derive(Debug)]
struct ReadyBlock {
    /// Quando o bloco pode ser tocado (`ms` no relógio do chamador).
    ready_at_ms: u64,
    samples: Vec<f32>,
}

/// Fila de blocos de áudio com **atraso fixo** para o monitor.
///
/// Pura de propósito: recebe o `now_ms` do chamador, então o atraso e a
/// proteção contra acúmulo são testáveis sem placa de som nem threads.
#[derive(Debug)]
pub struct DelayedAudioQueue {
    delay_ms: u64,
    blocks: VecDeque<ReadyBlock>,
    pending_samples: usize,
    dropped_samples: u64,
}

impl DelayedAudioQueue {
    /// Cria a fila com o atraso dado (ms).
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            blocks: VecDeque::new(),
            pending_samples: 0,
            dropped_samples: 0,
        }
    }

    /// Atraso configurado (ms).
    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }

    /// Amostras aguardando reprodução.
    pub fn pending_samples(&self) -> usize {
        self.pending_samples
    }

    /// Amostras descartadas por acúmulo (o consumidor não acompanhou).
    pub fn dropped_samples(&self) -> u64 {
        self.dropped_samples
    }

    /// Enfileira um bloco; ele fica pronto em `now_ms + delay_ms`.
    ///
    /// Se a fila passar de `atraso + folga`, os blocos **mais antigos** são
    /// descartados: melhor perder áudio velho do que deixar a voz cada vez mais
    /// atrasada.
    pub fn push(&mut self, samples: &[f32], now_ms: u64) {
        if samples.is_empty() {
            return;
        }
        let limit = ms_to_samples(self.delay_ms + MONITOR_MAX_LAG_MS);
        while self.pending_samples + samples.len() > limit {
            let Some(dropped) = self.blocks.pop_front() else {
                break;
            };
            self.pending_samples = self.pending_samples.saturating_sub(dropped.samples.len());
            self.dropped_samples += dropped.samples.len() as u64;
        }
        self.pending_samples += samples.len();
        self.blocks.push_back(ReadyBlock {
            ready_at_ms: now_ms + self.delay_ms,
            samples: samples.to_vec(),
        });
    }

    /// Anexa em `out` as amostras prontas (`ready_at_ms <= now_ms`).
    pub fn drain_ready(&mut self, now_ms: u64, out: &mut Vec<f32>) {
        while let Some(block) = self.blocks.front() {
            if block.ready_at_ms > now_ms {
                break;
            }
            let Some(block) = self.blocks.pop_front() else {
                break;
            };
            self.pending_samples = self.pending_samples.saturating_sub(block.samples.len());
            out.extend_from_slice(&block.samples);
        }
    }

    /// Descarta o pendente (ao desligar o monitor).
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.pending_samples = 0;
    }
}

/// Converte milissegundos em amostras na taxa de trabalho.
fn ms_to_samples(ms: u64) -> usize {
    (ms as usize * TARGET_SAMPLE_RATE as usize) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_from_samples() {
        assert_eq!(AudioLevel::from_samples(&[]), None);
        let level = AudioLevel::from_samples(&[0.5, -0.5]).expect("level");
        assert!((level.rms - 0.5).abs() < 1e-6);
        assert!((level.peak - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sustained_speech_rejects_short_blip() {
        // 40 ms de tom forte: menos que MIN_SPEECH_MS ⇒ não é fala.
        let blip = vec![0.5f32; TARGET_SAMPLE_RATE as usize * 40 / 1000];
        assert!(!has_sustained_speech(&blip, 0.01));
    }

    #[test]
    fn sustained_speech_accepts_real_tone() {
        // 500 ms de tom forte acima do limiar ⇒ fala sustentada.
        let tone = vec![0.5f32; TARGET_SAMPLE_RATE as usize / 2];
        assert!(has_sustained_speech(&tone, 0.01));
    }

    #[test]
    fn sustained_speech_rejects_silence() {
        let silence = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        assert!(!has_sustained_speech(&silence, 0.01));
    }

    #[test]
    fn gain_amplifies_and_saturates() {
        let mut samples = [0.05f32, -0.05, 0.5];
        apply_gain(&mut samples, 8.0);
        assert!((samples[0] - 0.4).abs() < 1e-6);
        assert!((samples[1] + 0.4).abs() < 1e-6);
        // 0,5 × 8 = 4 → satura em 1,0 (nunca sai da faixa do whisper).
        assert_eq!(samples[2], 1.0);
    }

    #[test]
    fn gain_one_is_identity_and_never_saturates() {
        let mut samples = [0.05f32, -1.5, 2.0];
        apply_gain(&mut samples, 1.0);
        // Sem mudança — inclusive para amostras já fora de faixa.
        assert_eq!(samples, [0.05, -1.5, 2.0]);
    }

    #[test]
    fn gain_ignores_invalid_values() {
        let mut samples = [0.1f32];
        apply_gain(&mut samples, f32::NAN);
        apply_gain(&mut samples, 0.0);
        apply_gain(&mut samples, -3.0);
        assert_eq!(samples, [0.1]);
    }

    #[test]
    fn mic_gain_is_clamped_or_falls_back_to_default() {
        assert_eq!(clamp_mic_gain(0.2), MIN_MIC_GAIN);
        assert_eq!(clamp_mic_gain(99.0), MAX_MIC_GAIN);
        assert_eq!(clamp_mic_gain(4.0), 4.0);
        assert_eq!(clamp_mic_gain(f32::NAN), DEFAULT_MIC_GAIN);
    }

    #[test]
    fn gain_lifts_a_quiet_mic_above_the_speech_threshold() {
        // Fala fraca (típica de microfone de notebook) não passaria no gate;
        // depois do ganho padrão, passa.
        let quiet = vec![0.004f32; TARGET_SAMPLE_RATE as usize / 2];
        assert!(!has_sustained_speech(&quiet, SPEECH_RMS_THRESHOLD));

        let mut boosted = quiet.clone();
        apply_gain(&mut boosted, DEFAULT_MIC_GAIN);
        assert!(has_sustained_speech(&boosted, SPEECH_RMS_THRESHOLD));
    }

    // --- Noise gate (Perssua: `Kv`, mute + hangover) -------------------------

    fn loud(ms: i64) -> Vec<f32> {
        vec![0.4f32; TARGET_SAMPLE_RATE as usize * ms as usize / 1000]
    }
    fn quiet(ms: i64) -> Vec<f32> {
        vec![0.0005f32; TARGET_SAMPLE_RATE as usize * ms as usize / 1000]
    }

    #[test]
    fn gate_passes_speech_and_mutes_noise() {
        let mut gate = NoiseGate::new(SPEECH_RMS_THRESHOLD, DEFAULT_NOISE_GATE_HANGOVER_MS);

        let mut speech = loud(100);
        assert!(gate.process(&mut speech, 0));
        assert!(speech.iter().any(|s| *s != 0.0), "fala passa intacta");

        // 1 s depois do hangover, ruído é silenciado (não descartado).
        let mut noise = quiet(100);
        assert!(!gate.process(&mut noise, 2_000));
        assert!(noise.iter().all(|s| *s == 0.0), "ruído vira silêncio");
    }

    #[test]
    fn gate_keeps_short_pauses_inside_a_sentence() {
        let mut gate = NoiseGate::new(SPEECH_RMS_THRESHOLD, DEFAULT_NOISE_GATE_HANGOVER_MS);
        let mut speech = loud(100);
        assert!(gate.process(&mut speech, 0));

        // 300 ms de pausa (< hangover de 600 ms) ainda passa.
        let mut pause = quiet(100);
        assert!(gate.process(&mut pause, 300));
        assert!(pause.iter().any(|s| *s != 0.0), "pausa curta não é cortada");

        // Depois do hangover, silencia.
        let mut late = quiet(100);
        assert!(!gate.process(&mut late, 900));
        assert!(late.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn gate_mutes_before_any_speech() {
        let mut gate = NoiseGate::new(SPEECH_RMS_THRESHOLD, DEFAULT_NOISE_GATE_HANGOVER_MS);
        let mut noise = quiet(100);
        assert!(!gate.process(&mut noise, 10_000));
        assert!(noise.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn gate_can_be_disabled_with_a_non_positive_threshold() {
        let mut gate = NoiseGate::new(NOISE_GATE_DISABLED, DEFAULT_NOISE_GATE_HANGOVER_MS);
        let mut noise = quiet(100);
        assert!(gate.process(&mut noise, 0));
        assert!(noise.iter().all(|s| *s != 0.0), "sem limiar, nada é mexido");
    }

    #[test]
    fn gate_falls_back_to_the_default_threshold_on_garbage() {
        let gate = NoiseGate::new(f32::NAN, DEFAULT_NOISE_GATE_HANGOVER_MS);
        assert_eq!(gate.threshold(), SPEECH_RMS_THRESHOLD);
    }

    // --- Monitor atrasado (task 15) -----------------------------------------

    /// 100 ms de áudio, com o valor dado (para identificar o bloco).
    fn block(value: f32) -> Vec<f32> {
        vec![value; TARGET_SAMPLE_RATE as usize / 10]
    }

    #[test]
    fn delayed_queue_holds_the_block_until_the_delay_elapses() {
        let mut queue = DelayedAudioQueue::new(400);
        queue.push(&block(0.1), 1_000);

        // Antes do atraso: nada pronto.
        let mut out = Vec::new();
        queue.drain_ready(1_000, &mut out);
        assert!(out.is_empty());
        queue.drain_ready(1_399, &mut out);
        assert!(out.is_empty(), "ainda dentro do atraso");

        // No atraso exato o bloco fica pronto.
        queue.drain_ready(1_400, &mut out);
        assert_eq!(out.len(), TARGET_SAMPLE_RATE as usize / 10);
        assert!(out
            .iter()
            .all(|sample| (*sample - 0.1).abs() < f32::EPSILON));
        assert_eq!(queue.pending_samples(), 0);
    }

    #[test]
    fn delayed_queue_preserves_the_order_of_blocks() {
        let mut queue = DelayedAudioQueue::new(100);
        queue.push(&block(1.0), 0);
        queue.push(&block(2.0), 10);

        let mut out = Vec::new();
        queue.drain_ready(100, &mut out);
        assert!(out
            .iter()
            .all(|sample| (*sample - 1.0).abs() < f32::EPSILON));

        out.clear();
        queue.drain_ready(110, &mut out);
        assert!(out
            .iter()
            .all(|sample| (*sample - 2.0).abs() < f32::EPSILON));
    }

    #[test]
    fn delayed_queue_drops_the_oldest_blocks_when_lagging_behind() {
        // Atraso de 100 ms + folga de 500 ms = 600 ms de teto.
        let mut queue = DelayedAudioQueue::new(100);
        for index in 0..10 {
            // 10 blocos de 100 ms = 1 s de áudio (acima do teto).
            queue.push(&block(index as f32), 0);
        }

        assert!(
            queue.pending_samples() <= ms_to_samples(600),
            "a fila não pode crescer sem limite"
        );
        assert!(
            queue.dropped_samples() > 0,
            "o excedente antigo é descartado"
        );

        let mut out = Vec::new();
        queue.drain_ready(1_000, &mut out);
        // O que sobrou é o mais recente (o bloco 9 é o último empilhado).
        assert!(
            out.last()
                .is_some_and(|sample| (*sample - 9.0).abs() < f32::EPSILON),
            "áudio recente preservado"
        );
    }

    #[test]
    fn delayed_queue_ignores_empty_blocks_and_clears_on_demand() {
        let mut queue = DelayedAudioQueue::new(50);
        queue.push(&[], 0);
        assert_eq!(queue.pending_samples(), 0);

        queue.push(&block(0.5), 0);
        assert!(queue.pending_samples() > 0);
        queue.clear();
        assert_eq!(queue.pending_samples(), 0);

        let mut out = Vec::new();
        queue.drain_ready(1_000, &mut out);
        assert!(out.is_empty(), "descartado não toca");
    }

    #[test]
    fn delayed_queue_reports_its_delay() {
        assert_eq!(DelayedAudioQueue::new(0).delay_ms(), 0);
        assert_eq!(
            DelayedAudioQueue::new(DEFAULT_MONITOR_DELAY_MS).delay_ms(),
            400
        );
    }
}
