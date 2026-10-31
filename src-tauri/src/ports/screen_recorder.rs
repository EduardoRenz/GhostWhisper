//! Portas da gravação: captura (segmentos) e finalização (concat/remux).
//!
//! Adaptadores: `FfmpegScreenRecorder` e `FfmpegRecordingFinalizer`.
//! Dobras de teste: `FakeScreenRecorder` e `FakeRecordingFinalizer`.

use std::path::PathBuf;

use serde::Serialize;

use crate::domain::recording::{RecordingArtifact, RecordingSpec};
use crate::error::Result;

/// Por que o ambiente não grava.
///
/// Código estável (snake_case) que a UI traduz; `reason` fica como fallback e
/// em log. A regra de frontend vem da BR-086 (nada de texto hardcoded na UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingUnavailableCode {
    /// Sessão Wayland: o XWayland não captura janelas nativas (BR-007).
    Wayland,
    /// Sem `DISPLAY` (nem X11 nem XWayland).
    NoDisplay,
    /// `ffmpeg` não está no `PATH`, no sidecar nem em `GHOSTWHISPER_FFMPEG`.
    FfmpegMissing,
    /// Sem `pulse` no `ffmpeg` e sem `parec` (RNF-09).
    NoAudio,
    /// GStreamer/pipewiresrc ausente (caminho de gravação no Wayland).
    #[serde(rename = "gstreamer_missing")]
    GStreamerMissing,
}

impl RecordingUnavailableCode {
    /// Nome estável (o mesmo do `Serialize`) — usado nos testes do contrato.
    #[cfg(test)]
    fn as_str(self) -> &'static str {
        match self {
            RecordingUnavailableCode::Wayland => "wayland",
            RecordingUnavailableCode::NoDisplay => "no_display",
            RecordingUnavailableCode::FfmpegMissing => "ffmpeg_missing",
            RecordingUnavailableCode::NoAudio => "no_audio",
            RecordingUnavailableCode::GStreamerMissing => "gstreamer_missing",
        }
    }
}

/// O ambiente consegue gravar? (gate do botão de câmera)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingCapabilities {
    /// `true` quando há `ffmpeg` e display X11.
    pub available: bool,
    /// Encoder que será usado (`vaapi` ou `x264`).
    pub encoder: String,
    /// Motivo da indisponibilidade (fallback; a UI prefere o código).
    pub reason: Option<String>,
    /// Código do motivo, para a UI traduzir.
    pub reason_code: Option<RecordingUnavailableCode>,
    /// `true` quando o backend consegue seguir/trocar a janela ativa.
    ///
    /// No Wayland a fonte é fixada pelo portal ScreenCast; a UI esconde o
    /// "seguir a janela ativa" quando isto é `false`.
    #[serde(default)]
    pub follow_supported: bool,
    /// Como o alvo é escolhido: `internal` (X11) ou `portal` (Wayland).
    #[serde(default = "default_target_mode")]
    pub target_mode: String,
}

/// Modo de escolha de alvo padrão (X11).
fn default_target_mode() -> String {
    "internal".to_string()
}

impl RecordingCapabilities {
    /// Indisponível, com o código e o motivo de fallback.
    pub fn unavailable(code: RecordingUnavailableCode, reason: impl Into<String>) -> Self {
        Self {
            available: false,
            encoder: "x264".to_string(),
            reason: Some(reason.into()),
            reason_code: Some(code),
            follow_supported: false,
            target_mode: default_target_mode(),
        }
    }

    /// Disponível com o encoder escolhido.
    pub fn available(encoder: impl Into<String>) -> Self {
        Self {
            available: true,
            encoder: encoder.into(),
            reason: None,
            reason_code: None,
            follow_supported: true,
            target_mode: default_target_mode(),
        }
    }

    /// Marca o backend do portal: o alvo é escolhido no GNOME e não segue foco.
    pub fn with_portal_targets(mut self) -> Self {
        self.follow_supported = false;
        self.target_mode = "portal".to_string();
        self
    }
}

/// Áudio cru capturado fora do `ffmpeg` (fallback quando o build não tem Pulse).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackAudio {
    /// S16LE 48 kHz estéreo, concatenado por append entre as pausas.
    pub microphone: Option<PathBuf>,
    /// S16LE 48 kHz estéreo (monitor do sink).
    pub system: Option<PathBuf>,
}

