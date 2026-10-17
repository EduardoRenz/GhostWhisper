//! Conversão de PCM para o formato de trabalho (f32 mono 16 kHz).
//!
//! Reamostragem linear é suficiente para fala no MVP e evita dependências
//! nativas. Anti-aliasing/resampling de maior qualidade é melhoria futura.

use crate::domain::audio::TARGET_SAMPLE_RATE;

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

/// Reamostra linearmente de `from_rate` para `to_rate`.
pub fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return Vec::new();
    }
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let idx = src.floor() as usize;
        let frac = (src - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Downmix + reamostragem para o formato de trabalho (mono 16 kHz).
pub fn to_target_mono(interleaved: &[f32], channels: u16, from_rate: u32) -> Vec<f32> {
    let mono = downmix_to_mono(interleaved, channels);
    resample_linear(&mono, from_rate, TARGET_SAMPLE_RATE)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // O primeiro valor é preservado.
        assert_eq!(out[0], 0.0);
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
}
