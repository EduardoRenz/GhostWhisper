//! Configurações do usuário (não sensíveis) e catálogo de providers.

use serde::{Deserialize, Serialize};

use crate::domain::ai_route::{validate_routes, AiRoute};
use crate::domain::ai_source::{validate_sources, AiSource};
use crate::domain::assistant::{is_builtin_id, validate_custom_modes, AssistantMode};
use crate::domain::shortcuts::ShortcutBinding;
use crate::error::{GhostWhisperError, Result};

/// Idiomas suportados em origem e destino.
///
/// `auto` na origem = detecção; `auto` no destino = **não traduzir**.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["auto", "pt", "en"];

/// Indica se o código de idioma é suportado.
pub fn is_supported_language(code: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&code.trim())
}

/// Nome legível do idioma, usado nos prompts do LLM.
pub fn language_name(code: &str) -> &'static str {
    match code.trim() {
        "pt" => "português",
        "en" => "inglês",
        _ => "o mesmo idioma da transcrição",
    }
}

/// Prompt base de decodificação do whisper (Perssua: remote config `whisper_prompt`).
///
/// Reduz alucinação: instrui o modelo a não inventar quando não tem certeza. A
/// regra de idioma é acrescentada pela camada de aplicação
/// (`domain::transcription::decode_prompt`).
pub const DEFAULT_WHISPER_PROMPT: &str =
    "Only transcribe what is clearly spoken. If uncertain, transcribe nothing.";

/// Teto de caracteres do prompt de decodificação.
pub const MAX_WHISPER_PROMPT_CHARS: usize = 1_000;

/// Modos de aceleração do ASR (`Settings.acceleration`).
///
/// - `auto` (padrão): usa GPU se o build tiver o backend e houver dispositivo;
/// - `cpu`: força CPU (útil para driver instável);
/// - `gpu`: exige GPU (o adapter tenta e cai em CPU se não houver dispositivo).
pub const ACCELERATION_MODES: [&str; 3] = ["auto", "cpu", "gpu"];

/// Indica se o modo de aceleração é suportado.
pub fn is_supported_acceleration(mode: &str) -> bool {
    ACCELERATION_MODES.contains(&mode.trim())
}

/// Provider de LLM suportado (BYO key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAi,
    Gemini,
    OpenRouter,
    /// DeepSeek — API compatível com OpenAI (BR-046).
    DeepSeek,
    Custom,
}

impl ProviderKind {
    /// Valor persistido/exposto.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Gemini => "gemini",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::DeepSeek => "deepseek",
            ProviderKind::Custom => "custom",
        }
    }

    /// Todos os providers, na ordem de exibição.
    pub fn all() -> [ProviderKind; 5] {
        [
            ProviderKind::OpenAi,
            ProviderKind::Gemini,
            ProviderKind::OpenRouter,
            ProviderKind::DeepSeek,
            ProviderKind::Custom,
        ]
    }

    /// Modelo sugerido por provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "gpt-4o-mini",
            ProviderKind::Gemini => "gemini-2.0-flash",
            ProviderKind::OpenRouter => "openai/gpt-4o-mini",
            ProviderKind::DeepSeek => "deepseek-chat",
            ProviderKind::Custom => "",
        }
    }

    /// `base_url` padrão (OpenAI-compatível) quando aplicável.
    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAi => Some("https://api.openai.com/v1"),
            ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1"),
            ProviderKind::DeepSeek => Some("https://api.deepseek.com/v1"),
            ProviderKind::Gemini | ProviderKind::Custom => None,
        }
    }

    /// `base_url` é obrigatória para endpoints custom.
    pub fn requires_base_url(&self) -> bool {
        matches!(self, ProviderKind::Custom)
    }

    /// Chave no `SecretStore` legada (uma por *tipo* de provider).
    ///
    /// Só existe para a **migração** das configurações antigas (BR-044): as
    /// fontes novas usam `AiSource::secret_key()` (uma chave por fonte).
    pub fn legacy_secret_key(&self) -> String {
        format!("llm.{}", self.as_str())
    }
}

