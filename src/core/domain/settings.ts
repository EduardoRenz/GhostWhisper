/** Tipos de configuração do GhostWhisper (espelham `domain::settings` no core). */

import { defaultBindings, type ShortcutBinding } from "./shortcuts";
import type {
  AssistantMode,
  ClassifierSensitivity,
  SuggestionCue,
  SuggestionMode,
} from "./assistant";
import type { UiLanguage } from "../l10n";

/** Provedores de LLM suportados (BYO key). */
export type ProviderKind =
  | "openai"
  | "gemini"
  | "openrouter"
  | "deepseek"
  | "custom";

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

/**
 * Fonte de IA cadastrada (BR-044).
 *
 * A **ordem** da lista é a ordem de prioridade: a primeira é a primária.
 */
export interface AiSource {
  /** Id estável (uuid). A chave no keyring é `llm.source.{id}`. */
  id: string;
  /** Provedor. */
  kind: ProviderKind;
  /** Nome de exibição ("OpenAI · Empresa"). */
  label: string;
  /** Modelo usado nas chamadas. */
  model: string;
  /** Endpoint custom (formato OpenAI); só usado por `custom`. */
  base_url: string | null;
  /** A fonte participa do roteamento? */
  enabled: boolean;
}

/** Estado de saúde de uma fonte (BR-045). */
export type AiSourceState = "unknown" | "ready" | "degraded" | "failed";

/** Informe de saúde de uma fonte. */
export interface AiSourceHealth {
  source_id: string;
  state: AiSourceState;
  consecutive_failures: number;
  latency_ms: number | null;
  last_error: string | null;
  is_fallback: boolean;
  open_until_ms: number | null;
}

/** Capacidade que pode ter uma fonte própria (BR-048). */
export type AiCapability = "summary" | "suggestions" | "translation";

/** Escopo de uma rota de IA (BR-048). */
export type AiRouteScope =
  | { kind: "default" }
  | { kind: "capability"; capability: AiCapability }
  | { kind: "agent"; modeId: string };

/** Vínculo entre um escopo e uma fonte de IA. */
export interface AiRoute {
  scope: AiRouteScope;
  source_id: string;
}

