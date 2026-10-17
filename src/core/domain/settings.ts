/** Tipos de configuração do GhostWhisper (espelham `domain::settings` no core). */

import { defaultBindings, type ShortcutBinding } from "./shortcuts";

export type ProviderKind = "openai" | "gemini" | "openrouter" | "custom";

/** Idiomas suportados em origem e destino (`auto` = detectar / não traduzir). */
export type Language = "auto" | "pt" | "en";

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
  /** Modo stealth (sem barra de tarefas / sem título). */
  stealth: boolean;
  /** Atalhos globais. */
  shortcuts: ShortcutBinding[];
  /** Posição do overlay na tela (lembrada após o usuário mover). */
  overlay_position: [number, number] | null;
}

/** Providers na ordem de exibição. */
export const PROVIDERS: ProviderKind[] = [
  "openai",
  "gemini",
  "openrouter",
  "custom",
];

/** Modelos whisper.cpp locais suportados (baixados sob demanda). */
export const WHISPER_MODELS = ["tiny", "base", "small", "medium"] as const;

/** Tamanhos aproximados dos modelos whisper (download sob demanda). */
export const WHISPER_MODEL_SIZES: Record<string, string> = {
  tiny: "~75 MB",
  base: "~140 MB",
  small: "~470 MB",
  medium: "~1.5 GB",
};

/** Configurações padrão (espelham `Settings::default`). */
export const defaultSettings = (): Settings => ({
  provider: "openai",
  model: "gpt-4o-mini",
  base_url: null,
  source_language: "auto",
  target_language: "pt",
  translation_enabled: false,
  whisper_model: "tiny",
  stealth: false,
  shortcuts: defaultBindings(),
  overlay_position: null,
});

/** `base_url` é obrigatória apenas para endpoints custom. */
export const providerRequiresBaseUrl = (provider: ProviderKind): boolean =>
  provider === "custom";
