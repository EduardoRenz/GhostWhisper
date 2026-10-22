//! Tipos de domínio da gravação da sessão (BR-065…BR-071).
//!
//! A gravação é um **artefato da sessão**: nasce com ela, pausa/retoma em
//! segmentos e termina no `stop`. O domínio não conhece `ffmpeg`, `mkv` nem
//! `x11grab` — só o artefato, o alvo e o estado.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// FPS padrão da gravação (BR-066: custo é por frame, não por resolução).
pub const DEFAULT_RECORDING_FPS: u32 = 8;
/// Degraus de FPS oferecidos na UI.
pub const RECORDING_FPS_STEPS: [u32; 5] = [5, 8, 10, 15, 30];

/// Modo de gravação padrão.
pub const DEFAULT_RECORDING_MODE: &str = "video";

/// Alvo padrão (BR-070: o overlay fica fora do vídeo).
pub const DEFAULT_RECORDING_TARGET: &str = "window";
/// Alvos aceitos.
pub const RECORDING_TARGETS: [&str; 2] = ["window", "screen"];

/// Resolução padrão (nativa — reduzir não economiza e piora a leitura).
pub const DEFAULT_RECORDING_RESOLUTION: &str = "native";
/// Resoluções aceitas (`native` = geometria da janela/tela).
pub const RECORDING_RESOLUTIONS: [&str; 3] = ["native", "1280x720", "854x480"];

/// Qualidade padrão.
pub const DEFAULT_RECORDING_QUALITY: &str = "balanced";
/// Qualidades aceitas (viram CRF 30/27/24).
pub const RECORDING_QUALITIES: [&str; 3] = ["compact", "balanced", "sharp"];

/// Encoder padrão.
pub const DEFAULT_RECORDING_ENCODER: &str = "auto";
/// Encoders aceitos.
pub const RECORDING_ENCODERS: [&str; 3] = ["auto", "x264", "vaapi"];

/// Bitrate de áudio padrão (kbps).
pub const DEFAULT_RECORDING_AUDIO_KBPS: u32 = 96;
/// Menor bitrate aceito.
pub const MIN_RECORDING_AUDIO_KBPS: u32 = 64;
/// Maior bitrate aceito.
pub const MAX_RECORDING_AUDIO_KBPS: u32 = 192;

/// Container final padrão.
pub const DEFAULT_RECORDING_CONTAINER: &str = "mp4";
/// Containers aceitos.
pub const RECORDING_CONTAINERS: [&str; 2] = ["mp4", "mkv"];

/// Prefixo dos segmentos em disco.
pub const SEGMENT_PREFIX: &str = "part-";

/// Tipo do artefato.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingKind {
    /// Vídeo + áudio (H.264/AAC).
    Video,
    /// Somente áudio (FLAC sem perda).
    Audio,
}

impl RecordingKind {
    /// Representação persistida/exibida.
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordingKind::Video => "video",
            RecordingKind::Audio => "audio",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "video" => Some(RecordingKind::Video),
            "audio" => Some(RecordingKind::Audio),
            _ => None,
        }
    }

    /// Converte a configuração (`video`/`audio`) no tipo do artefato.
    pub fn parse(value: &str) -> Self {
        Self::from_str(value.trim()).unwrap_or(RecordingKind::Video)
    }
}

/// Estado do artefato (BR-067).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingStatus {
    /// Capturando o segmento atual.
    Recording,
    /// Pausada; o segmento atual foi fechado.
    Paused,
    /// Concatenando/remuxando os segmentos.
    Finalizing,
    /// Arquivo final pronto.
    Ready,
    /// Falhou (segmentos preservados).
    Failed,
    /// Descartada pelo usuário (sessão e transcrição permanecem).
    Discarded,
    /// App fechou no meio; segmentos preservados para recuperação.
    Interrupted,
}

impl RecordingStatus {
    /// Representação persistida.
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordingStatus::Recording => "recording",
            RecordingStatus::Paused => "paused",
            RecordingStatus::Finalizing => "finalizing",
            RecordingStatus::Ready => "ready",
            RecordingStatus::Failed => "failed",
            RecordingStatus::Discarded => "discarded",
            RecordingStatus::Interrupted => "interrupted",
        }
    }

    /// Converte do valor persistido.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "recording" => Some(RecordingStatus::Recording),
            "paused" => Some(RecordingStatus::Paused),
            "finalizing" => Some(RecordingStatus::Finalizing),
            "ready" => Some(RecordingStatus::Ready),
            "failed" => Some(RecordingStatus::Failed),
            "discarded" => Some(RecordingStatus::Discarded),
            "interrupted" => Some(RecordingStatus::Interrupted),
            _ => None,
        }
    }
}

