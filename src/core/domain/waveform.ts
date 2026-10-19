/**
 * Escala do waveform do notch (BR-032/task 08).
 *
 * O `rms` vem do `audio_level` (0..1, já pós-ganho do microfone). A fala fica
 * tipicamente em 0,02–0,3; usamos a raiz para dar sensação perceptual e um ganho
 * fixo para as barras reagirem a fala baixa.
 */

/** Normaliza o RMS em um nível 0..1 para as barras. */
export function waveformLevel(rms: number): number {
  if (!Number.isFinite(rms) || rms <= 0) {
    return 0;
  }
  return Math.min(1, Math.max(0, Math.sqrt(rms) * 1.6));
}

/** Altura (px) de uma barra para um nível 0..1 (mín. 4, máx. 14). */
export function waveformHeight(level: number): number {
  const clamped = Math.min(1, Math.max(0, level));
  return 4 + clamped * 10;
}

/** Fatores por barra para o movimento não ficar "sincronizado". */
export const WAVEFORM_FACTORS = [1, 0.72, 0.9, 0.62] as const;

/** Quantidade de barras do waveform do notch. */
export const WAVEFORM_BARS = WAVEFORM_FACTORS.length;