/// Configurações persistidas em disco (nunca contêm segredos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Fontes de IA cadastradas, **ordenadas** (a primeira é a primária) —
    /// BR-044.
    #[serde(default)]
    pub ai_sources: Vec<AiSource>,
    /// Vínculos de fonte por escopo (agente/capacidade) — BR-048.
    #[serde(default)]
    pub ai_routes: Vec<AiRoute>,
    /// Provider legado (configurações anteriores às fontes) — só de leitura.
    ///
    /// É lido uma vez na migração e **nunca** serializado de novo.
    #[serde(default, rename = "provider", skip_serializing)]
    pub legacy_provider: Option<ProviderKind>,
    /// Modelo legado (configurações anteriores às fontes) — só de leitura.
    #[serde(default, rename = "model", skip_serializing)]
    pub legacy_model: Option<String>,
    /// Endpoint custom legado (configurações anteriores às fontes).
    #[serde(default, rename = "base_url", skip_serializing)]
    pub legacy_base_url: Option<String>,
    /// Idioma de origem da transcrição (`auto` para detecção).
    pub source_language: String,
    /// Idioma de destino da tradução.
    pub target_language: String,
    pub translation_enabled: bool,
    /// Modelo whisper.cpp local (`tiny`, `base`, ...).
    pub whisper_model: String,
    /// Prompt de decodificação (`initial_prompt`) do whisper.
    ///
    /// Vazio desliga o prompt (comportamento antigo). Ver
    /// [`DEFAULT_WHISPER_PROMPT`] e BR-027.
    #[serde(default = "default_whisper_prompt")]
    pub whisper_prompt: String,
    /// Aceleração do ASR: `auto` | `cpu` | `gpu` (ver [`ACCELERATION_MODES`]).
    #[serde(default = "default_acceleration")]
    pub acceleration: String,
    /// Cancelamento de eco (AEC) no microfone, usando o áudio do sistema como
    /// referência. Ligado por padrão; desligue se usar fones e quiser evitar
    /// qualquer processamento extra.
    #[serde(default = "default_enabled")]
    pub aec_enabled: bool,
    /// Ganho aplicado ao microfone antes da transcrição (1×–16×).
    ///
    /// O áudio do sistema nunca é amplificado. Ver BR-018 e
    /// [`crate::domain::audio::apply_gain`].
    #[serde(default = "default_mic_gain")]
    pub mic_gain: f32,
    /// Modo stealth (sem barra de tarefas / sem título).
    #[serde(default)]
    pub stealth: bool,
    /// Vínculos de atalhos globais.
    #[serde(default = "crate::domain::shortcuts::default_bindings")]
    pub shortcuts: Vec<ShortcutBinding>,
    /// Posição do overlay na tela (lembrada após o usuário mover).
    #[serde(default)]
    pub overlay_position: Option<(i32, i32)>,
    /// Posição da barra "Perguntar" (lembrada após o usuário mover).
    ///
    /// Só é reaplicada no boot; abrir, fechar, capturar a tela ou salvar
    /// settings **não** move a barra (BR-072).
    #[serde(default)]
    pub ask_position: Option<(i32, i32)>,
    /// Agente/modo selecionado para sugestões (`default` — o agente Padrão).
    #[serde(default = "default_assistant_mode")]
    pub assistant_mode_id: String,
    /// Marca a migração do agente padrão (BR-059).
    ///
    /// O agente **Padrão** (contextual) entrou no lugar da "Reunião" como
    /// padrão. A marca garante que a migração acontece **uma vez**: depois
    /// dela, a escolha do usuário é respeitada (mesmo se ele voltar para a
    /// "Reunião").
    #[serde(default)]
    pub assistant_mode_migrated: bool,
    /// Agentes personalizados criados pelo usuário.
    #[serde(default)]
    pub custom_assistant_modes: Vec<AssistantMode>,
    /// Atualização automática do resumo enquanto a janela está aberta.
    #[serde(default = "default_enabled")]
    pub summary_auto_refresh: bool,
    /// Intervalo do resumo automático (segundos).
    #[serde(default = "default_summary_interval")]
    pub summary_interval_secs: u32,
    /// Atualização automática da janela de sugestões.
    #[serde(default = "default_enabled")]
    pub suggestion_auto_refresh: bool,
    /// Intervalo das sugestões automáticas (segundos).
    #[serde(default = "default_suggestion_interval")]
    pub suggestion_interval_secs: u32,
    /// Posição da janela de sugestões.
    #[serde(default)]
    pub suggestions_position: Option<(i32, i32)>,
    /// Tamanho (px físicos) da janela de sugestões.
    #[serde(default)]
    pub suggestions_size: Option<(u32, u32)>,
    /// Posição da janela de chat (painel da sessão).
    #[serde(default)]
    pub chat_position: Option<(i32, i32)>,
    /// Posição da janela de histórico.
    #[serde(default)]
    pub history_position: Option<(i32, i32)>,
    /// Tamanho (px físicos) da janela de chat (painel da sessão).
    #[serde(default)]
    pub chat_size: Option<(u32, u32)>,
    /// Tamanho (px físicos) da janela de histórico.
    #[serde(default)]
    pub history_size: Option<(u32, u32)>,
    /// Aba ativa do painel da sessão: `transcription` | `summary` (BR-033).
    #[serde(default = "default_panel_tab")]
    pub panel_tab: String,
    /// Drill-hover (ativação por permanência) habilitado (BR-036).
    #[serde(default = "default_enabled")]
    pub dwell_enabled: bool,
    /// Tempo de permanência do drill-hover, em milissegundos (BR-036).
    #[serde(default = "default_dwell_ms")]
    pub dwell_ms: u32,
    /// Opacidade das superfícies de todas as janelas (0,2–1,0) — BR-062.
    #[serde(default = "default_overlay_opacity")]
    pub overlay_opacity: f32,
    /// Ancoragem do notch: `top_center` | `top_right` | `free` (BR-062).
    #[serde(default = "default_notch_anchor")]
    pub notch_anchor: String,
    /// Densidade do notch: `compact` | `standard` | `expanded` (BR-062).
    #[serde(default = "default_hud_density")]
    pub hud_density: String,
    /// Perguntas prontas do usuário (aba "Sessão", BR-054).
    #[serde(default = "default_saved_questions")]
    pub saved_questions: Vec<SavedQuestion>,
    /// Marca a migração das perguntas prontas (BR-054): o preset **novo**
    /// entra uma vez em quem já tinha a lista persistida, sem ressuscitar as
    /// que o usuário apagou.
    #[serde(default)]
    pub saved_questions_migrated: bool,
    /// O que gravar (BR-066): `video` | `audio`.
    #[serde(default = "default_recording_mode")]
    pub recording_mode: String,
    /// Alvo padrão (BR-070): `window` | `screen`.
    #[serde(default = "default_recording_target")]
    pub recording_target: String,
    /// Inicia a gravação junto com a sessão.
    #[serde(default)]
    pub recording_auto_start: bool,
    /// Segue a janela ativa: troca o alvo quando o foco muda.
    #[serde(default)]
    pub recording_follow_active: bool,
    /// FPS da gravação (degraus de 5 a 30 — BR-066).
    #[serde(default = "default_recording_fps")]
    pub recording_fps: u32,
    /// `native` | `1280x720` | `854x480`.
    #[serde(default = "default_recording_resolution")]
    pub recording_resolution: String,
    /// `compact` | `balanced` | `sharp` (CRF 30/27/24).
    #[serde(default = "default_recording_quality")]
    pub recording_quality: String,
    /// `auto` | `x264` | `vaapi`.
    #[serde(default = "default_recording_encoder")]
    pub recording_encoder: String,
    /// Bitrate do AAC em kbps (64–192).
    #[serde(default = "default_recording_audio_kbps")]
    pub recording_audio_kbps: u32,
    /// Container final: `mp4` | `mkv`.
    #[serde(default = "default_recording_container")]
    pub recording_container: String,
    /// Pasta de destino (vazio = `~/Vídeos/GhostWhisper`).
    #[serde(default)]
    pub recording_dir: Option<String>,
    /// Mantém os segmentos MKV após a finalização.
    #[serde(default)]
    pub recording_keep_segments: bool,
}

/// Uma pergunta pronta do usuário (aba "Sessão", BR-054).
///
/// Vive nas **settings** — é configuração do usuário, não dado de sessão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedQuestion {
    /// Id estável (para editar/remover sem depender da posição).
    pub id: String,
    /// O texto da pergunta.
    pub text: String,
}

/// Teto de perguntas prontas guardadas.
pub const MAX_SAVED_QUESTIONS: usize = 20;
/// Teto de caracteres de uma pergunta pronta.
pub const MAX_SAVED_QUESTION_CHARS: usize = 200;

/// Pedido acrescentado na v6 (BR-054): responde à última pergunta
/// identificada na conversa/tela — uma pergunta pronta que é uma **ação**.
pub const REPLY_TO_LAST_QUESTION: &str = "Responda a última pergunta identificada";

/// Perguntas prontas semeadas (tradução das "Prompt Suggestions" do Perssua).
pub const DEFAULT_SAVED_QUESTIONS: [&str; 5] = [
    "Resumir tudo até agora",
    "Listar as ações",
    "Listar as principais ideias",
    "O que posso falar?",
    REPLY_TO_LAST_QUESTION,
];

/// Perguntas prontas padrão (com ids derivados do texto).
pub fn default_saved_questions() -> Vec<SavedQuestion> {
    DEFAULT_SAVED_QUESTIONS
        .iter()
        .enumerate()
        .map(|(index, text)| SavedQuestion {
            id: format!("default-{index}"),
            text: (*text).to_string(),
        })
        .collect()
}

/// Rótulos das janelas de painel (ordem de boot e de persistência do layout).
///
/// O resumo **não** é mais uma janela: virou a aba `summary` do painel `chat`
/// (BR-033).
pub const PANEL_LABELS: [&str; 3] = ["suggestions", "chat", "history"];

/// Abas válidas do painel da sessão (janela `chat`).
pub const PANEL_TABS: [&str; 3] = ["transcription", "summary", "session"];