/// Uma gravação persistida (uma por sessão).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recording {
    pub id: String,
    pub session_id: String,
    pub kind: RecordingKind,
    pub status: RecordingStatus,
    /// Arquivo final (ausente enquanto não finaliza).
    pub path: Option<String>,
    /// Pasta dos segmentos (recuperação após crash).
    pub segments_dir: Option<String>,
    pub size_bytes: u64,
    pub duration_ms: i64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub error: Option<String>,
}

/// Alvo de captura (BR-070).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RecordingTarget {
    /// Uma janela X11 (o overlay não entra no vídeo).
    Window {
        id: u32,
        label: String,
        width: u32,
        height: u32,
    },
    /// A tela inteira ou um monitor específico (inclui o overlay).
    Screen {
        label: String,
        width: u32,
        height: u32,
        x: i16,
        y: i16,
    },
}

impl RecordingTarget {
    /// Rótulo legível (mostrado no tooltip/modal).
    pub fn label(&self) -> &str {
        match self {
            RecordingTarget::Window { label, .. } => label,
            RecordingTarget::Screen { label, .. } => label,
        }
    }

    /// Id da janela, quando o alvo é uma janela.
    pub fn window_id(&self) -> Option<u32> {
        match self {
            RecordingTarget::Window { id, .. } => Some(*id),
            RecordingTarget::Screen { .. } => None,
        }
    }

    /// Origem da captura na tela (`0,0` para janelas).
    pub fn offset(&self) -> (i16, i16) {
        match self {
            RecordingTarget::Screen { x, y, .. } => (*x, *y),
            RecordingTarget::Window { .. } => (0, 0),
        }
    }

    /// Geometria conhecida do alvo (`0×0` quando o X11 não informou).
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            RecordingTarget::Window { width, height, .. }
            | RecordingTarget::Screen { width, height, .. } => (*width, *height),
        }
    }
}

/// Configuração **congelada** de uma gravação (BR-067: os segmentos precisam
/// ser concatenáveis; mudar settings no meio não vale para a sessão corrente).
#[derive(Debug, Clone, PartialEq)]
pub struct RecordingSpec {
    pub kind: RecordingKind,
    pub fps: u32,
    /// `native` ou `LARGURAxALTURA`.
    pub resolution: String,
    pub quality: String,
    pub encoder: String,
    pub audio_kbps: u32,
    pub container: String,
    pub target: RecordingTarget,
    pub keep_segments: bool,
    /// Capturar o microfone?
    pub microphone: bool,
    /// Capturar o áudio do sistema?
    pub system: bool,
    /// Pasta onde ficam os segmentos e o arquivo final.
    pub output_dir: PathBuf,
}

impl RecordingSpec {
    /// CRF do x264 conforme a qualidade (BR-066).
    pub fn crf(&self) -> u32 {
        match self.quality.as_str() {
            "compact" => 30,
            "sharp" => 24,
            _ => 27,
        }
    }

    /// Intervalo de keyframes (10 s) — facilita o *scrubbing*.
    pub fn gop(&self) -> u32 {
        self.fps.saturating_mul(10).max(1)
    }

    /// Largura/altura forçadas (quando a resolução não é nativa).
    pub fn forced_size(&self) -> Option<(u32, u32)> {
        match self.resolution.as_str() {
            "1280x720" => Some((1280, 720)),
            "854x480" => Some((854, 480)),
            _ => None,
        }
    }

    /// Extensão do arquivo final (áudio é sempre FLAC).
    pub fn extension(&self) -> &'static str {
        if self.kind == RecordingKind::Audio {
            return "flac";
        }
        if self.container == "mkv" {
            "mkv"
        } else {
            "mp4"
        }
    }
}

/// Estado exposto à UI (evento `recording_state`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingState {
    /// `idle` | `recording` | `paused` | `finalizing`.
    pub status: String,
    pub kind: Option<String>,
    pub session_id: Option<String>,
    pub target_label: Option<String>,
    pub elapsed_ms: i64,
    pub size_bytes: u64,
}

impl RecordingState {
    /// Estado ocioso (sem gravação).
    pub fn idle() -> Self {
        Self {
            status: "idle".to_string(),
            kind: None,
            session_id: None,
            target_label: None,
            elapsed_ms: 0,
            size_bytes: 0,
        }
    }
}

/// Artefato produzido pela finalização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingArtifact {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub duration_ms: i64,
    pub width: u32,
    pub height: u32,
}

/// Normaliza o FPS para o degrau mais próximo aceito.
pub fn clamp_fps(fps: u32) -> u32 {
    RECORDING_FPS_STEPS
        .iter()
        .copied()
        .min_by_key(|step| step.abs_diff(fps))
        .unwrap_or(DEFAULT_RECORDING_FPS)
}

