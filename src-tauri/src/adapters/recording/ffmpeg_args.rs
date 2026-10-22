//! Builders **puros** dos argumentos do `ffmpeg` (testáveis sem processo).
//!
//! Nenhum teste unitário executa o binário (RNF-10): os testes comparam a lista
//! de argumentos. A captura é feita pelo `FfmpegScreenRecorder`; a finalização,
//! pelo `FfmpegRecordingFinalizer`.

use std::path::{Path, PathBuf};

use crate::domain::recording::{RecordingKind, RecordingSpec};
use crate::ports::screen_recorder::FallbackAudio;

/// Nome do dispositivo de microfone no PulseAudio.
pub const DEFAULT_MICROPHONE: &str = "default";
/// Dispositivo de vídeo do VAAPI (quando usado).
pub const VAAPI_DEVICE: &str = "/dev/dri/renderD128";

/// Entradas de captura de um segmento.
pub struct SegmentRequest<'a> {
    pub spec: &'a RecordingSpec,
    pub output: &'a Path,
    /// `None` desliga o microfone.
    pub microphone: Option<&'a str>,
    /// `None` desliga o áudio do sistema.
    pub system: Option<&'a str>,
    /// Geometria da tela (quando o alvo é `Screen`); ignorada em `Window`.
    pub screen_size: Option<(u32, u32)>,
    /// Captura o áudio no próprio `ffmpeg` (build com Pulse).
    pub with_audio: bool,
}

/// Monta os argumentos de **um segmento**.
pub fn segment_args(request: &SegmentRequest<'_>) -> Vec<String> {
    let spec = request.spec;
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
    ];

    // --- Entradas ---------------------------------------------------------
    if spec.kind == RecordingKind::Video {
        args.extend(["-f".into(), "x11grab".into()]);
        args.extend(["-framerate".into(), spec.fps.to_string()]);
        match spec.target.window_id() {
            Some(id) => args.extend(["-window_id".into(), id.to_string()]),
            None => {
                let (width, height) = spec
                    .forced_size()
                    .or(request.screen_size)
                    .unwrap_or((1920, 1080));
                args.extend(["-video_size".into(), format!("{width}x{height}")]);
            }
        }
        let (x, y) = spec.target.offset();
        let input = if x == 0 && y == 0 {
            ":0.0".to_string()
        } else {
            format!(":0.0+{x},{y}")
        };
        args.extend(["-i".into(), input]);
    }

    let mut audio_index = if spec.kind == RecordingKind::Video {
        1
    } else {
        0
    };
    let mic_index = request
        .microphone
        .filter(|_| request.with_audio)
        .map(|device| {
            let index = audio_index;
            audio_index += 1;
            args.extend(["-f".into(), "pulse".into(), "-i".into(), device.to_string()]);
            index
        });
    let system_index = request.system.filter(|_| request.with_audio).map(|device| {
        let index = audio_index;
        args.extend(["-f".into(), "pulse".into(), "-i".into(), device.to_string()]);
        index
    });

    // --- Filtros ----------------------------------------------------------
    let vaapi = spec.encoder == "vaapi";
    let mut filters: Vec<String> = Vec::new();
    let mut video_label = "0:v".to_string();
    if spec.kind == RecordingKind::Video {
        let mut video_chain: Vec<String> = Vec::new();
        if let Some((width, height)) = spec.forced_size() {
            video_chain.push(format!("scale={width}:{height}"));
        }
        if vaapi {
            video_chain.push("format=nv12".to_string());
            video_chain.push("hwupload".to_string());
        }
        if !video_chain.is_empty() {
            filters.push(format!("[0:v]{}[v]", video_chain.join(",")));
            video_label = "[v]".to_string();
        }
    }

    if request.with_audio {
        if let Some(audio) = audio_filter(mic_index, system_index) {
            filters.push(audio);
        }
    }

    if !filters.is_empty() {
        args.extend(["-filter_complex".into(), filters.join(";")]);
    }

    // --- Mapas e codecs ---------------------------------------------------
    if spec.kind == RecordingKind::Video {
        args.extend(["-map".into(), video_label]);
    }
    if request.with_audio {
        args.extend(["-map".into(), "[a]".into()]);
    }

    if spec.kind == RecordingKind::Video {
        if vaapi {
            args.extend([
                "-vaapi_device".into(),
                VAAPI_DEVICE.into(),
                "-c:v".into(),
                "h264_vaapi".into(),
                "-qp".into(),
                spec.crf().to_string(),
            ]);
        } else {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                spec.crf().to_string(),
                "-g".into(),
                spec.gop().to_string(),
                "-threads".into(),
                "2".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ]);
        }
    }

    match spec.kind {
        RecordingKind::Video => {
            if request.with_audio {
                args.extend([
                    "-c:a".into(),
                    "aac".into(),
                    "-b:a".into(),
                    format!("{}k", spec.audio_kbps),
                    "-ar".into(),
                    "48000".into(),
                ]);
            } else {
                args.extend(["-an".into()]);
            }
            args.extend(["-f".into(), "matroska".into()]);
        }
        RecordingKind::Audio => {
            args.extend(["-c:a".into(), "flac".into(), "-ar".into(), "48000".into()]);
            args.extend(["-f".into(), "flac".into()]);
        }
    }

    args.push(request.output.to_string_lossy().into_owned());
    args
}