export interface Settings {
  /** Fontes de IA, ordenadas (a primeira é a primária) — BR-044. */
  ai_sources: AiSource[];
  /** Vínculos de fonte por escopo — BR-048. */
  ai_routes: AiRoute[];
  /**
   * Idioma da interface (`pt` | `en`).
   *
   * `null` só antes da primeira inicialização: o core detecta o padrão do
   * sistema uma única vez e persiste. Depois disso, a escolha do usuário manda.
   */
  ui_language: UiLanguage | null;
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
  /** Posição da barra "Perguntar" (só muda por arraste; BR-072). */
  ask_position: [number, number] | null;
  /** Agente/modo selecionado para sugestões (`default` — o agente Padrão). */
  assistant_mode_id: string;
  /**
   * Marca a migração do agente padrão (BR-059).
   *
   * O agente **Padrão** (contextual) entrou no lugar da "Reunião" como
   * padrão; a marca garante que a migração acontece **uma vez** (depois dela,
   * a escolha do usuário é respeitada).
   */
  assistant_mode_migrated: boolean;
  /** Agentes personalizados criados pelo usuário. */
  custom_assistant_modes: AssistantMode[];
  /**
   * Agentes de fábrica já materializados na lista do usuário (BR-075).
   *
   * O default do core nasce `false`; o boot semeia a lista e liga a marca.
   */
  assistant_modes_seeded: boolean;
  /**
   * Revisão do catálogo de agentes de fábrica (BR-083).
   *
   * `0` = settings anteriores; o boot aplica as mudanças uma vez.
   */
  assistant_catalog_revision: number;
  /** Atualização automática do resumo enquanto a janela está aberta. */
  summary_auto_refresh: boolean;
  /** Intervalo do resumo automático (segundos). */
  summary_interval_secs: number;
  /** Atualização automática da janela de sugestões. */
  suggestion_auto_refresh: boolean;
  /** Intervalo das sugestões automáticas (segundos). */
  suggestion_interval_secs: number;
  /** Estratégia de disparo: automático ou classificador. */
  suggestion_mode: SuggestionMode;
  /** Escopo do classificador: blocos ME+THEY ou adaptativo. */
  suggestion_scope: "blocks" | "adaptive";
  /** Mostra a faixa de última classificação. */
  suggestion_show_classification: boolean;
  /** Backend do classificador. */
  suggestion_classifier_backend: "heuristic" | "laya";
  /** Sensibilidade (deriva o limiar). */
  suggestion_classifier_sensitivity: ClassifierSensitivity;
  /** Modelo do model manager. */
  suggestion_classifier_model_id: string;
  /** Versão ativa (vazio = a ativa no cache). */
  suggestion_classifier_model_version: string;
  /** Override de dev para o diretório ONNX. */
  suggestion_classifier_model_path: string;
  /** Catálogo de modelos. */
  suggestion_classifier_model_catalog_url: string;
  /** Nunca baixar sem ação explícita. */
  suggestion_classifier_auto_download: boolean;
  /** Checkpoint Laya (só multilingual; English não é exposto). */
  suggestion_classifier_checkpoint: "multilingual";
  /** Cooldown mínimo do classificador. */
  suggestion_classifier_cooldown_secs: number;
  /** Timeout do classificador. */
  suggestion_classifier_timeout_ms: number;
  /** Em erro: não sugerir ou cair no automático. */
  suggestion_classifier_fallback: "skip" | "auto";
  /** Silêncio que fecha um bloco. */
  suggestion_block_gap_ms: number;
  /** Duração máxima do bloco. */
  suggestion_block_max_ms: number;
  /** Turnos máximos do bloco. */
  suggestion_block_max_turns: number;
  /** Blocos anteriores no contexto. */
  suggestion_context_blocks: number;
  /** Cues permitidas (vazio = todas as positivas). */
  suggestion_allowed_cues: SuggestionCue[];
  /** Sonda de bloco aberto. */
  suggestion_probe_open_block: boolean;
  /** Fala contínua que habilita a sonda. */
  suggestion_hesitation_probe_ms: number;
  /** Posição da janela de sugestões. */
  suggestions_position: [number, number] | null;
  /** Tamanho (px físicos) da janela de sugestões. */
  suggestions_size: [number, number] | null;
  /** Posição da janela de chat (painel da sessão). */
  chat_position: [number, number] | null;
  /** Tamanho (px físicos) da janela de chat (painel da sessão). */
  chat_size: [number, number] | null;
  /** Posição da janela de histórico. */
  history_position: [number, number] | null;
  /** Tamanho (px físicos) da janela de histórico. */
  history_size: [number, number] | null;
  /** Aba ativa do painel da sessão (BR-033). */
  panel_tab: PanelTab;
  /** Drill-hover (ativação por permanência) habilitado (BR-036). */
  dwell_enabled: boolean;
  /** Tempo de permanência do drill-hover, em milissegundos (BR-036). */
  dwell_ms: number;
  /** Opacidade das superfícies de todas as janelas (0,2–1,0) — BR-062. */
  overlay_opacity: number;
  /** Ancoragem do notch na área útil do monitor primário — BR-062. */
  notch_anchor: NotchAnchor;
  /** Densidade (espaçamento/espessura) do notch — BR-062. */
  hud_density: HudDensity;
  /**
   * Escala relativa da fonte de toda a UI, 0,75–1,6 — BR-087.
   *
   * Só vale nas **janelas de conteúdo**: notch e barra "Perguntar" mantêm o
   * tamanho próprio (densidade do HUD).
   */
  font_scale: number;
  /** Bônus de tamanho do texto de IA (1,0–1,5) — BR-087. */
  ai_font_scale: number;
  saved_questions: SavedQuestion[];
  /**
   * Marca a migração das perguntas prontas (BR-054): o preset **novo**
   * entra uma vez em quem já tinha a lista persistida.
   */
  saved_questions_migrated: boolean;
  /** O que gravar: vídeo + áudio ou só áudio (BR-066). */
  recording_mode: RecordingMode;
  /** Alvo padrão: janela da reunião ou tela inteira (BR-070). */
  recording_target: RecordingTargetKind;
  /** Inicia a gravação junto com a sessão. */
  recording_auto_start: boolean;
  /** Segue a janela ativa, trocando o alvo quando o foco muda. */
  recording_follow_active: boolean;
  /** FPS da gravação (degraus de 5 a 30). */
  recording_fps: number;
  /** Resolução da captura. */
  recording_resolution: RecordingResolution;
  /** Qualidade (CRF 30/27/24). */
  recording_quality: RecordingQuality;
  /** Encoder: automático, x264 ou VAAPI. */
  recording_encoder: RecordingEncoder;
  /** Bitrate do AAC (kbps). */
  recording_audio_kbps: number;
  /** Container final. */
  recording_container: RecordingContainer;
  /** Pasta de destino (vazio = padrão do sistema). */
  recording_dir: string | null;
  /** Mantém os segmentos MKV após a finalização. */
  recording_keep_segments: boolean;
}

