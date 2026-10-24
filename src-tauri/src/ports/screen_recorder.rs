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
}

impl RecordingCapabilities {
    /// Indisponível, com o código e o motivo de fallback.
    pub fn unavailable(code: RecordingUnavailableCode, reason: impl Into<String>) -> Self {
        Self {
            available: false,
            encoder: "x264".to_string(),
            reason: Some(reason.into()),
            reason_code: Some(code),
        }
    }

    /// Disponível com o encoder escolhido.
    pub fn available(encoder: impl Into<String>) -> Self {
        Self {
            available: true,
            encoder: encoder.into(),
            reason: None,
            reason_code: None,
        }
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
        assert_eq!(RecordingUnavailableCode::NoDisplay.as_str(), "no_display");
        assert_eq!(RecordingUnavailableCode::NoAudio.as_str(), "no_audio");
    }
}