/// Grafo de áudio: mixa microfone e sistema quando ambos existem.
fn audio_filter(mic: Option<usize>, system: Option<usize>) -> Option<String> {
    match (mic, system) {
        (Some(mic), Some(system)) => Some(format!(
            "[{mic}:a]aresample=48000,aformat=channel_layouts=mono[m];\
             [{system}:a]aresample=48000,aformat=channel_layouts=stereo[s];\
             [m][s]amix=inputs=2:normalize=0:duration=longest,alimiter=limit=0.95,             aformat=channel_layouts=stereo[a]"
        )),
        (Some(index), None) | (None, Some(index)) => Some(format!(
            "[{index}:a]aresample=48000,aformat=channel_layouts=stereo[a]"
        )),
        (None, None) => None,
    }
}

/// Argumentos do `concat` para o arquivo final.
pub fn concat_args(request: &super::finalizer::ConcatRequest) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        request.parts_file.to_string_lossy().into_owned(),
        "-c".into(),
        "copy".into(),
    ];
    if request.spec.kind == RecordingKind::Video && request.spec.container != "mkv" {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
    args.push(request.output.to_string_lossy().into_owned());
    args
}

/// Argumentos do reencode de emergência (dimensões diferentes — ISS-053).
pub fn normalize_args(
    request: &super::finalizer::ConcatRequest,
    width: u32,
    height: u32,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        request.parts_file.to_string_lossy().into_owned(),
        "-vf".into(),
        format!(
            "scale={width}:{height}:force_original_aspect_ratio=decrease,\
             pad={width}:{height}:(ow-iw)/2:(oh-ih)/2"
        ),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "27".into(),
        "-threads".into(),
        "2".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "96k".into(),
    ];
    if request.spec.container != "mkv" {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
    args.push(request.output.to_string_lossy().into_owned());
    args
}

/// Entradas de áudio cru do fallback (`parec`), com os índices do `ffmpeg`.
fn fallback_audio_inputs(
    args: &mut Vec<String>,
    audio: &FallbackAudio,
) -> (Option<usize>, Option<usize>) {
    let mut index = 1;
    let mut push = |args: &mut Vec<String>, path: &std::path::Path| {
        let current = index;
        index += 1;
        args.extend([
            "-f".into(),
            "s16le".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            "-i".into(),
            path.to_string_lossy().into_owned(),
        ]);
        current
    };
    let microphone = audio.microphone.as_deref().map(|path| push(args, path));
    let system = audio.system.as_deref().map(|path| push(args, path));
    (microphone, system)
}