/** Abas do painel da sessão (janela `chat`). */
export type PanelTab = "transcription" | "summary" | "session";

/** Uma pergunta pronta do usuário (BR-054). */
export interface SavedQuestion {
  /** Id estável (para editar/remover sem depender da posição). */
  id: string;
  /** O texto da pergunta. */
  text: string;
}

/** Teto de perguntas prontas guardadas (BR-054). */
export const MAX_SAVED_QUESTIONS = 20;
/** Teto de caracteres de uma pergunta pronta (BR-054). */
export const MAX_SAVED_QUESTION_CHARS = 200;

/** Perguntas prontas semeadas (tradução das "Prompt Suggestions" do Perssua). */
export const DEFAULT_SAVED_QUESTIONS: SavedQuestion[] = [
  { id: "default-0", text: "Resumir tudo até agora" },
  { id: "default-1", text: "Listar as ações" },
  { id: "default-2", text: "Listar as principais ideias" },
  { id: "default-3", text: "O que posso falar?" },
  // Um pedido: a IA responde à última pergunta identificada (BR-054).
  { id: "default-4", text: "Responda a última pergunta identificada" },
];

/** Gravação da sessão (BR-066/BR-070) ------------------------------------- */

export type RecordingMode = "video" | "audio";
export type RecordingTargetKind = "window" | "screen";
export type RecordingResolution = "native" | "1280x720" | "854x480";
export type RecordingQuality = "compact" | "balanced" | "sharp";
export type RecordingEncoder = "auto" | "x264" | "vaapi";
export type RecordingContainer = "mp4" | "mkv";

/** FPS padrão: 8 (BR-066 — o custo é por frame, não por resolução). */
export const DEFAULT_RECORDING_FPS = 8;
export const RECORDING_FPS_STEPS = [5, 8, 10, 15, 30];

/** Bitrate de áudio padrão (kbps). */
export const DEFAULT_RECORDING_AUDIO_KBPS = 96;
export const RECORDING_AUDIO_KBPS_STEPS = [64, 96, 128, 160];

export const RECORDING_MODES: RecordingMode[] = ["video", "audio"];
export const RECORDING_MODE_LABELS: Record<RecordingMode, string> = {
  video: "Vídeo + áudio",
  audio: "Somente áudio",
};

export const RECORDING_TARGETS: RecordingTargetKind[] = ["window", "screen"];
export const RECORDING_TARGET_LABELS: Record<RecordingTargetKind, string> = {
  window: "Janela da reunião",
  screen: "Tela inteira",
};

export const RECORDING_RESOLUTIONS: RecordingResolution[] = [
  "native",
  "1280x720",
  "854x480",
];
export const RECORDING_RESOLUTION_LABELS: Record<RecordingResolution, string> = {
  native: "Nativa",
  "1280x720": "1280×720",
  "854x480": "854×480",
};

export const RECORDING_QUALITIES: RecordingQuality[] = [
  "compact",
  "balanced",
  "sharp",
];
export const RECORDING_QUALITY_LABELS: Record<RecordingQuality, string> = {
  compact: "Compacta",
  balanced: "Equilibrada",
  sharp: "Nítida",
};

export const RECORDING_ENCODERS: RecordingEncoder[] = ["auto", "x264", "vaapi"];
export const RECORDING_ENCODER_LABELS: Record<RecordingEncoder, string> = {
  auto: "Automático",
  x264: "x264 (software)",
  vaapi: "VAAPI (hardware)",
};

export const RECORDING_CONTAINERS: RecordingContainer[] = ["mp4", "mkv"];
export const RECORDING_CONTAINER_LABELS: Record<RecordingContainer, string> = {
  mp4: "MP4",
  mkv: "MKV",
};

/** Providers na ordem de exibição. */
export const PROVIDERS: ProviderKind[] = [
  "openai",
  "gemini",
  "openrouter",
  "deepseek",
  "custom",
];

/** Rótulo de exibição de um provedor. */
export const PROVIDER_LABELS: Record<ProviderKind, string> = {
  openai: "OpenAI",
  gemini: "Google Gemini",
  openrouter: "OpenRouter",
  deepseek: "DeepSeek",
  custom: "Endpoint OpenAI-compatível",
};

/** Rótulo curto de um provedor (nos cards). */
export const PROVIDER_SHORT_LABELS: Record<ProviderKind, string> = {
  openai: "OpenAI",
  gemini: "Gemini",
  openrouter: "OpenRouter",
  deepseek: "DeepSeek",
  custom: "Custom",
};

