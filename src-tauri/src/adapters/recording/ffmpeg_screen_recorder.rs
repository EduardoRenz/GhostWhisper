//! Captura de tela/janela em segmentos MKV via `ffmpeg` (BR-067/BR-070).
//!
//! Um processo por segmento: `pause` fecha o arquivo (comando `q`), `resume`
//! abre o próximo e `stop` fecha o último. MKV é válido mesmo truncado — um
//! crash não perde a reunião inteira.
//!
//! **Áudio:** se o build do `ffmpeg` tiver a entrada `pulse`, mic e sistema vão
//! no mesmo processo. Se não tiver (builds estáticos), o áudio é capturado por
//! `parec` em PCM cru (S16LE 48 kHz estéreo) e remuxado na finalização
//! (BR-069). Isso mantém o app funcional com o `ffmpeg` que o usuário tem.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::domain::recording::{RecordingKind, RecordingSpec};
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_recorder::{
    FallbackAudio, RecordingCapabilities, RecordingStart, ScreenRecorder,
};

use super::ffmpeg_args::{
    segment_args, segment_path, SegmentRequest, DEFAULT_MICROPHONE, VAAPI_DEVICE,
};

/// Tempo máximo para o `ffmpeg` fechar o segmento após o `q`.
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

/// Como o áudio é capturado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioMode {
    /// No próprio `ffmpeg` (`-f pulse`).
    Ffmpeg,
    /// Em PCM cru via `parec` (build sem Pulse).
    Parec,
    /// Sem áudio: gravação indisponível (RNF-09).
    None,
}

/// Captura com o binário do `ffmpeg`.
pub struct FfmpegScreenRecorder {
    ffmpeg: PathBuf,
    parec: Option<PathBuf>,
    audio_mode: AudioMode,
    capabilities: RecordingCapabilities,
    inner: Mutex<Inner>,
}

struct Inner {
    child: Option<Child>,
    audio_children: Vec<Child>,
    spec: Option<RecordingSpec>,
    segments_dir: Option<PathBuf>,
    segments: Vec<PathBuf>,
    durations: Vec<i64>,
    segment_index: u32,
    started: Option<Instant>,
    audio: Option<FallbackAudio>,
}

impl Inner {
    fn empty() -> Self {
        Self {
            child: None,
            audio_children: Vec::new(),
            spec: None,
            segments_dir: None,
            segments: Vec::new(),
            durations: Vec::new(),
            segment_index: 0,
            started: None,
            audio: None,
        }
    }
}

