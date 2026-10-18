//! Conversão de PCM para o formato de trabalho (f32 mono 16 kHz).
//!
//! A decimação (ex.: 48 kHz → 16 kHz) aplica um **passa-baixa** antes de reduzir
//! a taxa: sem isso, tudo acima da nova Nyquist "espelha" para dentro da banda de
//! fala (aliasing) e degrada o ASR (ISS-025). O filtro é um FIR *windowed-sinc*
//! curto, com estado entre blocos para não cortar o início/fim de cada bloco de
//! captura.
//!
//! O áudio do sistema **não** passa por aqui: o PulseAudio já entrega mono 16 kHz.

use std::f64::consts::PI;

#[cfg(test)]
use crate::domain::audio::TARGET_SAMPLE_RATE;

/// Número de coeficientes do FIR de decimação (ímpar = fase linear exata).
const FIR_TAPS: usize = 63;

/// Converte amostras intercaladas (N canais) em mono por média dos canais.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let channels = channels as usize;
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Coeficientes de um passa-baixa com corte logo abaixo da Nyquist de destino.
fn low_pass_taps(from_rate: u32, to_rate: u32) -> Vec<f32> {
    let cutoff = 0.5 * to_rate as f64 / from_rate as f64;
    // Um pouco abaixo da Nyquist: a banda de transição do FIR fica fora da fala.
    let fc = (cutoff * 0.94).clamp(1e-6, 0.5);
    let middle = (FIR_TAPS - 1) as f64 / 2.0;
    let mut taps = Vec::with_capacity(FIR_TAPS);
    let mut sum = 0.0f64;
    for index in 0..FIR_TAPS {
        let offset = index as f64 - middle;
        let sinc = if offset.abs() < 1e-9 {
            2.0 * fc
        } else {
            (2.0 * PI * fc * offset).sin() / (PI * offset)
        };
        let window = 0.54 - 0.46 * ((2.0 * PI * index as f64) / (FIR_TAPS as f64 - 1.0)).cos();
        let value = sinc * window;
        taps.push(value);
        sum += value;
    }
    // Ganho unitário em DC (não altera o nível da fala).
    if sum.abs() > 1e-9 {
        for tap in taps.iter_mut() {
            *tap /= sum;
        }
    }
    taps.into_iter().map(|tap| tap as f32).collect()
}

/// Reamostrador com **estado**: use um por stream de captura.
///
/// Mantém o histórico do FIR e a fase fracionária entre blocos, para o resultado
/// ser o mesmo de um stream contínuo (sem estalos nas emendas nem deriva de
/// comprimento).
pub struct Resampler {
    from_rate: u32,
    to_rate: u32,
    channels: u16,
    /// Coeficientes do FIR; vazio quando não há decimação (sem filtro).
    taps: Vec<f32>,
    /// Últimas `taps.len() / 2` amostras do bloco anterior (fase linear).
    history: Vec<f32>,
    /// Posição fracionária (em amostras da origem) do próximo destino.
    position: f64,
}

impl Resampler {
    /// Cria o reamostrador para um stream de `from_rate` para `to_rate`.
    pub fn new(channels: u16, from_rate: u32, to_rate: u32) -> Self {
        let taps = if to_rate > 0 && to_rate < from_rate {
            low_pass_taps(from_rate, to_rate)
        } else {
            Vec::new()
        };
        let history = vec![0.0f32; taps.len() / 2];
        Self {
            from_rate,
            to_rate,
            channels: channels.max(1),
            taps,
            history,
            position: 0.0,
        }
    }

    /// Converte um bloco intercalado em mono na taxa de destino.
    pub fn process(&mut self, interleaved: &[f32]) -> Vec<f32> {
        if interleaved.is_empty() || self.from_rate == 0 || self.to_rate == 0 {
            return Vec::new();
        }
        let mono = downmix_to_mono(interleaved, self.channels);
        if mono.is_empty() {
            return Vec::new();
        }
        let filtered = self.filter(&mono);
        if self.from_rate == self.to_rate {
            return filtered;
        }
        self.resample(&filtered)
    }

    /// Aplica o FIR com o histórico do bloco anterior (ou devolve sem filtrar).
    ///
    /// O filtro é de **fase linear** e centrado: a saída `i` corresponde à amostra
    /// `mono[i]` (sem deslocamento entre canais/timestamps).
    fn filter(&mut self, mono: &[f32]) -> Vec<f32> {
        if self.taps.is_empty() {
            return mono.to_vec();
        }
        let half = self.taps.len() / 2;
        // Sinal estendido: `half` amostras passadas + bloco + `half` futuras (zeros).
        let mut extended = Vec::with_capacity(half + mono.len() + half);
        extended.extend_from_slice(&self.history);
        extended.extend_from_slice(mono);
        extended.extend(std::iter::repeat(0.0).take(half));

        let mut out = Vec::with_capacity(mono.len());
        for index in 0..mono.len() {
            let mut acc = 0.0f32;
            // Convolução direta (FIR curto, blocos pequenos): clareza > micro-otimização.
            for (tap_index, &tap) in self.taps.iter().enumerate() {
                acc += tap * extended[index + tap_index];
            }
            out.push(acc);
        }

        // Histórico = últimas `half` amostras já vistas (histórico + bloco),
        // completando com zeros à esquerda quando o stream é curto.
        let mut combined = Vec::with_capacity(self.history.len() + mono.len());
        combined.extend_from_slice(&self.history);
        combined.extend_from_slice(mono);
        let mut next_history: Vec<f32> = Vec::with_capacity(half);
        if combined.len() < half {
            next_history.extend(std::iter::repeat(0.0).take(half - combined.len()));
        }
        let start = combined.len().saturating_sub(half);
        next_history.extend_from_slice(&combined[start..]);
        self.history = next_history;
        out
    }