/// Remuxa vídeo concatenado + áudio cru (`parec`) num MP4/MKV.
pub fn fallback_mux_args(
    request: &super::finalizer::ConcatRequest,
    audio: &FallbackAudio,
    output: &Path,
    normalize: Option<(u32, u32)>,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        request.parts_file.to_string_lossy().into_owned(),
    ];
    let (mic, system) = fallback_audio_inputs(&mut args, audio);
    let graph = audio_filter(mic, system).unwrap_or_default();
    args.extend([
        "-filter_complex".into(),
        graph,
        "-map".into(),
        "0:v".into(),
        "-map".into(),
        "[a]".into(),
    ]);
    match normalize {
        Some((width, height)) => args.extend([
            "-vf".into(),
            format!(
                "scale={width}:{height}:force_original_aspect_ratio=decrease,\
                 pad={width}:{height}:(ow-iw)/2:(oh-ih)/2"
            ),
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-crf".into(),
            "27".into(),
            "-threads".into(),
            "2".into(),
        ]),
        None => args.extend(["-c:v".into(), "copy".into()]),
    }
    args.extend([
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        format!("{}k", request.spec.audio_kbps),
        "-ar".into(),
        "48000".into(),
    ]);
    if request.spec.container != "mkv" {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
    args.push(output.to_string_lossy().into_owned());
    args
}

/// Finaliza o modo "somente áudio" com `parec` (FLAC sem perda).
pub fn fallback_audio_args(
    _request: &super::finalizer::ConcatRequest,
    audio: &FallbackAudio,
    output: &Path,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
    ];
    // O primeiro input é o áudio; o índice começa em 0.
    let mut index = 0;
    let mut push = |args: &mut Vec<String>, path: &std::path::Path| {
        let current = index;
        index += 1;
        args.extend([
            "-f".into(),
            "s16le".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            "-i".into(),
            path.to_string_lossy().into_owned(),
        ]);
        current
    };
    let microphone = audio
        .microphone
        .as_deref()
        .map(|path| push(&mut args, path));
    let system = audio.system.as_deref().map(|path| push(&mut args, path));
    let graph = audio_filter(microphone, system).unwrap_or_default();
    args.extend([
        "-filter_complex".into(),
        graph,
        "-map".into(),
        "[a]".into(),
        "-c:a".into(),
        "flac".into(),
        "-ar".into(),
        "48000".into(),
        "-f".into(),
        "flac".into(),
    ]);
    args.push(output.to_string_lossy().into_owned());
    args
}

/// Escapa um caminho no arquivo `parts.txt` do concat.
pub fn concat_line(path: &Path) -> String {
    format!("file '{}'\n", path.to_string_lossy().replace('\'', "'\\''"))
}