impl FfmpegScreenRecorder {
    /// Cria o capturador com o caminho do binário já resolvido.
    pub fn new(ffmpeg: PathBuf) -> Self {
        let parec = find_in_path("parec");
        let audio_mode = if ffmpeg_has_pulse(&ffmpeg) {
            AudioMode::Ffmpeg
        } else if parec.is_some() {
            AudioMode::Parec
        } else {
            AudioMode::None
        };
        let capabilities = detect_capabilities(&ffmpeg, audio_mode);
        Self {
            ffmpeg,
            parec,
            audio_mode,
            capabilities,
            inner: Mutex::new(Inner::empty()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock da gravação envenenado".to_string()))
    }

    /// Abre o próximo segmento (chamado pelo `start` e pelo `resume`).
    fn open_segment(&self, inner: &mut Inner) -> Result<()> {
        let spec = inner
            .spec
            .clone()
            .ok_or_else(|| GhostWhisperError::Other("gravação sem configuração".to_string()))?;
        let dir = inner
            .segments_dir
            .clone()
            .ok_or_else(|| GhostWhisperError::Other("gravação sem pasta".to_string()))?;

        if spec.kind == RecordingKind::Video {
            let output = segment_path(&dir, inner.segment_index, spec.kind);
            let with_audio = self.audio_mode == AudioMode::Ffmpeg;
            let microphone = spec.microphone.then_some(DEFAULT_MICROPHONE);
            let system_device = if spec.system { system_device() } else { None };
            let system = system_device.as_deref();
            if with_audio && microphone.is_none() && system.is_none() {
                return Err(GhostWhisperError::Other(
                    "nenhum canal de áudio para gravar".to_string(),
                ));
            }

            let (width, height) = spec.target.dimensions();
            let args = segment_args(&SegmentRequest {
                spec: &spec,
                output: &output,
                microphone,
                system,
                screen_size: (width > 0 && height > 0).then_some((width, height)),
                with_audio,
            });

            let child = Command::new(&self.ffmpeg)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| {
                    GhostWhisperError::Other(format!("não foi possível iniciar o ffmpeg: {error}"))
                })?;
            inner.child = Some(child);

            tracing::info!(
                path = %output.display(),
                audio = ?self.audio_mode,
                "segmento de gravação aberto"
            );
        }

        if self.audio_mode == AudioMode::Parec {
            if spec.microphone {
                if let Some(path) = inner.audio.as_ref().and_then(|audio| audio.microphone.clone())
                {
                    inner.audio_children.push(self.spawn_parec(None, &path)?);
                }
            }
            if spec.system {
                if let (Some(device), Some(path)) = (
                    system_device(),
                    inner.audio.as_ref().and_then(|audio| audio.system.clone()),
                ) {
                    inner
                        .audio_children
                        .push(self.spawn_parec(Some(&device), &path)?);
                }
            }
        }

        inner.segment_index += 1;
        inner.started = Some(Instant::now());
        Ok(())
    }

    /// Captura o áudio cru via `parec` no arquivo (append).
    fn spawn_parec(&self, device: Option<&str>, path: &Path) -> Result<Child> {
        let parec = self.parec.as_ref().ok_or_else(|| {
            GhostWhisperError::Other("parec não encontrado para capturar o áudio".to_string())
        })?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| {
                GhostWhisperError::Other(format!("não foi possível abrir o áudio cru: {error}"))
            })?;

        let mut command = Command::new(parec);
        command.args([
            "--format=s16le",
            "--rate=48000",
            "--channels=2",
            "--raw",
            "--latency-msec=20",
        ]);
        if let Some(device) = device {
            command.arg(format!("--device={device}"));
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                GhostWhisperError::Other(format!("não foi possível iniciar o parec: {error}"))
            })
    }

    /// Fecha o segmento atual (`q`; `kill` se não sair a tempo).
    fn close_segment(&self, inner: &mut Inner) -> Result<()> {
        for mut child in inner.audio_children.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(mut child) = inner.child.take() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }

            let deadline = Instant::now() + STOP_TIMEOUT;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }

        if let Some(started) = inner.started.take() {
            let duration_ms = started.elapsed().as_millis() as i64;
            inner.durations.push(duration_ms);
            if inner.spec.as_ref().map(|spec| spec.kind) == Some(RecordingKind::Video) {
                if let Some(dir) = &inner.segments_dir {
                    let index = inner.segment_index.saturating_sub(1);
                    inner.segments.push(segment_path(dir, index, RecordingKind::Video));
                }
            }
        }
        Ok(())
    }

    /// Áudio cru existente e não vazio (falha de `parec` vira canal ausente).
    fn current_audio(&self, inner: &Inner) -> Option<FallbackAudio> {
        let audio = inner.audio.as_ref()?;
        let non_empty = |path: &Option<PathBuf>| {
            path.as_ref().filter(|path| {
                std::fs::metadata(path)
                    .map(|meta| meta.len() > 0)
                    .unwrap_or(false)
            })
            .cloned()
        };
        let microphone = non_empty(&audio.microphone);
        let system = non_empty(&audio.system);
        if microphone.is_none() && system.is_none() {
            return None;
        }
        Some(FallbackAudio {
            microphone,
            system,
        })
    }
}

impl ScreenRecorder for FfmpegScreenRecorder {
    fn capabilities(&self) -> RecordingCapabilities {
        self.capabilities.clone()
    }