/** `base_url` padrão por provedor (quando existe). */
export const PROVIDER_DEFAULT_BASE_URLS: Partial<Record<ProviderKind, string>> = {
  openai: "https://api.openai.com/v1",
  openrouter: "https://openrouter.ai/api/v1",
  deepseek: "https://api.deepseek.com/v1",
};

/**
 * **Não há catálogo de modelos.** O usuário digita o nome do modelo
 * (`gpt-6`, `deepseek-chat`, …): qualquer lista sugerida fica desatualizada
 * sooner or later e trava o usuário num modelo que existe mas não está na
 * lista.
 */
export const MODEL_PLACEHOLDER = "nome-do-modelo";

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

/** Contexto do `initial_prompt` (vazio por padrão — ver ISS-059). */
export const DEFAULT_WHISPER_PROMPT = "";

/**
 * Ganho de microfone (1×–12×). O padrão e o teto são os do Perssua
 * (`local_whisper_user_mic_gain`, padrão 8, faixa 1–12) — microfones de notebook
 * costumam entregar fala fraca demais para o gate de fala abrir (BR-018).
 */
export const DEFAULT_MIC_GAIN = 8;
export const MIN_MIC_GAIN = 1;
export const MAX_MIC_GAIN = 12;

/** Id reservado da fonte padrão (instalação nova / migração). */
export const DEFAULT_SOURCE_ID = "default";

/** Fonte padrão: OpenAI com `gpt-4o-mini` (sem chave). */
export const defaultSource = (): AiSource => ({
  id: DEFAULT_SOURCE_ID,
  kind: "openai",
  label: "OpenAI",
  model: "gpt-4o-mini",
  base_url: null,
  enabled: true,
});

/** Configurações padrão (espelham `Settings::default`). */
export const defaultSettings = (): Settings => ({
  ai_sources: [defaultSource()],
  ai_routes: [],
  ui_language: null,
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
  ask_position: null,
  assistant_mode_id: "default",
  assistant_mode_migrated: false,
  custom_assistant_modes: [],
  assistant_modes_seeded: false,
  assistant_catalog_revision: 0,
  summary_auto_refresh: true,
  summary_interval_secs: 30,
  suggestion_auto_refresh: true,
  suggestion_interval_secs: 15,
  suggestion_mode: "auto",
  suggestion_scope: "adaptive",
  suggestion_show_classification: true,
  suggestion_classifier_backend: "heuristic",
  suggestion_classifier_sensitivity: "balanced",
  suggestion_classifier_model_id: "laya-multilingual",
  suggestion_classifier_model_version: "",
  suggestion_classifier_model_path: "",
  suggestion_classifier_model_catalog_url:
    "https://downloads.ghostwhisper.app/laya/catalog.json",
  suggestion_classifier_auto_download: false,
  suggestion_classifier_checkpoint: "multilingual",
  suggestion_classifier_cooldown_secs: 15,
  suggestion_classifier_timeout_ms: 3_000,
  suggestion_classifier_fallback: "skip",
  suggestion_block_gap_ms: 3_000,
  suggestion_block_max_ms: 45_000,
  suggestion_block_max_turns: 12,
  suggestion_context_blocks: 3,
  suggestion_allowed_cues: [],
  suggestion_probe_open_block: true,
  suggestion_hesitation_probe_ms: 4_000,
  suggestions_position: null,
  suggestions_size: null,
  chat_position: null,
  chat_size: null,
  history_position: null,
  history_size: null,
  panel_tab: "transcription",
  dwell_enabled: true,
  dwell_ms: 500,
  overlay_opacity: DEFAULT_OVERLAY_OPACITY,
  notch_anchor: "top_center",
  hud_density: "standard",
  font_scale: DEFAULT_FONT_SCALE,
  ai_font_scale: DEFAULT_AI_FONT_SCALE,
  saved_questions: DEFAULT_SAVED_QUESTIONS,
  saved_questions_migrated: false,
  recording_mode: "video",
  recording_target: "window",
  recording_auto_start: false,
  recording_follow_active: false,
  recording_fps: DEFAULT_RECORDING_FPS,
  recording_resolution: "native",
  recording_quality: "balanced",
  recording_encoder: "auto",
  recording_audio_kbps: DEFAULT_RECORDING_AUDIO_KBPS,
  recording_container: "mp4",
  recording_dir: null,
  recording_keep_segments: false,
});