/// Intervalo mínimo aceito para atualizações automáticas (segundos).
pub const MIN_AUTO_INTERVAL_SECS: u32 = 10;
/// Intervalo máximo aceito para atualizações automáticas (segundos).
pub const MAX_AUTO_INTERVAL_SECS: u32 = 120;
/// Tempo padrão do drill-hover (ms) — BR-036.
pub const DEFAULT_DWELL_MS: u32 = 500;
/// Tempo mínimo aceito do drill-hover (ms) — faixa do mockup (BR-062).
pub const MIN_DWELL_MS: u32 = 100;
/// Tempo máximo aceito do drill-hover (ms).
pub const MAX_DWELL_MS: u32 = 1_500;
/// Opacidade padrão da superfície do notch (BR-062).
pub const DEFAULT_OVERLAY_OPACITY: f32 = 0.75;
/// Opacidade mínima aceita (20 %).
pub const MIN_OVERLAY_OPACITY: f32 = 0.2;
/// Opacidade máxima aceita (100 %).
pub const MAX_OVERLAY_OPACITY: f32 = 1.0;
/// Valores aceitos de ancoragem do notch (BR-062).
pub const NOTCH_ANCHORS: [&str; 3] = ["top_center", "top_right", "free"];
/// Ancoragem padrão: centralizado no topo.
pub const DEFAULT_NOTCH_ANCHOR: &str = "top_center";
/// Valores aceitos de densidade do notch (BR-062).
pub const HUD_DENSITIES: [&str; 3] = ["compact", "standard", "expanded"];
/// Densidade padrão (72 px de janela).
pub const DEFAULT_HUD_DENSITY: &str = "standard";
/// Largura mínima dos painéis (piso salvo; o mínimo real de cada janela está
/// no `tauri.conf.json` — o de Sugestões é 520 px, para o código caber).
pub const MIN_PANEL_WIDTH: u32 = 320;
/// Altura mínima dos painéis (piso salvo; o mínimo real de cada janela está no
/// `tauri.conf.json`).
pub const MIN_PANEL_HEIGHT: u32 = 240;

/// Id reservado da fonte padrão (instalação nova / migração).
pub const DEFAULT_SOURCE_ID: &str = "default";

/// Fonte padrão: OpenAI com `gpt-4o-mini` (sem chave — o usuário cadastra a
/// dele).
pub fn default_source() -> AiSource {
    let mut source = AiSource::new(DEFAULT_SOURCE_ID, ProviderKind::OpenAi);
    source.label = "OpenAI".to_string();
    source
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ai_sources: vec![default_source()],
            ai_routes: Vec::new(),
            legacy_provider: None,
            legacy_model: None,
            legacy_base_url: None,
            source_language: "auto".to_string(),
            target_language: "pt".to_string(),
            translation_enabled: false,
            whisper_model: "tiny".to_string(),
            whisper_prompt: default_whisper_prompt(),
            acceleration: default_acceleration(),
            aec_enabled: true,
            mic_gain: default_mic_gain(),
            stealth: false,
            shortcuts: crate::domain::shortcuts::default_bindings(),
            overlay_position: None,
            ask_position: None,
            assistant_mode_id: default_assistant_mode(),
            assistant_mode_migrated: false,
            custom_assistant_modes: Vec::new(),
            summary_auto_refresh: true,
            summary_interval_secs: default_summary_interval(),
            suggestion_auto_refresh: true,
            suggestion_interval_secs: default_suggestion_interval(),
            suggestions_position: None,
            suggestions_size: None,
            chat_position: None,
            history_position: None,
            chat_size: None,
            history_size: None,
            panel_tab: default_panel_tab(),
            dwell_enabled: true,
            dwell_ms: default_dwell_ms(),
            overlay_opacity: default_overlay_opacity(),
            notch_anchor: default_notch_anchor(),
            hud_density: default_hud_density(),
            saved_questions: default_saved_questions(),
            saved_questions_migrated: false,
            recording_mode: default_recording_mode(),
            recording_target: default_recording_target(),
            recording_auto_start: false,
            recording_follow_active: false,
            recording_fps: default_recording_fps(),
            recording_resolution: default_recording_resolution(),
            recording_quality: default_recording_quality(),
            recording_encoder: default_recording_encoder(),
            recording_audio_kbps: default_recording_audio_kbps(),
            recording_container: default_recording_container(),
            recording_dir: None,
            recording_keep_segments: false,
        }
    }
}

/// Configuração de gravação que entra em instalações novas (BR-066).
fn default_recording_mode() -> String {
    crate::domain::recording::DEFAULT_RECORDING_MODE.to_string()
}
fn default_recording_target() -> String {
    crate::domain::recording::DEFAULT_RECORDING_TARGET.to_string()
}
fn default_recording_fps() -> u32 {
    crate::domain::recording::DEFAULT_RECORDING_FPS
}
fn default_recording_resolution() -> String {
    crate::domain::recording::DEFAULT_RECORDING_RESOLUTION.to_string()
}
fn default_recording_quality() -> String {
    crate::domain::recording::DEFAULT_RECORDING_QUALITY.to_string()
}
fn default_recording_encoder() -> String {
    crate::domain::recording::DEFAULT_RECORDING_ENCODER.to_string()
}
fn default_recording_audio_kbps() -> u32 {
    crate::domain::recording::DEFAULT_RECORDING_AUDIO_KBPS
}
fn default_recording_container() -> String {
    crate::domain::recording::DEFAULT_RECORDING_CONTAINER.to_string()
}

/// Aba padrão do painel da sessão (Transcrição).
fn default_panel_tab() -> String {
    "transcription".to_string()
}

/// Sanitiza as perguntas prontas: texto vazio sai, o texto é limitado a
/// [`MAX_SAVED_QUESTION_CHARS`] e a lista a [`MAX_SAVED_QUESTIONS`] itens.
///
/// O **excesso é recusado** no save (a UI avisa); no load, o que passa do teto
/// é descartado para o arquivo não crescer sem limite.
fn sanitize_saved_questions(questions: Vec<SavedQuestion>) -> Vec<SavedQuestion> {
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<SavedQuestion> = Vec::new();
    for question in questions {
        let text = question.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let text: String = text.chars().take(MAX_SAVED_QUESTION_CHARS).collect();
        let id = if question.id.trim().is_empty() {
            format!("q-{}", out.len())
        } else {
            question.id.trim().to_string()
        };
        if seen.contains(&id) {
            continue;
        }
        seen.push(id.clone());
        out.push(SavedQuestion { id, text });
        if out.len() >= MAX_SAVED_QUESTIONS {
            break;
        }
    }
    out
}

/// Corrige uma aba desconhecida/vazia para a padrão.
fn sanitize_panel_tab(value: &str) -> String {
    let normalized = value.trim().to_lowercase();
    if PANEL_TABS.contains(&normalized.as_str()) {
        normalized
    } else {
        default_panel_tab()
    }
}