/// Nome do segmento `part-NNNN.mkv`.
pub fn segment_path(dir: &Path, index: u32, kind: RecordingKind) -> PathBuf {
    let extension = match kind {
        RecordingKind::Video => "mkv",
        RecordingKind::Audio => "flac",
    };
    dir.join(format!(
        "{}{index:04}.{extension}",
        crate::domain::recording::SEGMENT_PREFIX
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recording::{RecordingSpec, RecordingTarget};

    fn spec() -> RecordingSpec {
        RecordingSpec {
            kind: RecordingKind::Video,
            fps: 8,
            resolution: "native".to_string(),
            quality: "balanced".to_string(),
            encoder: "x264".to_string(),
            audio_kbps: 96,
            container: "mp4".to_string(),
            target: RecordingTarget::Window {
                id: 12345,
                label: "Chrome".to_string(),
                width: 1920,
                height: 1080,
            },
            keep_segments: false,
            microphone: true,
            system: true,
            output_dir: PathBuf::from("/tmp/rec"),
        }
    }

    fn args_for(spec: &RecordingSpec, mic: bool, system: bool) -> Vec<String> {
        let output = PathBuf::from("/tmp/rec/part-0001.mkv");
        segment_args(&SegmentRequest {
            spec,
            output: &output,
            microphone: mic.then_some("default"),
            system: system.then_some("alsa_output.pci.monitor"),
            screen_size: Some((1920, 1080)),
            with_audio: true,
        })
    }

    #[test]
    fn video_window_uses_window_id_not_video_size() {
        let args = args_for(&spec(), true, true);
        let joined = args.join(" ");
        assert!(joined.contains("-window_id 12345"), "{joined}");
        assert!(!joined.contains("-video_size"), "{joined}");
        assert!(joined.contains("-i :0.0"), "{joined}");
    }

    #[test]
    fn video_screen_uses_video_size() {
        let mut spec = spec();
        spec.target = RecordingTarget::Screen {
            label: "Tela".into(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
        };
        let args = args_for(&spec, true, true);
        assert!(args.join(" ").contains("-video_size 1920x1080"));
    }

    #[test]
    fn a_monitor_target_uses_its_offset() {
        let mut spec = spec();
        spec.target = RecordingTarget::Screen {
            label: "HDMI-1 — 1920×1080".into(),
            width: 1920,
            height: 1080,
            x: 1920,
            y: 0,
        };
        let args = args_for(&spec, true, true);
        assert!(
            args.join(" ").contains("-i :0.0+1920,0"),
            "{}",
            args.join(" ")
        );
    }

    #[test]
    fn mixes_microphone_and_system_audio() {
        let args = args_for(&spec(), true, true);
        let joined = args.join(" ");
        assert!(joined.contains("amix=inputs=2"), "{joined}");
        assert!(joined.contains("[1:a]"), "{joined}");
        assert!(joined.contains("[2:a]"), "{joined}");
        assert!(joined.contains("-c:a aac -b:a 96k"), "{joined}");
        assert!(joined.contains("-f matroska"), "{joined}");
    }

    #[test]
    fn single_audio_source_skips_amix() {
        let args = args_for(&spec(), true, false);
        let joined = args.join(" ");
        assert!(!joined.contains("amix"), "{joined}");
        assert!(joined.contains("[1:a]"), "{joined}");
    }

    #[test]
    fn audio_only_starts_at_index_zero_and_uses_flac() {
        let mut spec = spec();
        spec.kind = RecordingKind::Audio;
        spec.target = RecordingTarget::Screen {
            label: "x".into(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
        };
        let args = args_for(&spec, true, true);
        let joined = args.join(" ");
        assert!(!joined.contains("x11grab"), "{joined}");
        assert!(joined.contains("[0:a]"), "{joined}");
        assert!(joined.contains("-c:a flac"), "{joined}");
        assert!(joined.contains("-f flac"), "{joined}");
        assert!(!joined.contains("-map 0:v"), "{joined}");
    }

    #[test]
    fn x264_limits_threads_and_sets_crf() {
        let args = args_for(&spec(), true, true);
        let joined = args.join(" ");
        assert!(joined.contains("-crf 27"), "{joined}");
        assert!(joined.contains("-threads 2"), "{joined}");
        assert!(joined.contains("-g 80"), "{joined}");
        assert!(joined.contains("-preset veryfast"), "{joined}");
    }

    #[test]
    fn vaapi_uses_hardware_encoder_and_upload_filter() {
        let mut spec = spec();
        spec.encoder = "vaapi".into();
        let args = args_for(&spec, true, true);
        let joined = args.join(" ");
        assert!(joined.contains("h264_vaapi"), "{joined}");
        assert!(joined.contains("hwupload"), "{joined}");
        assert!(!joined.contains("libx264"), "{joined}");
    }

    #[test]
    fn forced_resolution_adds_scale() {
        let mut spec = spec();
        spec.resolution = "1280x720".into();
        let args = args_for(&spec, true, true);
        assert!(args.join(" ").contains("[0:v]scale=1280:720[v]"));
    }

    #[test]
    fn segment_path_is_zero_padded() {
        let path = segment_path(Path::new("/tmp/rec"), 7, RecordingKind::Video);
        assert_eq!(path, PathBuf::from("/tmp/rec/part-0007.mkv"));
        let audio = segment_path(Path::new("/tmp/rec"), 2, RecordingKind::Audio);
        assert_eq!(audio, PathBuf::from("/tmp/rec/part-0002.flac"));
    }
}
