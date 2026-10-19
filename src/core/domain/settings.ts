/** Tipos de configuração do GhostWhisper (espelham `domain::settings` no core). */

import { defaultBindings, type ShortcutBinding } from "./shortcuts";
import type { AssistantMode } from "./assistant";

export type ProviderKind = "openai" | "gemini" | "openrouter" | "custom";

/** Idiomas suportados em origem e destino (`auto` = detectar / não traduzir). */
export type Language = "auto" | "pt" | "en";

/** Aceleração do ASR (espelha `domain::settings::ACCELERATION_MODES`). */
export type AccelerationMode = "auto" | "cpu" | "gpu";

/** Opções de aceleração, na ordem de exibição. */
export const ACCELERATION_MODES: AccelerationMode[] = ["auto", "cpu", "gpu"];

/** Rótulos de aceleração. */
export const ACCELERATION_LABELS: Record<AccelerationMode, string> = {
  auto: "Auto (GPU se disponível)",
  cpu: "CPU (sem GPU)",
  gpu: "GPU (Vulkan/CUDA)",
};

/** Opções de idioma, na ordem de exibição. */
export const LANGUAGES: Language[] = ["auto", "pt", "en"];

/** Rótulos para origem (Auto = detectar). */
export const SOURCE_LANGUAGE_LABELS: Record<Language, string> = {
  auto: "Auto (detectar)",
  pt: "Português",
  en: "English",
};

/** Rótulos para destino (Auto = não traduzir). */
export const TARGET_LANGUAGE_LABELS: Record<Language, string> = {
  auto: "Auto (não traduzir)",
  pt: "Português",
  en: "English",
};

export interface Settings {
  provider: ProviderKind;
  model: string;
  /** Endpoint custom (formato OpenAI); obrigatório quando `provider = custom`. */
  base_url: string | null;
  source_language: Language;
  target_language: Language;
  translation_enabled: boolean;
  whisper_model: string;
  /**
   * Prompt de decodificação (`initial_prompt`) do whisper; vazio desliga.
   *
   * Reduz alucinação e impede a tradução; a regra de idioma é acrescentada pelo
   * core (BR-027).
   */
  whisper_prompt: string;
  /** Aceleração do ASR: `auto` (GPU se disponível), `cpu` ou `gpu`. */
  acceleration: AccelerationMode;
  /** Cancelamento de eco (AEC) no microfone, usando o áudio do sistema. */
  aec_enabled: boolean;
  /** Ganho do microfone (1×–12×); não afeta o áudio do sistema. */
  mic_gain: number;
  /** Modo stealth (sem barra de tarefas / sem título). */
  stealth: boolean;
  /** Atalhos globais. */
  shortcuts: ShortcutBinding[];
  /** Posição do overlay na tela (lembrada após o usuário mover). */
  overlay_position: [number, number] | null;
  /** Agente/modo selecionado para sugestões (`meeting` por padrão). */
  assistant_mode_id: string;
  /** Agentes personalizados criados pelo usuário. */
  custom_assistant_modes: AssistantMode[];
  /** Atualização automática do resumo enquanto a janela está aberta. */
  summary_auto_refresh: boolean;
  /** Intervalo do resumo automático (segundos). */
  summary_interval_secs: number;
  /** Atualização automática da janela de sugestões. */
  suggestion_auto_refresh: boolean;
  /** Intervalo das sugestões automáticas (segundos). */
  suggestion_interval_secs: number;
  /** Posição da janela de resumo. */
  summary_position: [number, number] | null;
  /** Posição da janela de sugestões. */
  suggestions_position: [number, number] | null;
  /** Tamanho (px físicos) da janela de resumo. */
  summary_size: [number, number] | null;
  /** Tamanho (px físicos) da janela de sugestões. */
  suggestions_size: [number, number] | null;
  /** Posição da janela de chat. */
  chat_position: [number, number] | null;
  /** Posição da janela de histórico. */
  history_position: [number, number] | null;
  /** Tamanho (px físicos) da janela de chat. */
  chat_size: [number, number] | null;
  /** Tamanho (px físicos) da janela de histórico. */
  history_size: [number, number] | null;
}

/** Providers na ordem de exibição. */
export const PROVIDERS: ProviderKind[] = [
  "openai",
  "gemini",
  "openrouter",
  "custom",
];

/** Modelos whisper.cpp locais suportados (baixados sob demanda). */
export const WHISPER_MODELS = [
  "tiny",
  "base",
  "small",
  "medium",
  "large-v3-turbo",
  "large-v3",
] as const;

/**
 * Modelos que exigem GPU para tempo real (BR-027).
 *
 * Em CPU eles são lentos demais para o streaming; a UI avisa.
 */
export const WHISPER_GPU_MODELS: string[] = ["large-v3-turbo", "large-v3"];

/** Tamanhos aproximados dos modelos whisper (download sob demanda). */
export const WHISPER_MODEL_SIZES: Record<string, string> = {
  tiny: "~75 MB",
  base: "~140 MB",
  small: "~470 MB",
  medium: "~1.5 GB",
  "large-v3-turbo": "~1.6 GB",
  "large-v3": "~3.1 GB",
};

/** Prompt de decodificação padrão (Perssua: `whisper_prompt`). */
export const DEFAULT_WHISPER_PROMPT =
  "Only transcribe what is clearly spoken. If uncertain, transcribe nothing.";

/**
 * Ganho de microfone (1×–12×). O padrão e o teto são os do Perssua
 * (`local_whisper_user_mic_gain`, padrão 8, faixa 1–12) — microfones de notebook
 * costumam entregar fala fraca demais para o gate de fala abrir (BR-018).
 */
export const DEFAULT_MIC_GAIN = 8;
export const MIN_MIC_GAIN = 1;
export const MAX_MIC_GAIN = 12;

/** Configurações padrão (espelham `Settings::default`). */
export const defaultSettings = (): Settings => ({
  provider: "openai",
  model: "gpt-4o-mini",
  base_url: null,
  source_language: "auto",
  target_language: "pt",
  translation_enabled: false,
  whisper_model: "tiny",
  whisper_prompt: DEFAULT_WHISPER_PROMPT,
  acceleration: "auto",
  aec_enabled: true,
  mic_gain: DEFAULT_MIC_GAIN,
  stealth: false,
  shortcuts: defaultBindings(),
  overlay_position: null,
  assistant_mode_id: "meeting",
  custom_assistant_modes: [],
  summary_auto_refresh: true,
  summary_interval_secs: 30,
  suggestion_auto_refresh: true,
  suggestion_interval_secs: 20,
  summary_position: null,
  suggestions_position: null,
  summary_size: null,
  suggestions_size: null,
  chat_position: null,
  history_position: null,
  chat_size: null,
  history_size: null,
});

/** Intervalos aceitos pelas atualizações automáticas (segundos). */
export const MIN_AUTO_INTERVAL_SECS = 10;
export const MAX_AUTO_INTERVAL_SECS = 120;

/** Opções de cadência do resumo automático. */
export const SUMMARY_INTERVALS = [15, 30, 60, 120] as const;
/** Opções de cadência das sugestões automáticas. */
export const SUGGESTION_INTERVALS = [10, 20, 30, 60] as const;

/** `base_url` é obrigatória apenas para endpoints custom. */
export const providerRequiresBaseUrl = (provider: ProviderKind): boolean =>
  provider === "custom";