fn default_assistant_mode() -> String {
    crate::domain::assistant::DEFAULT_MODE_ID.to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_summary_interval() -> u32 {
    30
}

fn default_suggestion_interval() -> u32 {
    20
}

fn default_dwell_ms() -> u32 {
    DEFAULT_DWELL_MS
}

fn default_overlay_opacity() -> f32 {
    DEFAULT_OVERLAY_OPACITY
}

fn default_notch_anchor() -> String {
    DEFAULT_NOTCH_ANCHOR.to_string()
}

fn default_hud_density() -> String {
    DEFAULT_HUD_DENSITY.to_string()
}

/// `true` se a ancoragem é uma das aceitas (BR-062).
fn is_supported_anchor(value: &str) -> bool {
    crate::domain::hud_layout::NotchAnchor::ALL
        .iter()
        .any(|anchor| anchor.as_str() == value)
}

/// `true` se a densidade é uma das aceitas (BR-062).
fn is_supported_density(value: &str) -> bool {
    crate::domain::hud_layout::HudDensity::ALL
        .iter()
        .any(|density| density.as_str() == value)
}

/// Traz um tamanho de painel para o mínimo permitido.
fn clamp_panel_size(size: Option<(u32, u32)>) -> Option<(u32, u32)> {
    size.map(|(width, height)| (width.max(MIN_PANEL_WIDTH), height.max(MIN_PANEL_HEIGHT)))
}

/// Ganho de microfone padrão (Perssua: `local_whisper_user_mic_gain = 8`).
fn default_mic_gain() -> f32 {
    crate::domain::audio::DEFAULT_MIC_GAIN
}

/// Prompt de decodificação padrão (Perssua: `whisper_prompt`).
fn default_whisper_prompt() -> String {
    DEFAULT_WHISPER_PROMPT.to_string()
}

/// Aceleração padrão: `auto` (GPU quando o build/hardware permitir).
fn default_acceleration() -> String {
    "auto".to_string()
}

impl Settings {
    /// Valida invariantes antes de persistir/usar.
    pub fn validate(&self) -> Result<()> {
        validate_sources(&self.ai_sources)?;
        if let Err(message) = validate_routes(&self.ai_routes) {
            return Err(GhostWhisperError::Config(message));
        }
        if self.whisper_prompt.contains('\0') {
            return Err(GhostWhisperError::Config(
                "prompt de decodificação contém caractere nulo".into(),
            ));
        }
        if self.whisper_prompt.chars().count() > MAX_WHISPER_PROMPT_CHARS {
            return Err(GhostWhisperError::Config(format!(
                "prompt de decodificação deve ter no máximo {MAX_WHISPER_PROMPT_CHARS} caracteres"
            )));
        }
        if !is_supported_acceleration(&self.acceleration) {
            return Err(GhostWhisperError::Config(format!(
                "aceleração inválida: {} (use auto, cpu ou gpu)",
                self.acceleration
            )));
        }
        if self.target_language.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "idioma de destino não pode ser vazio".into(),
            ));
        }
        if !is_supported_language(&self.source_language) {
            return Err(GhostWhisperError::Config(format!(
                "idioma de origem não suportado: {} (use auto, pt ou en)",
                self.source_language
            )));
        }
        if !is_supported_language(&self.target_language) {
            return Err(GhostWhisperError::Config(format!(
                "idioma de destino não suportado: {} (use auto, pt ou en)",
                self.target_language
            )));
        }
        validate_custom_modes(&self.custom_assistant_modes)?;
        if !self.selected_mode_exists() {
            return Err(GhostWhisperError::Config(format!(
                "agente selecionado não existe: {}",
                self.assistant_mode_id
            )));
        }
        if !(MIN_AUTO_INTERVAL_SECS..=MAX_AUTO_INTERVAL_SECS).contains(&self.summary_interval_secs)
        {
            return Err(GhostWhisperError::Config(format!(
                "intervalo do resumo deve ficar entre {MIN_AUTO_INTERVAL_SECS} e {MAX_AUTO_INTERVAL_SECS} s"
            )));
        }
        if !(MIN_AUTO_INTERVAL_SECS..=MAX_AUTO_INTERVAL_SECS)
            .contains(&self.suggestion_interval_secs)
        {
            return Err(GhostWhisperError::Config(format!(
                "intervalo das sugestões deve ficar entre {MIN_AUTO_INTERVAL_SECS} e {MAX_AUTO_INTERVAL_SECS} s"
            )));
        }
        if !(MIN_DWELL_MS..=MAX_DWELL_MS).contains(&self.dwell_ms) {
            return Err(GhostWhisperError::Config(format!(
                "tempo do drill-hover deve ficar entre {MIN_DWELL_MS} e {MAX_DWELL_MS} ms"
            )));
        }
        if !self.overlay_opacity.is_finite()
            || !(MIN_OVERLAY_OPACITY..=MAX_OVERLAY_OPACITY).contains(&self.overlay_opacity)
        {
            return Err(GhostWhisperError::Config(format!(
                "opacidade das janelas deve ficar entre {MIN_OVERLAY_OPACITY} e {MAX_OVERLAY_OPACITY}"
            )));
        }
        if !is_supported_anchor(&self.notch_anchor) {
            return Err(GhostWhisperError::Config(format!(
                "ancoragem do notch inválida: {} (use {})",
                self.notch_anchor,
                NOTCH_ANCHORS.join(", ")
            )));
        }
        if !is_supported_density(&self.hud_density) {
            return Err(GhostWhisperError::Config(format!(
                "densidade do HUD inválida: {} (use {})",
                self.hud_density,
                HUD_DENSITIES.join(", ")
            )));
        }
        Ok(())
    }

    /// Indica se o agente selecionado existe (embutido ou personalizado).
    pub fn selected_mode_exists(&self) -> bool {
        is_builtin_id(&self.assistant_mode_id)
            || self
                .custom_assistant_modes
                .iter()
                .any(|mode| mode.id == self.assistant_mode_id)
    }

    /// Corrige valores fora do contrato (ex.: settings antigas com idioma livre).
    pub fn sanitized(mut self) -> Self {
        self.ai_sources = sanitize_sources(
            self.ai_sources,
            &self.legacy_provider,
            &self.legacy_model,
            &self.legacy_base_url,
        );
        // Rotas que apontam para uma fonte inexistente são descartadas (a fonte
        // foi removida pelo usuário): o escopo volta a usar a primária.
        self.ai_routes.retain(|route| {
            self.ai_sources
                .iter()
                .any(|source| source.id == route.source_id)
        });
        if !is_supported_acceleration(&self.acceleration) {
            self.acceleration = default_acceleration();
        }
        if !is_supported_language(&self.source_language) {
            self.source_language = "auto".to_string();
        }
        if !is_supported_language(&self.target_language) {
            self.target_language = "pt".to_string();
        }
        self.mic_gain = crate::domain::audio::clamp_mic_gain(self.mic_gain);
        // Prompt de decodificação: sem NUL (o `whisper-rs` entra em pânico) e no teto.
        self.whisper_prompt = self
            .whisper_prompt
            .replace('\0', "")
            .chars()
            .take(MAX_WHISPER_PROMPT_CHARS)
            .collect();
        // Agentes inválidos são descartados ao carregar um arquivo antigo/Manual;
        // no save, `validate` recusa em vez de descartar. A identidade (ícone/cor)
        // é normalizada antes: um ícone desconhecido não derruba o agente (BR-037).
        self.custom_assistant_modes = self
            .custom_assistant_modes
            .into_iter()
            .map(|mode| mode.with_valid_identity())
            .collect();
        self.custom_assistant_modes
            .retain(|mode| crate::domain::assistant::validate_mode(mode).is_ok());
        if !self.selected_mode_exists() {
            self.assistant_mode_id = default_assistant_mode();
        }
        self.summary_interval_secs = self
            .summary_interval_secs
            .clamp(MIN_AUTO_INTERVAL_SECS, MAX_AUTO_INTERVAL_SECS);
        self.suggestion_interval_secs = self
            .suggestion_interval_secs
            .clamp(MIN_AUTO_INTERVAL_SECS, MAX_AUTO_INTERVAL_SECS);
        self.dwell_ms = self.dwell_ms.clamp(MIN_DWELL_MS, MAX_DWELL_MS);
        // HUD (BR-062): opacidade fora da faixa é trazida para dentro; âncora e
        // densidade desconhecidas caem no padrão.
        if !self.overlay_opacity.is_finite() {
            self.overlay_opacity = DEFAULT_OVERLAY_OPACITY;
        }
        self.overlay_opacity = self
            .overlay_opacity
            .clamp(MIN_OVERLAY_OPACITY, MAX_OVERLAY_OPACITY);
        if !is_supported_anchor(&self.notch_anchor) {
            self.notch_anchor = default_notch_anchor();
        }
        if !is_supported_density(&self.hud_density) {
            self.hud_density = default_hud_density();
        }
        self.suggestions_size = clamp_panel_size(self.suggestions_size);
        self.chat_size = clamp_panel_size(self.chat_size);
        self.history_size = clamp_panel_size(self.history_size);
        self.panel_tab = sanitize_panel_tab(&self.panel_tab);
        self.saved_questions = sanitize_saved_questions(self.saved_questions);
        self
    }

    /// Preserva a geometria de janelas (posição/tamanho) já persistida.
    ///
    /// O frontend envia um snapshot completo das settings em cada save. As
    /// posições/tamanhos são escritos pelos **trackers de janela do core**;
    /// se um save de outra janela (aba, agente, select) trouxesse um snapshot
    /// antigo, ele moveria o notch/painéis sem o usuário ter movido nada —
    /// por isso o valor persistido sempre vence o payload (ISS-045).
    pub fn preserve_window_geometry(&mut self, persisted: &Settings) {
        self.overlay_position = persisted.overlay_position;
        self.ask_position = persisted.ask_position;
        self.suggestions_position = persisted.suggestions_position;
        self.suggestions_size = persisted.suggestions_size;
        self.chat_position = persisted.chat_position;
        self.chat_size = persisted.chat_size;
        self.history_position = persisted.history_position;
        self.history_size = persisted.history_size;
    }

    /// Indica se a âncora ou a densidade do HUD mudou entre dois snapshots.
    ///
    /// Só nesse caso o layout é reaplicado: um save qualquer não pode
    /// reposicionar o notch (ISS-045).
    pub fn hud_layout_changed(&self, other: &Settings) -> bool {
        use crate::domain::hud_layout::{HudDensity, NotchAnchor};

        HudDensity::parse(&self.hud_density) != HudDensity::parse(&other.hud_density)
            || NotchAnchor::parse(&self.notch_anchor) != NotchAnchor::parse(&other.notch_anchor)
    }

    /// Posição persistida de um painel pelo rótulo (`None` se desconhecido).
    pub fn panel_position(&self, label: &str) -> Option<(i32, i32)> {
        match label {
            "suggestions" => self.suggestions_position,
            "chat" => self.chat_position,
            "history" => self.history_position,
            _ => None,
        }
    }

    /// Atualiza a posição persistida de um painel pelo rótulo.
    pub fn set_panel_position(&mut self, label: &str, position: Option<(i32, i32)>) {
        match label {
            "suggestions" => self.suggestions_position = position,
            "chat" => self.chat_position = position,
            "history" => self.history_position = position,
            _ => {}
        }
    }

    /// Tamanho persistido (px físicos) de um painel pelo rótulo.
    pub fn panel_size(&self, label: &str) -> Option<(u32, u32)> {
        match label {
            "suggestions" => self.suggestions_size,
            "chat" => self.chat_size,
            "history" => self.history_size,
            _ => None,
        }
    }

    /// Atualiza o tamanho persistido (px físicos) de um painel pelo rótulo.
    pub fn set_panel_size(&mut self, label: &str, size: Option<(u32, u32)>) {
        match label {
            "suggestions" => self.suggestions_size = size,
            "chat" => self.chat_size = size,
            "history" => self.history_size = size,
            _ => {}
        }
    }

    /// Fonte de IA pelo id.
    pub fn source(&self, source_id: &str) -> Option<&AiSource> {
        self.ai_sources.iter().find(|source| source.id == source_id)
    }
}