/** Intervalos aceitos pelas atualizações automáticas (segundos). */
export const MIN_AUTO_INTERVAL_SECS = 10;
export const MAX_AUTO_INTERVAL_SECS = 120;
/** Piso do ciclo de sugestões (o resumo continua em 10 s). */
export const MIN_SUGGESTION_INTERVAL_SECS = 8;
/** Presets do card “Sugestão por tempo”. */
export const SUGGESTION_INTERVALS = [8, 15, 30] as const;

/** Drill-hover: padrão e faixa em milissegundos (BR-036, faixa da BR-062). */
export const DEFAULT_DWELL_MS = 500;
export const MIN_DWELL_MS = 100;
export const MAX_DWELL_MS = 1_500;
/** Passo da UI de duração (0,1 s). */
export const DWELL_STEP_MS = 100;

/** HUD (BR-062) ----------------------------------------------------------- */

/** Ponto de fixação do notch na área útil do monitor primário. */
export type NotchAnchor = "top_center" | "top_right" | "free";

/** Ancoragens, na ordem de exibição. */
export const NOTCH_ANCHORS: NotchAnchor[] = ["top_center", "top_right", "free"];

/** Rótulos das ancoragens. */
export const NOTCH_ANCHOR_LABELS: Record<NotchAnchor, string> = {
  top_center: "Topo Central",
  top_right: "Canto Sup. Dir.",
  free: "Flutuante Livre",
};

/** Espaçamento interno/espessura da barra do notch. */
export type HudDensity = "compact" | "standard" | "expanded";

/** Densidades, na ordem de exibição. */
export const HUD_DENSITIES: HudDensity[] = ["compact", "standard", "expanded"];

/** Rótulos das densidades. */
export const HUD_DENSITY_LABELS: Record<HudDensity, string> = {
  compact: "Compacto",
  standard: "Padrão",
  expanded: "Expandido",
};

/** Opacidade das superfícies de todas as janelas (20 %–100 %, padrão 75 %). */
export const DEFAULT_OVERLAY_OPACITY = 0.75;
export const MIN_OVERLAY_OPACITY = 0.2;
export const MAX_OVERLAY_OPACITY = 1;

/** Legendas do slider de opacidade (mockup). */
export const OPACITY_LEGEND = [
  { value: MIN_OVERLAY_OPACITY, label: "Discreto (20%)" },
  { value: DEFAULT_OVERLAY_OPACITY, label: "Equilibrado (75%)" },
  { value: MAX_OVERLAY_OPACITY, label: "Opaco (100%)" },
] as const;

/** Traz a opacidade para a faixa aceita (passo livre). */
export function clampOverlayOpacity(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_OVERLAY_OPACITY;
  }
  return Math.min(MAX_OVERLAY_OPACITY, Math.max(MIN_OVERLAY_OPACITY, value));
}

/** Escala de fonte da UI (BR-087) ----------------------------------------- */

/** Escala padrão (100 %). */
export const DEFAULT_FONT_SCALE = 1;
/** Escala mínima aceita (75 %). */
export const MIN_FONT_SCALE = 0.75;
/** Escala máxima aceita (160 %). */
export const MAX_FONT_SCALE = 1.6;
/** Bônus de texto de IA padrão (100 %). */
export const DEFAULT_AI_FONT_SCALE = 1;
/** Bônus mínimo (100 %): ele só acrescenta à escala global. */
export const MIN_AI_FONT_SCALE = 1;
/** Bônus máximo de texto de IA (150 %). */
export const MAX_AI_FONT_SCALE = 1.5;

/** Traz a escala global para a faixa aceita (passo livre). */
export function clampFontScale(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_FONT_SCALE;
  }
  return Math.min(MAX_FONT_SCALE, Math.max(MIN_FONT_SCALE, value));
}

/** Traz o bônus de texto de IA para a faixa aceita (passo livre). */
export function clampAiFontScale(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_AI_FONT_SCALE;
  }
  return Math.min(MAX_AI_FONT_SCALE, Math.max(MIN_AI_FONT_SCALE, value));
}

/** Opções de cadência do resumo automático. */
export const SUMMARY_INTERVALS = [15, 30, 60, 120] as const;

/** `base_url` é obrigatória apenas para endpoints custom. */
export const providerRequiresBaseUrl = (provider: ProviderKind): boolean =>
  provider === "custom";

/** Gera um id de fonte (uuid v4) com fallback para ambientes sem crypto. */
export const newSourceId = (): string => {
  const globalCrypto = globalThis.crypto as Crypto | undefined;
  if (globalCrypto && typeof globalCrypto.randomUUID === "function") {
    return globalCrypto.randomUUID();
  }
  return `fonte-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
};