/// Normaliza o bitrate de áudio para a faixa aceita.
pub fn clamp_audio_kbps(kbps: u32) -> u32 {
    kbps.clamp(MIN_RECORDING_AUDIO_KBPS, MAX_RECORDING_AUDIO_KBPS)
}

/// Mantém o valor quando está na lista; caso contrário, devolve o padrão.
pub fn pick_or_default(value: &str, allowed: &[&str], default: &str) -> String {
    let trimmed = value.trim();
    if allowed.contains(&trimmed) {
        trimmed.to_string()
    } else {
        default.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> RecordingSpec {
        RecordingSpec {
            kind: RecordingKind::Video,
            fps: 8,
            resolution: "native".to_string(),
            quality: "balanced".to_string(),
            encoder: "auto".to_string(),
            audio_kbps: 96,
            container: "mp4".to_string(),
            target: RecordingTarget::Screen {
                label: "Tela inteira".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
            },
            keep_segments: false,
            microphone: true,
            system: true,
            output_dir: PathBuf::from("/tmp"),
        }
    }

    #[test]
    fn status_round_trips() {
        for status in [
            RecordingStatus::Recording,
            RecordingStatus::Paused,
            RecordingStatus::Finalizing,
            RecordingStatus::Ready,
            RecordingStatus::Failed,
            RecordingStatus::Discarded,
            RecordingStatus::Interrupted,
        ] {
            assert_eq!(RecordingStatus::from_str(status.as_str()), Some(status));
        }
        assert_eq!(RecordingStatus::from_str("nope"), None);
    }

    #[test]
    fn kind_round_trips() {
        assert_eq!(RecordingKind::from_str("video"), Some(RecordingKind::Video));
        assert_eq!(RecordingKind::from_str("audio"), Some(RecordingKind::Audio));
        assert_eq!(RecordingKind::parse("garbage"), RecordingKind::Video);
    }

    #[test]
    fn quality_maps_to_crf() {
        let mut spec = spec();
        assert_eq!(spec.crf(), 27);
        spec.quality = "compact".into();
        assert_eq!(spec.crf(), 30);
        spec.quality = "sharp".into();
        assert_eq!(spec.crf(), 24);
        spec.quality = "unknown".into();
        assert_eq!(spec.crf(), 27, "desconhecida cai no equilibrado");
    }

    #[test]
    fn gop_is_ten_seconds() {
        let mut spec = spec();
        assert_eq!(spec.gop(), 80);
        spec.fps = 30;
        assert_eq!(spec.gop(), 300);
    }

    #[test]
    fn resolution_is_optional() {
        let mut spec = spec();
        assert_eq!(spec.forced_size(), None);
        spec.resolution = "1280x720".into();
        assert_eq!(spec.forced_size(), Some((1280, 720)));
        spec.resolution = "854x480".into();
        assert_eq!(spec.forced_size(), Some((854, 480)));
    }

    #[test]
    fn fps_clamps_to_the_nearest_step() {
        assert_eq!(clamp_fps(1), 5);
        assert_eq!(clamp_fps(7), 8);
        assert_eq!(clamp_fps(12), 10);
        assert_eq!(clamp_fps(16), 15);
        assert_eq!(clamp_fps(99), 30);
    }

    #[test]
    fn audio_bitrate_is_bounded_even_on_garbage() {
        assert_eq!(clamp_audio_kbps(0), MIN_RECORDING_AUDIO_KBPS);
        assert_eq!(clamp_audio_kbps(999), MAX_RECORDING_AUDIO_KBPS);
        assert_eq!(clamp_audio_kbps(128), 128);
    }

    #[test]
    fn target_round_trips_labels_and_window_id() {
        let window = RecordingTarget::Window {
            id: 42,
            label: "Chrome".to_string(),
            width: 1920,
            height: 1080,
        };
        assert_eq!(window.window_id(), Some(42));
        assert_eq!(window.label(), "Chrome");

        let screen = RecordingTarget::Screen {
            label: "Tela inteira".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
        };
        assert_eq!(screen.window_id(), None);
        assert_eq!(screen.label(), "Tela inteira");
        assert_eq!(window.dimensions(), (1920, 1080));
    }

    #[test]
    fn container_picks_the_extension() {
        let mut spec = spec();
        assert_eq!(spec.extension(), "mp4");
        spec.container = "mkv".into();
        assert_eq!(spec.extension(), "mkv");
    }

    #[test]
    fn idle_state_has_no_session_or_media() {
        let state = RecordingState::idle();
        assert_eq!(state.status, "idle");
        assert!(state.session_id.is_none());
        assert_eq!(state.elapsed_ms, 0);
    }
}