/// Normaliza a lista de fontes (BR-044).
///
/// - Lista vazia + campos legados ⇒ migra para **uma** fonte (configurações
///   anteriores às fontes).
/// - Lista vazia sem legados ⇒ a fonte padrão (o app sempre tem uma fonte).
/// - Fontes inválidas são descartadas; ids duplicados são removidos.
fn sanitize_sources(
    mut sources: Vec<AiSource>,
    legacy_provider: &Option<ProviderKind>,
    legacy_model: &Option<String>,
    legacy_base_url: &Option<String>,
) -> Vec<AiSource> {
    if sources.is_empty() {
        let mut source = match legacy_provider {
            Some(kind) => {
                let mut source = AiSource::new(DEFAULT_SOURCE_ID, *kind);
                if let Some(model) = legacy_model.as_ref().filter(|m| !m.trim().is_empty()) {
                    source.model = model.clone();
                }
                if let Some(url) = legacy_base_url.as_ref().filter(|u| !u.trim().is_empty()) {
                    source.base_url = Some(url.clone());
                }
                source
            }
            None => default_source(),
        };
        // Sem modelo (custom sem `base_url`) a fonte é inválida: cai na padrão.
        if source.model.trim().is_empty() {
            source = default_source();
        }
        sources.push(source);
    }

    let mut seen = std::collections::HashSet::new();
    sources.retain(|source| {
        if source.id.trim().is_empty() || !seen.insert(source.id.clone()) {
            return false;
        }
        crate::domain::ai_source::validate_source(source).is_ok()
    });
    if sources.is_empty() {
        sources.push(default_source());
    }
    sources
}