    /// Interpolação linear com fase carregada entre blocos.
    fn resample(&mut self, mono: &[f32]) -> Vec<f32> {
        let step = self.from_rate as f64 / self.to_rate as f64;
        let mut out = Vec::with_capacity((mono.len() as f64 / step).ceil() as usize + 1);
        let mut source = self.position;
        while source < mono.len() as f64 {
            let clamped = source.max(0.0);
            let index = clamped.floor() as usize;
            let frac = (clamped - index as f64) as f32;
            let a = mono.get(index).copied().unwrap_or(0.0);
            let b = mono.get(index + 1).copied().unwrap_or(a);
            out.push(a + (b - a) * frac);
            source += step;
        }
        self.position = source - mono.len() as f64;
        out
    }
}

/// Reamostra linearmente de `from_rate` para `to_rate` (um bloco, sem estado).
///
/// Conveniência para testes; a captura de microfone usa [`Resampler`] para manter
/// o estado entre blocos.
#[cfg(test)]
pub fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return Vec::new();
    }
    if from_rate == to_rate {
        return input.to_vec();
    }
    Resampler::new(1, from_rate, to_rate).process(input)
}

/// Downmix + reamostragem para o formato de trabalho (mono 16 kHz), um bloco.
#[cfg(test)]
pub fn to_target_mono(interleaved: &[f32], channels: u16, from_rate: u32) -> Vec<f32> {
    Resampler::new(channels, from_rate, TARGET_SAMPLE_RATE).process(interleaved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn tone(frequency: f64, rate: u32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| (2.0 * PI * frequency * i as f64 / rate as f64).sin() as f32 * 0.5)
            .collect()
    }

    #[test]
    fn downmix_stereo_averages_channels() {
        let input = [1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
        assert_eq!(downmix_to_mono(&input, 2), vec![0.5, 0.5, 0.5]);
    }

    #[test]
    fn downmix_mono_is_identity() {
        let input = [0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&input, 1), input.to_vec());
    }

    #[test]
    fn resample_downscales_48000_to_16000() {
        let input: Vec<f32> = (0..4800).map(|i| i as f32).collect();
        let out = resample_linear(&input, 48_000, 16_000);
        assert_eq!(out.len(), 1600);
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let input = [0.1, 0.2, 0.3, 0.4];
        assert_eq!(resample_linear(&input, 16_000, 16_000), input.to_vec());
    }

    #[test]
    fn resample_empty_is_empty() {
        assert!(resample_linear(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn to_target_mono_stereo_48k_len() {
        // 4800 frames estéreo (9600 amostras) a 48k → 1600 amostras a 16k.
        let input = vec![0.0f32; 9600];
        assert_eq!(to_target_mono(&input, 2, 48_000).len(), 1600);
    }

    #[test]
    fn decimation_low_passes_above_target_nyquist() {
        // 10 kHz a 48 kHz espelharia para 6 kHz com decimação "crua" (Nyquist 8 kHz).
        let high = tone(10_000.0, 48_000, 48_000);
        let low = tone(1_000.0, 48_000, 48_000);
        let filtered_high = rms(&resample_linear(&high, 48_000, 16_000));
        let filtered_low = rms(&resample_linear(&low, 48_000, 16_000));
        assert!(
            filtered_high < filtered_low * 0.2,
            "tom acima da Nyquist devia ser atenuado: alto={filtered_high} baixo={filtered_low}"
        );
    }

    #[test]
    fn decimation_keeps_the_speech_band_level() {
        let speech = tone(1_000.0, 48_000, 48_000);
        let out = resample_linear(&speech, 48_000, 16_000);
        let input_rms = rms(&speech[4_800..]);
        let output_rms = rms(&out[1_600..]);
        assert!(
            (output_rms - input_rms).abs() < 0.05 * input_rms.max(1e-6),
            "nível da fala mudou: {input_rms} -> {output_rms}"
        );
    }

    #[test]
    fn streaming_blocks_match_a_single_shot() {
        let input = tone(1_000.0, 48_000, 48_000);
        let one_shot = resample_linear(&input, 48_000, 16_000);

        let mut streamed = Vec::new();
        let mut resampler = Resampler::new(1, 48_000, 16_000);
        for chunk in input.chunks(1_024) {
            streamed.extend(resampler.process(chunk));
        }

        // Comprimento equivalente (±1 amostra pela emenda dos blocos).
        let diff = one_shot.len().abs_diff(streamed.len());
        assert!(
            diff <= 1,
            "comprimentos divergiram: {} vs {}",
            one_shot.len(),
            streamed.len()
        );
    }

    #[test]
    fn the_filter_does_not_shift_the_signal() {
        // Filtro de fase linear centrado: um impulso em `i` continua em `i`.
        let mut input = vec![0.0f32; 400];
        input[100] = 1.0;
        let out = Resampler::new(1, 16_000, 16_000).process(&input);
        let peak = out
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).expect("ordenável"))
            .map(|(index, _)| index)
            .expect("peak");
        assert_eq!(peak, 100, "o filtro deslocou o sinal");
    }

    #[test]
    fn constant_signal_survives_the_filter() {
        let input = vec![0.5f32; 9_600];
        let out = to_target_mono(&input, 1, 48_000);
        let tail = &out[100..];
        assert!((rms(tail) - 0.5).abs() < 0.01, "DC mudou: {}", rms(tail));
    }
}