    fn start(&self, request: &RecordingStart) -> Result<()> {
        let mut inner = self.lock()?;
        if inner.child.is_some() || !inner.audio_children.is_empty() {
            return Err(GhostWhisperError::Other(
                "já existe uma gravação em andamento".to_string(),
            ));
        }
        std::fs::create_dir_all(&request.segments_dir)?;
        inner.spec = Some(request.spec.clone());
        inner.segments_dir = Some(request.segments_dir.clone());
        inner.segments.clear();
        inner.durations.clear();
        inner.segment_index = 0;
        inner.audio = None;

        if self.audio_mode == AudioMode::Parec {
            let dir = &request.segments_dir;
            let microphone = dir.join("audio-mic.s16le");
            let system = dir.join("audio-system.s16le");
            // Começa do zero; o `resume` faz append.
            std::fs::write(&microphone, b"")?;
            std::fs::write(&system, b"")?;
            inner.audio = Some(FallbackAudio {
                microphone: request.spec.microphone.then_some(microphone),
                system: request.spec.system.then_some(system),
            });
        }

        self.open_segment(&mut inner)
    }

    fn pause(&self) -> Result<()> {
        let mut inner = self.lock()?;
        self.close_segment(&mut inner)
    }

    fn resume(&self) -> Result<()> {
        let mut inner = self.lock()?;
        if inner.child.is_some() || !inner.audio_children.is_empty() {
            return Ok(());
        }
        self.open_segment(&mut inner)
    }

    fn stop(&self) -> Result<()> {
        let mut inner = self.lock()?;
        self.close_segment(&mut inner)
    }

    fn segments(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .map(|inner| inner.segments.clone())
            .unwrap_or_default()
    }

    fn segment_durations_ms(&self) -> Vec<i64> {
        self.inner
            .lock()
            .map(|inner| inner.durations.clone())
            .unwrap_or_default()
    }

    fn fallback_audio(&self) -> Option<FallbackAudio> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| self.current_audio(&inner))
    }
}

impl Drop for FfmpegScreenRecorder {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.inner.lock() {
            for mut child in inner.audio_children.drain(..) {
                let _ = child.kill();
                let _ = child.wait();
            }
            if let Some(mut child) = inner.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

/// Dispositivo do áudio do sistema (`<sink>.monitor`).
fn system_device() -> Option<String> {
    crate::adapters::audio::pulse_system_audio_source::default_monitor_name().ok()
}

/// A sessão é Wayland nativo?
///
/// Sob Wayland, o `DISPLAY` do XWayland existe, mas a root do XWayland **não**
/// contém as janelas Wayland nativas: o `x11grab` gravaria um desktop vazio ou
/// incompleto. Melhor recusar com explicação do que entregar um vídeo quebrado.
fn is_wayland(wayland_display: Option<&str>, session_type: Option<&str>) -> bool {
    wayland_display.is_some_and(|value| !value.trim().is_empty())
        || session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
}

fn is_wayland_session() -> bool {
    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    let session = std::env::var("XDG_SESSION_TYPE").ok();
    is_wayland(wayland.as_deref(), session.as_deref())
}

/// Detecta `ffmpeg` + X11 + áudio.
fn detect_capabilities(ffmpeg: &Path, audio: AudioMode) -> RecordingCapabilities {
    if is_wayland_session() {
        return RecordingCapabilities::unavailable(
            "gravação indisponível em Wayland: o XWayland não captura janelas Wayland nativas (use uma sessão X11)",
        );
    }
    if std::env::var_os("DISPLAY").is_none() {
        return RecordingCapabilities::unavailable(
            "gravação disponível apenas em X11/XWayland (BR-007)",
        );
    }
    if !ffmpeg.is_file() {
        return RecordingCapabilities::unavailable(
            "ffmpeg não encontrado — instale o pacote ou defina GHOSTWHISPER_FFMPEG",
        );
    }
    if audio == AudioMode::None {
        return RecordingCapabilities::unavailable(
            "sem captura de áudio: o ffmpeg não tem Pulse e o parec não foi encontrado",
        );
    }
    let encoder = if supports_vaapi(ffmpeg) {
        "vaapi"
    } else {
        "x264"
    };
    RecordingCapabilities::available(encoder)
}

/// O build do `ffmpeg` tem a entrada `pulse`?
fn ffmpeg_has_pulse(ffmpeg: &Path) -> bool {
    if !ffmpeg.is_file() {
        return false;
    }
    Command::new(ffmpeg)
        .args(["-hide_banner", "-devices"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.contains(" pulse"))
        })
        .unwrap_or(false)
}

/// O build do `ffmpeg` tem `h264_vaapi` e o dispositivo existe?
fn supports_vaapi(ffmpeg: &Path) -> bool {
    if !Path::new(VAAPI_DEVICE).exists() {
        return false;
    }
    Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout).contains("h264_vaapi")
                || String::from_utf8_lossy(&output.stderr).contains("h264_vaapi")
        })
        .unwrap_or(false)
}