/// Mascara um segredo para exibição na UI (nunca devolve o valor bruto).
///
/// Regra de produto: o campo mostra `••••••••·abcd` (8 balas + últimos 4
/// caracteres) para indicar que a chave **está lá**; chaves curtas (< 8) não
/// expõem o sufixo, e chaves vazias viram string vazia.
pub fn redact_secret(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.chars().count() < 8 {
        if trimmed.is_empty() {
            return String::new();
        }
        return "••••••••".to_string();
    }
    let tail: String = trimmed.chars().skip(trimmed.chars().count() - 4).collect();
    format!("••••••••·{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_round_trips() {
        for kind in ProviderKind::all() {
            let json = serde_json::to_string(&kind).expect("serializa");
            let parsed: ProviderKind = serde_json::from_str(&json).expect("desserializa");
            assert_eq!(parsed, kind);
            assert!(!kind.as_str().is_empty());
        }
        assert!(serde_json::from_str::<ProviderKind>("\"nope\"").is_err());
    }

    #[test]
    fn default_settings_are_valid() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
        assert!(!settings.translation_enabled);
    }

    #[test]
    fn custom_requires_base_url() {
        let mut custom = crate::domain::ai_source::AiSource::new("custom-1", ProviderKind::Custom);
        custom.model = "llama-3.1-8b".into();
        let missing_base_url = Settings {
            ai_sources: vec![custom.clone()],
            ..Settings::default()
        };
        assert!(missing_base_url.validate().is_err());

        custom.base_url = Some("https://ollama.local/v1".into());
        let with_base_url = Settings {
            ai_sources: vec![custom],
            ..Settings::default()
        };
        assert!(with_base_url.validate().is_ok());
    }

    #[test]
    fn openai_has_default_base_url() {
        let settings = Settings::default();
        let source = settings.ai_sources.first().expect("fonte padrão");
        assert_eq!(source.kind, ProviderKind::OpenAi);
        assert_eq!(
            source.effective_base_url().as_deref(),
            Some("https://api.openai.com/v1")
        );
    }

    #[test]
    fn legacy_settings_become_one_source() {
        // Config anterior às fontes (BR-044): o provider/modelo viram uma fonte.
        let legacy = Settings {
            ai_sources: vec![],
            legacy_provider: Some(ProviderKind::OpenRouter),
            legacy_model: Some("meta-llama/llama-3.1-8b".into()),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(legacy.ai_sources.len(), 1);
        assert_eq!(legacy.ai_sources[0].kind, ProviderKind::OpenRouter);
        assert_eq!(legacy.ai_sources[0].model, "meta-llama/llama-3.1-8b");
        assert_eq!(legacy.ai_sources[0].id, DEFAULT_SOURCE_ID);
        assert!(legacy.validate().is_ok());
    }

    #[test]
    fn legacy_fields_are_not_serialized_again() {
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        let value: serde_json::Value = serde_json::from_str(&json).expect("json");
        let object = value.as_object().expect("objeto");
        // Os campos legados sumiram do arquivo…
        assert!(!object.contains_key("provider"));
        assert!(!object.contains_key("model"));
        assert!(!object.contains_key("base_url"));
        // …e a lista de fontes está lá (com o modelo de cada fonte).
        assert!(object.contains_key("ai_sources"));
        assert!(json.contains("\"model\""));
    }

    #[test]
    fn routes_to_unknown_sources_are_dropped() {
        let settings = Settings {
            ai_routes: vec![crate::domain::ai_route::AiRoute::new(
                crate::domain::ai_route::AiRouteScope::Agent("meeting".into()),
                "nao-existe",
            )],
            ..Settings::default()
        }
        .sanitized();
        assert!(settings.ai_routes.is_empty());
    }

    #[test]
    fn settings_serialize_without_secret_fields() {
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        assert!(!json.contains("key"));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn redact_secret_hides_value_but_shows_tail() {
        let masked = redact_secret("sk-proj-abcdefgh-1234");
        assert_eq!(masked, "••••••••·1234");
        // Nunca devolve o segredo inteiro nem a maior parte dele.
        assert!(!masked.contains("sk-proj"));
        assert!(!masked.contains("abcdefgh"));
    }

    #[test]
    fn redact_secret_hides_short_keys_completely() {
        assert_eq!(redact_secret("abc"), "••••••••");
        assert_eq!(redact_secret("  "), "");
        assert_eq!(redact_secret(""), "");
    }

    #[test]
    fn rejects_unsupported_languages() {
        let source = Settings {
            source_language: "es".into(),
            ..Settings::default()
        };
        assert!(source.validate().is_err());

        let target = Settings {
            target_language: "de".into(),
            ..Settings::default()
        };
        assert!(target.validate().is_err());

        let ok = Settings {
            source_language: "pt".into(),
            target_language: "en".into(),
            ..Settings::default()
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn language_names_are_readable() {
        assert_eq!(language_name("pt"), "português");
        assert_eq!(language_name("en"), "inglês");
        assert_eq!(language_name("auto"), "o mesmo idioma da transcrição");
        assert!(is_supported_language("auto"));
        assert!(!is_supported_language("fr"));
    }

    #[test]
    fn sanitized_coerces_unsupported_languages() {
        let dirty = Settings {
            source_language: "es".into(),
            target_language: "de".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.source_language, "auto");
        assert_eq!(dirty.target_language, "pt");
        assert!(dirty.validate().is_ok());
    }

    #[test]
    fn default_assistant_settings_are_valid() {
        let settings = Settings::default();
        assert_eq!(settings.assistant_mode_id, "default");
        assert!(!settings.assistant_mode_migrated);
        assert_eq!(settings.summary_interval_secs, 30);
        assert_eq!(settings.suggestion_interval_secs, 20);
        assert!(settings.custom_assistant_modes.is_empty());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn legacy_settings_get_assistant_defaults() {
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json
            .replace("\"assistant_mode_id\":\"default\",", "")
            .replace("\"assistant_mode_migrated\":false,", "")
            .replace("\"custom_assistant_modes\":[],", "")
            .replace("\"summary_auto_refresh\":true,", "")
            .replace("\"summary_interval_secs\":30,", "")
            .replace("\"suggestion_auto_refresh\":true,", "")
            .replace("\"suggestion_interval_secs\":20,", "")
            .replace("\"suggestions_position\":null,", "");
        assert!(!legacy.contains("assistant_mode_id"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.assistant_mode_id, "default");
        assert!(!parsed.assistant_mode_migrated);
        assert_eq!(parsed.summary_interval_secs, 30);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn sanitized_falls_back_when_the_selected_mode_is_missing() {
        let dirty = Settings {
            assistant_mode_id: "custom-removido".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.assistant_mode_id, "default");
        assert!(dirty.validate().is_ok());
    }

    #[test]
    fn sanitized_clamps_auto_intervals() {
        let dirty = Settings {
            summary_interval_secs: 1,
            suggestion_interval_secs: 9_999,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.summary_interval_secs, MIN_AUTO_INTERVAL_SECS);
        assert_eq!(dirty.suggestion_interval_secs, MAX_AUTO_INTERVAL_SECS);
        assert!(dirty.validate().is_ok());
    }

    #[test]
    fn dwell_defaults_are_enabled_at_half_a_second() {
        let settings = Settings::default();
        assert!(settings.dwell_enabled);
        assert_eq!(settings.dwell_ms, DEFAULT_DWELL_MS);
        assert_eq!(DEFAULT_DWELL_MS, 500);
    }

    #[test]
    fn legacy_settings_get_dwell_defaults() {
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json
            .replace(",\"dwell_enabled\":true", "")
            .replace(",\"dwell_ms\":500", "");
        assert!(!legacy.contains("dwell_enabled"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert!(parsed.dwell_enabled);
        assert_eq!(parsed.dwell_ms, DEFAULT_DWELL_MS);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn sanitized_clamps_dwell_and_weight_validates() {
        let dirty = Settings {
            dwell_ms: 1,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.dwell_ms, MIN_DWELL_MS);

        let dirty = Settings {
            dwell_ms: 9_999,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.dwell_ms, MAX_DWELL_MS);

        let invalid = Settings {
            dwell_ms: MIN_DWELL_MS - 1,
            ..Settings::default()
        };
        assert!(invalid.validate().is_err());
        let valid = Settings {
            dwell_ms: MAX_DWELL_MS,
            dwell_enabled: false,
            ..Settings::default()
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn hud_defaults_are_balanced() {
        let settings = Settings::default();
        assert_eq!(settings.overlay_opacity, DEFAULT_OVERLAY_OPACITY);
        assert_eq!(settings.notch_anchor, "top_center");
        assert_eq!(settings.hud_density, "standard");
        assert_eq!(DEFAULT_OVERLAY_OPACITY, 0.75);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn legacy_settings_get_hud_defaults() {
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json
            .replace(",\"overlay_opacity\":0.75", "")
            .replace(",\"notch_anchor\":\"top_center\"", "")
            .replace(",\"hud_density\":\"standard\"", "");
        assert!(!legacy.contains("overlay_opacity"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.overlay_opacity, DEFAULT_OVERLAY_OPACITY);
        assert_eq!(parsed.notch_anchor, DEFAULT_NOTCH_ANCHOR);
        assert_eq!(parsed.hud_density, DEFAULT_HUD_DENSITY);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn sanitized_clamps_opacity_and_resets_unknown_hud_values() {
        let dirty = Settings {
            overlay_opacity: 0.05,
            notch_anchor: "canto_do_mundo".to_string(),
            hud_density: "gigante".to_string(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.overlay_opacity, MIN_OVERLAY_OPACITY);
        assert_eq!(dirty.notch_anchor, DEFAULT_NOTCH_ANCHOR);
        assert_eq!(dirty.hud_density, DEFAULT_HUD_DENSITY);
        assert!(dirty.validate().is_ok());

        let nan = Settings {
            overlay_opacity: f32::NAN,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(nan.overlay_opacity, DEFAULT_OVERLAY_OPACITY);
    }

    #[test]
    fn hud_validation_rejects_out_of_contract() {
        let opacity = Settings {
            overlay_opacity: 1.5,
            ..Settings::default()
        };
        assert!(opacity.validate().is_err());

        let anchor = Settings {
            notch_anchor: "esquerda".to_string(),
            ..Settings::default()
        };
        assert!(anchor.validate().is_err());

        let density = Settings {
            hud_density: "enorme".to_string(),
            ..Settings::default()
        };
        assert!(density.validate().is_err());

        let valid = Settings {
            overlay_opacity: MAX_OVERLAY_OPACITY,
            notch_anchor: "free".to_string(),
            hud_density: "expanded".to_string(),
            ..Settings::default()
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn sanitized_clamps_panel_size_to_the_minimum() {
        let dirty = Settings {
            suggestions_size: Some((900, 700)),
            chat_size: Some((10, 10)),
            history_size: Some((640, 480)),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.suggestions_size, Some((900, 700)));
        assert_eq!(dirty.chat_size, Some((MIN_PANEL_WIDTH, MIN_PANEL_HEIGHT)));
        assert_eq!(dirty.history_size, Some((640, 480)));
        assert!(dirty.validate().is_ok());
    }

    #[test]
    fn panel_tab_defaults_to_transcription_and_sanitizes() {
        assert_eq!(Settings::default().panel_tab, "transcription");
        assert_eq!(
            Settings {
                panel_tab: " SUMMARY ".into(),
                ..Settings::default()
            }
            .sanitized()
            .panel_tab,
            "summary"
        );
        assert_eq!(
            Settings {
                panel_tab: "desconhecida".into(),
                ..Settings::default()
            }
            .sanitized()
            .panel_tab,
            "transcription"
        );
    }

    #[test]
    fn panel_layout_accessors_round_trip_for_every_label() {
        let mut settings = Settings::default();
        for label in PANEL_LABELS {
            assert_eq!(settings.panel_position(label), None);
            assert_eq!(settings.panel_size(label), None);

            settings.set_panel_position(label, Some((10, 20)));
            settings.set_panel_size(label, Some((640, 480)));
            assert_eq!(settings.panel_position(label), Some((10, 20)));
            assert_eq!(settings.panel_size(label), Some((640, 480)));
        }

        // Rótulo desconhecido não inventa um slot.
        assert_eq!(settings.panel_position("desconhecido"), None);
        assert_eq!(settings.panel_size("desconhecido"), None);
        settings.set_panel_position("desconhecido", Some((1, 1)));
        settings.set_panel_size("desconhecido", Some((1, 1)));
        assert_eq!(settings.panel_position("desconhecido"), None);
    }

    #[test]
    fn preserve_window_geometry_ignores_stale_payload_geometry() {
        let persisted = Settings {
            overlay_position: Some((570, 312)),
            ask_position: Some((400, 80)),
            suggestions_position: Some((10, 20)),
            suggestions_size: Some((420, 640)),
            chat_position: Some((30, 40)),
            chat_size: Some((760, 680)),
            history_position: Some((50, 60)),
            history_size: Some((800, 700)),
            ..Settings::default()
        };
        let mut payload = Settings {
            overlay_position: Some((0, 0)),
            ask_position: Some((1, 2)),
            suggestions_position: None,
            suggestions_size: Some((300, 300)),
            chat_position: Some((1, 1)),
            chat_size: Some((640, 480)),
            history_position: None,
            history_size: None,
            assistant_mode_id: "leetcode".into(),
            ..Settings::default()
        };

        payload.preserve_window_geometry(&persisted);

        assert_eq!(payload.overlay_position, Some((570, 312)));
        assert_eq!(payload.ask_position, Some((400, 80)));
        assert_eq!(payload.suggestions_position, Some((10, 20)));
        assert_eq!(payload.suggestions_size, Some((420, 640)));
        assert_eq!(payload.chat_position, Some((30, 40)));
        assert_eq!(payload.chat_size, Some((760, 680)));
        assert_eq!(payload.history_position, Some((50, 60)));
        assert_eq!(payload.history_size, Some((800, 700)));
        // O resto do payload (configuração de fato) é preservado.
        assert_eq!(payload.assistant_mode_id, "leetcode");
    }

    #[test]
    fn hud_layout_changed_only_for_anchor_or_density() {
        let base = Settings::default();
        let unrelated = Settings {
            panel_tab: "summary".into(),
            assistant_mode_id: "leetcode".into(),
            ..Settings::default()
        };
        assert!(!unrelated.hud_layout_changed(&base));

        let density = Settings {
            hud_density: "expanded".into(),
            ..Settings::default()
        };
        assert!(density.hud_layout_changed(&base));

        let anchor = Settings {
            notch_anchor: "free".into(),
            ..Settings::default()
        };
        assert!(anchor.hud_layout_changed(&base));

        // Valores equivalentes (com espaços/caixa) não contam como mudança.
        let equivalent = Settings {
            hud_density: " EXPANDED ".into(),
            ..Settings::default()
        };
        assert!(!equivalent.hud_layout_changed(&Settings {
            hud_density: "expanded".into(),
            ..Settings::default()
        }));
    }

    #[test]
    fn whisper_prompt_defaults_to_the_perssua_contract() {
        assert_eq!(Settings::default().whisper_prompt, DEFAULT_WHISPER_PROMPT);
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        let legacy = json.replace(
            &format!("\"whisper_prompt\":\"{DEFAULT_WHISPER_PROMPT}\","),
            "",
        );
        assert!(!legacy.contains("whisper_prompt"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.whisper_prompt, DEFAULT_WHISPER_PROMPT);
    }

    #[test]
    fn whisper_prompt_can_be_disabled_and_is_bounded() {
        let disabled = Settings {
            whisper_prompt: String::new(),
            ..Settings::default()
        };
        assert!(disabled.validate().is_ok());

        let huge = Settings {
            whisper_prompt: "a".repeat(MAX_WHISPER_PROMPT_CHARS + 1),
            ..Settings::default()
        };
        assert!(huge.validate().is_err());
        let clamped = huge.sanitized();
        assert_eq!(
            clamped.whisper_prompt.chars().count(),
            MAX_WHISPER_PROMPT_CHARS
        );
    }

    #[test]
    fn whisper_prompt_rejects_null_bytes_and_sanitizes_them() {
        let dirty = Settings {
            whisper_prompt: "a\0b".into(),
            ..Settings::default()
        };
        assert!(dirty.validate().is_err());
        assert_eq!(dirty.sanitized().whisper_prompt, "ab");
    }

    #[test]
    fn acceleration_defaults_to_auto_and_is_validated() {
        assert_eq!(Settings::default().acceleration, "auto");
        assert!(Settings::default().validate().is_ok());

        for mode in ACCELERATION_MODES {
            let settings = Settings {
                acceleration: mode.into(),
                ..Settings::default()
            };
            assert!(settings.validate().is_ok(), "modo {mode}");
        }

        let invalid = Settings {
            acceleration: "turbo".into(),
            ..Settings::default()
        };
        assert!(invalid.validate().is_err());
        assert_eq!(invalid.sanitized().acceleration, "auto");
    }

    #[test]
    fn aec_is_enabled_by_default_and_survives_legacy_settings() {
        assert!(Settings::default().aec_enabled);
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json.replace("\"aec_enabled\":true,", "");
        assert!(!legacy.contains("aec_enabled"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert!(parsed.aec_enabled);
    }

    #[test]
    fn legacy_settings_get_acceleration_default() {
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json.replace("\"acceleration\":\"auto\",", "");
        assert!(!legacy.contains("acceleration"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.acceleration, "auto");
    }

    #[test]
    fn legacy_settings_get_panel_size_defaults() {
        let json = serde_json::to_string(&Settings::default()).expect("json");
        let legacy = json
            .replace("\"suggestions_size\":null,", "")
            .replace("\"chat_size\":null,", "")
            .replace("\"history_size\":null,", "")
            .replace("\"chat_position\":null,", "")
            .replace("\"history_position\":null,", "")
            .replace(",\"panel_tab\":\"transcription\"", "");
        assert!(!legacy.contains("chat_size"));
        assert!(!legacy.contains("panel_tab"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.suggestions_size, None);
        assert_eq!(parsed.chat_size, None);
        assert_eq!(parsed.history_size, None);
        assert_eq!(parsed.chat_position, None);
        assert_eq!(parsed.history_position, None);
        // Componente antigo: a aba cai na padrão.
        assert_eq!(parsed.panel_tab, "transcription");
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn validate_rejects_invalid_custom_mode_and_unknown_selection() {
        use crate::domain::assistant::AssistantMode;

        let invalid = Settings {
            custom_assistant_modes: vec![AssistantMode {
                id: "custom-x".into(),
                name: "  ".into(),
                category: String::new(),
                system_prompt: "p".into(),
                suggestion_prompt: String::new(),
                context: String::new(),
                icon: "sparkle".into(),
                color: "slate".into(),
                source_id: None,
            }],
            ..Settings::default()
        };
        assert!(invalid.validate().is_err());

        let unknown = Settings {
            assistant_mode_id: "custom-nao-existe".into(),
            ..Settings::default()
        };
        assert!(unknown.validate().is_err());
    }

    #[test]
    fn sanitized_keeps_valid_custom_mode_selected() {
        use crate::domain::assistant::AssistantMode;

        let settings = Settings {
            assistant_mode_id: "custom-vendas".into(),
            custom_assistant_modes: vec![AssistantMode {
                id: "custom-vendas".into(),
                name: "Vendas".into(),
                category: "custom".into(),
                system_prompt: "Você ajuda em vendas.".into(),
                suggestion_prompt: "Sugira a próxima fala.".into(),
                context: String::new(),
                icon: "sparkle".into(),
                color: "slate".into(),
                source_id: None,
            }],
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(settings.assistant_mode_id, "custom-vendas");
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn sanitized_normalizes_an_unknown_agent_identity() {
        use crate::domain::assistant::AssistantMode;

        let settings = Settings {
            custom_assistant_modes: vec![AssistantMode {
                id: "custom-vendas".into(),
                name: "Vendas".into(),
                category: "custom".into(),
                system_prompt: "Você ajuda em vendas.".into(),
                suggestion_prompt: "Sugira a próxima fala.".into(),
                context: String::new(),
                icon: "rocket".into(),
                color: "#00f2fe".into(),
                source_id: None,
            }],
            ..Settings::default()
        }
        .sanitized();
        // O agente sobrevive: só a identidade volta ao padrão (BR-037).
        assert_eq!(settings.custom_assistant_modes.len(), 1);
        assert_eq!(settings.custom_assistant_modes[0].icon, "sparkle");
        assert_eq!(settings.custom_assistant_modes[0].color, "slate");
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn mic_gain_defaults_to_the_perssua_value() {
        // BR-018: ganho padrão do Perssua (local_whisper_user_mic_gain = 8).
        assert_eq!(Settings::default().mic_gain, 8.0);
    }

    #[test]
    fn sanitized_clamps_mic_gain() {
        let too_loud = Settings {
            mic_gain: 99.0,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(too_loud.mic_gain, crate::domain::audio::MAX_MIC_GAIN);

        let too_quiet = Settings {
            mic_gain: 0.1,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(too_quiet.mic_gain, crate::domain::audio::MIN_MIC_GAIN);

        let broken = Settings {
            mic_gain: f32::NAN,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(broken.mic_gain, crate::domain::audio::DEFAULT_MIC_GAIN);
    }

    #[test]
    fn mic_gain_survives_a_round_trip_and_old_settings_get_the_default() {
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        let parsed: Settings = serde_json::from_str(&json).expect("desserializa");
        assert_eq!(parsed.mic_gain, 8.0);

        // Config antiga (sem o campo) cai no padrão em vez de falhar.
        let legacy = json.replace("\"mic_gain\":8.0,", "");
        assert!(!legacy.contains("mic_gain"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.mic_gain, 8.0);
    }
}