/// O que abrir no início de um segmento.
#[derive(Debug, Clone)]
pub struct RecordingStart {
    pub spec: RecordingSpec,
    /// Pasta onde os segmentos serão criados.
    pub segments_dir: PathBuf,
}

/// Captura contínua em segmentos (um processo `ffmpeg` por segmento).
///
/// `pause` fecha o segmento atual; `resume` abre o próximo; `stop` fecha o
/// último. A finalização é responsabilidade de [`RecordingFinalizer`].
pub trait ScreenRecorder: Send + Sync {
    /// Capacidades do ambiente (encoder, disponibilidade).
    fn capabilities(&self) -> RecordingCapabilities;
    /// Abre o primeiro segmento.
    fn start(&self, request: &RecordingStart) -> Result<()>;
    /// Fecha o segmento atual (mantém a gravação pausada).
    fn pause(&self) -> Result<()>;
    /// Abre o próximo segmento.
    fn resume(&self) -> Result<()>;
    /// Fecha o último segmento.
    fn stop(&self) -> Result<()>;
    /// Troca o alvo no meio da gravação (fecha um segmento e abre outro).
    fn switch_target(&self, target: crate::domain::recording::RecordingTarget) -> Result<()>;
    /// Segmentos fechados até agora, na ordem.
    fn segments(&self) -> Vec<PathBuf>;
    /// Duração de cada segmento (ms), medida no fechamento.
    fn segment_durations_ms(&self) -> Vec<i64>;

    /// Áudio capturado fora do `ffmpeg` (quando o build não tem entrada Pulse).
    fn fallback_audio(&self) -> Option<FallbackAudio>;
}

/// Pedido de finalização (concat + remux).
#[derive(Debug, Clone)]
pub struct FinalizeRequest {
    /// Segmentos fechados, em ordem.
    pub parts: Vec<PathBuf>,
    /// Arquivo final (`.mp4`/`.mkv`).
    pub output: PathBuf,
    /// Configuração congelada (container, dimensões).
    pub spec: RecordingSpec,
    /// Soma das durações dos segmentos (denominador da barra).
    pub total_ms: i64,
    /// Dimensões do **primeiro** segmento (alvo do `scale`+`pad` quando os
    /// trechos têm tamanhos diferentes — troca de janela).
    pub width: u32,
    pub height: u32,
    /// Reencodar (dimensões mudaram entre segmentos)? `-c copy` não serve.
    pub normalize: bool,
    /// Áudio cru do fallback (`parec`), quando usado.
    pub audio: Option<FallbackAudio>,
}

/// Junta os segmentos num arquivo único.
///
/// `-c copy` por padrão; reencoda com `scale`/`pad` quando as dimensões
/// divergem (ISS-053). O progresso é chamado com 0–100.
pub trait RecordingFinalizer: Send + Sync {
    fn finalize(
        &self,
        request: &FinalizeRequest,
        progress: &dyn Fn(u8),
    ) -> Result<RecordingArtifact>;

    /// Apaga os segmentos e o arquivo de controle após o sucesso.
    fn cleanup(&self, parts: &[PathBuf], output: &std::path::Path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_code_serializes_as_snake_case() {
        // Contrato com o frontend (`RecordingUnavailableCode` no TS).
        let json = serde_json::to_string(&RecordingUnavailableCode::FfmpegMissing).expect("json");
        assert_eq!(json, "\"ffmpeg_missing\"");
        assert_eq!(RecordingUnavailableCode::Wayland.as_str(), "wayland");
        assert_eq!(
            RecordingUnavailableCode::GStreamerMissing.as_str(),
            "gstreamer_missing"
        );
        assert_eq!(RecordingUnavailableCode::NoDisplay.as_str(), "no_display");
        assert_eq!(RecordingUnavailableCode::NoAudio.as_str(), "no_audio");
    }

    #[test]
    fn x11_capabilities_support_following_the_active_window() {
        let capabilities = RecordingCapabilities::available("x264");
        assert!(capabilities.follow_supported);
        assert_eq!(capabilities.target_mode, "internal");
    }

    #[test]
    fn portal_capabilities_disable_following_the_active_window() {
        let capabilities = RecordingCapabilities::available("x264").with_portal_targets();
        assert!(!capabilities.follow_supported);
        assert_eq!(capabilities.target_mode, "portal");
        let json = serde_json::to_value(&capabilities).expect("json");
        assert_eq!(json["follow_supported"], false);
        assert_eq!(json["target_mode"], "portal");
    }
}