/// Procura um binário no `PATH`.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Teste manual: grava 2 s, captura o áudio e finaliza num MP4.
    ///
    /// Roda com `cargo test --lib -- --ignored records_and_finalizes`.
    #[test]
    #[ignore = "precisa de X11 + ffmpeg + parec"]
    fn records_and_finalizes_a_real_screen() {
        use crate::adapters::recording::ffmpeg_path::resolve_ffmpeg;
        use crate::adapters::recording::finalizer::FfmpegRecordingFinalizer;
        use crate::domain::recording::{RecordingSpec, RecordingTarget};
        use crate::ports::screen_recorder::{
            FinalizeRequest, RecordingFinalizer, RecordingStart, ScreenRecorder,
        };

        let ffmpeg = resolve_ffmpeg().expect("ffmpeg");
        let recorder = FfmpegScreenRecorder::new(ffmpeg.clone());
        assert!(recorder.capabilities().available);

        let dir = tempfile::tempdir().expect("tempdir");
        let spec = RecordingSpec {
            kind: RecordingKind::Video,
            fps: 8,
            resolution: "native".to_string(),
            quality: "balanced".to_string(),
            encoder: "x264".to_string(),
            audio_kbps: 96,
            container: "mp4".to_string(),
            target: RecordingTarget::Screen {
                label: "Tela inteira".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
            },
            keep_segments: true,
            microphone: true,
            system: true,
            output_dir: dir.path().to_path_buf(),
        };

        recorder
            .start(&RecordingStart {
                spec: spec.clone(),
                segments_dir: dir.path().join("parts"),
            })
            .expect("start");
        std::thread::sleep(Duration::from_secs(2));
        recorder.stop().expect("stop");

        let parts = recorder.segments();
        assert!(!parts.is_empty(), "gerou segmento de vídeo");
        let audio = recorder.fallback_audio();
        assert!(audio.is_some(), "capturou áudio");

        let finalizer = FfmpegRecordingFinalizer::new(ffmpeg);
        let artifact = finalizer
            .finalize(
                &FinalizeRequest {
                    parts,
                    output: dir.path().join("saida.mp4"),
                    spec,
                    total_ms: 2_000,
                    audio,
                },
                &|_| {},
            )
            .expect("finalize");
        assert!(artifact.size_bytes > 0, "arquivo final não vazio");
        assert!(artifact.path.exists());
    }

    #[test]
    fn detects_wayland_sessions() {
        assert!(is_wayland(Some("wayland-0"), None));
        assert!(is_wayland(None, Some("wayland")));
        assert!(!is_wayland(None, Some("x11")));
        assert!(!is_wayland(Some(""), Some("x11")));
        assert!(!is_wayland(None, None));
    }

    #[test]
    fn capabilities_explain_wayland() {
        // Não mexe no ambiente: testa a mensagem de indisponibilidade.
        let capabilities = RecordingCapabilities::unavailable(
            "gravação disponível apenas em X11/XWayland (BR-007)",
        );
        assert!(!capabilities.available);
        assert!(capabilities.reason.expect("motivo").contains("X11"));
    }
}
