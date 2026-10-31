//! Builders **puros** do pipeline GStreamer para o Wayland (ScreenCast).
//!
//! O portal entrega um nó PipeWire e um `fd` remoto; o `pipewiresrc` consome
//! esse nó. Os argumentos são testáveis sem abrir o portal nem o GStreamer.
//!
//! O pipeline é declarado com o muxer/mixer **antes** das ramificações que os
//! referenciam (`mux.`/`amix.`) — é a forma canônica do `gst-launch` e evita
//! depender de resolução de nomes fora de ordem.

use std::path::Path;

use crate::domain::recording::{RecordingKind, RecordingSpec};

/// Nó/fd devolvidos pelo portal ScreenCast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalStream {
    /// fd do PipeWire remoto (herdado pelo `gst-launch`).
    pub fd: i32,
    /// Node id do stream.
    pub node: u32,
}

/// Entradas de um segmento GStreamer.
pub struct GstSegmentRequest<'a> {
    pub spec: &'a RecordingSpec,
    pub output: &'a Path,
    /// `None` em gravação só de áudio (o portal não é aberto).
    pub stream: Option<PortalStream>,
    /// Dispositivo Pulse do microfone (`None` desliga).
    pub microphone: Option<&'a str>,
    /// Dispositivo Pulse do monitor do sistema (`None` desliga).
    pub system: Option<&'a str>,
}

/// Caps de áudio comuns a todas as fontes (mixer e encoder).
const AUDIO_CAPS: &str = "audio/x-raw,rate=48000,channels=2";

/// Monta os argumentos do `gst-launch-1.0` para um segmento.
///
/// Devolve vazio quando falta o essencial (vídeo sem stream do portal ou áudio
/// sem nenhuma fonte). O chamador transforma isso em erro antes de gravar.
pub fn segment_args(request: &GstSegmentRequest<'_>) -> Vec<String> {
    let spec = request.spec;
    let video = spec.kind == RecordingKind::Video;
    let microphone = request.microphone;
    let system = request.system;
    let has_audio = microphone.is_some() || system.is_some();

    if video && request.stream.is_none() {
        return Vec::new();
    }
    if !video && !has_audio {
        return Vec::new();
    }

    let mut args = vec!["-q".into(), "-e".into()];

    if video {
        // O muxer vem primeiro; as ramificações de vídeo/áudio só o referenciam.
        args.extend([
            "matroskamux".into(),
            "name=mux".into(),
            "!".into(),
            "filesink".into(),
            format!("location={}", request.output.display()),
            "pipewiresrc".into(),
        ]);
        let stream = request.stream.expect("vídeo exige stream do portal");
        args.extend([
            format!("fd={}", stream.fd),
            format!("path={}", stream.node),
            "!".into(),
            "queue".into(),
            "!".into(),
            "videoconvert".into(),
        ]);
        if spec.forced_size().is_some() {
            args.extend(["!".into(), "videoscale".into()]);
        }
        if spec.fps > 0 {
            args.extend(["!".into(), "videorate".into()]);
        }
        let mut caps = vec!["video/x-raw".to_string(), "format=I420".to_string()];
        if let Some((width, height)) = spec.forced_size() {
            caps.push(format!("width={width}"));
            caps.push(format!("height={height}"));
        }
        if spec.fps > 0 {
            caps.push(format!("framerate={}/1", spec.fps));
        }
        args.extend([
            "!".into(),
            caps.join(","),
            "!".into(),
            "x264enc".into(),
            "tune=zerolatency".into(),
            "speed-preset=veryfast".into(),
            // Sem `pass=qual` o x264enc fica em CBR (bitrate 2048) e ignora
            // o `quantizer`; a qualidade configurada (CRF) não valeria.
            "pass=qual".into(),
            format!("quantizer={}", spec.crf()),
            format!("key-int-max={}", spec.gop()),
            "threads=2".into(),
            "!".into(),
            "h264parse".into(),
            "!".into(),
            "mux.".into(),
        ]);
    }

    if has_audio {
        let mixed = microphone.is_some() && system.is_some();
        if mixed {
            args.extend(mixer_chain());
            args.extend(encoder_tail(spec, request.output));
        }
        for device in [microphone, system].into_iter().flatten() {
            args.extend(source_chain(device));
            if mixed {
                args.push("amix.".into());
            } else {
                args.extend(encoder_tail(spec, request.output));
            }
        }
    }

    args
}

/// Ramo de uma fonte Pulse até os caps comuns.
fn source_chain(device: &str) -> Vec<String> {
    vec![
        "pulsesrc".to_string(),
        format!("device={device}"),
        "!".into(),
        "audioconvert".into(),
        "!".into(),
        "audioresample".into(),
        "!".into(),
        AUDIO_CAPS.into(),
        "!".into(),
    ]
}

/// Mixer com conversão/resample para os caps comuns (quando há duas fontes).
fn mixer_chain() -> Vec<String> {
    vec![
        "audiomixer".into(),
        "name=amix".into(),
        "!".into(),
        "audioconvert".into(),
        "!".into(),
        "audioresample".into(),
        "!".into(),
        AUDIO_CAPS.into(),
        "!".into(),
    ]
}

/// Codificador de áudio até o destino (muxer do vídeo ou arquivo FLAC).
fn encoder_tail(spec: &RecordingSpec, output: &Path) -> Vec<String> {
    match spec.kind {
        RecordingKind::Video => vec![
            "avenc_aac".into(),
            format!("bitrate={}", spec.audio_kbps.saturating_mul(1000)),
            "!".into(),
            "aacparse".into(),
            "!".into(),
            "mux.".into(),
        ],
        RecordingKind::Audio => vec![
            "flacenc".into(),
            "!".into(),
            "flacparse".into(),
            "!".into(),
            "filesink".into(),
            format!("location={}", output.display()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recording::RecordingTarget;
    use std::path::PathBuf;

    fn spec(kind: RecordingKind, resolution: &str) -> RecordingSpec {
        RecordingSpec {
            kind,
            fps: 8,
            resolution: resolution.to_string(),
            quality: "balanced".to_string(),
            encoder: "x264".to_string(),
            audio_kbps: 96,
            container: "mp4".to_string(),
            target: RecordingTarget::Screen {
                label: "Tela (portal)".to_string(),
                width: 0,
                height: 0,
                x: 0,
                y: 0,
            },
            keep_segments: false,
            microphone: true,
            system: true,
            output_dir: PathBuf::from("/tmp"),
        }
    }

    fn args(spec: &RecordingSpec, mic: Option<&str>, system: Option<&str>) -> Vec<String> {
        let output = PathBuf::from("/tmp/part-0000.mkv");
        segment_args(&GstSegmentRequest {
            spec,
            output: &output,
            stream: Some(PortalStream { fd: 12, node: 82 }),
            microphone: mic,
            system,
        })
    }

    #[test]
    fn builds_video_with_both_audio_sources() {
        let spec = spec(RecordingKind::Video, "native");
        let args = args(&spec, Some("default"), Some("alsa_output.monitor"));
        assert!(args.contains(&"pipewiresrc".to_string()));
        assert!(args.contains(&"fd=12".to_string()));
        assert!(args.contains(&"path=82".to_string()));
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "pulsesrc").count(),
            2
        );
        assert!(args.contains(&"audiomixer".to_string()));
        assert!(args.contains(&"avenc_aac".to_string()));
        assert!(args.contains(&"matroskamux".to_string()));
        assert!(args.contains(&"x264enc".to_string()));
        assert!(args.contains(&"pass=qual".to_string()));
        assert!(args.contains(&"quantizer=27".to_string()));
        // O muxer precisa existir antes do primeiro `mux.`.
        let muxer = args
            .iter()
            .position(|arg| arg == "name=mux")
            .expect("muxer");
        let first_ref = args.iter().position(|arg| arg == "mux.").expect("mux.");
        assert!(muxer < first_ref);
        // E o mixer antes de `amix.`.
        let mixer = args
            .iter()
            .position(|arg| arg == "name=amix")
            .expect("mixer");
        let first_mix = args.iter().position(|arg| arg == "amix.").expect("amix.");
        assert!(mixer < first_mix);
    }

    #[test]
    fn one_audio_source_skips_the_mixer() {
        let spec = spec(RecordingKind::Video, "native");
        let args = args(&spec, Some("default"), None);
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "pulsesrc").count(),
            1
        );
        assert!(!args.contains(&"audiomixer".to_string()));
        assert!(args.contains(&"avenc_aac".to_string()));
    }

    #[test]
    fn forced_resolution_and_fps_reach_the_caps() {
        let spec = spec(RecordingKind::Video, "1280x720");
        let args = args(&spec, None, None);
        let caps = args
            .iter()
            .find(|arg| arg.starts_with("video/x-raw"))
            .expect("caps");
        assert!(caps.contains("width=1280"));
        assert!(caps.contains("height=720"));
        assert!(caps.contains("framerate=8/1"));
        assert!(args.contains(&"videoscale".to_string()));
        assert!(args.contains(&"videorate".to_string()));
    }

    #[test]
    fn video_without_audio_has_no_pulse() {
        let spec = spec(RecordingKind::Video, "native");
        let args = args(&spec, None, None);
        assert!(!args.contains(&"pulsesrc".to_string()));
        assert!(!args.contains(&"avenc_aac".to_string()));
        assert!(args.contains(&"matroskamux".to_string()));
    }

    #[test]
    fn audio_only_uses_flac_without_the_portal() {
        let spec = spec(RecordingKind::Audio, "native");
        let output = PathBuf::from("/tmp/part-0000.flac");
        let args = segment_args(&GstSegmentRequest {
            spec: &spec,
            output: &output,
            stream: None,
            microphone: Some("default"),
            system: None,
        });
        assert!(args.contains(&"flacenc".to_string()));
        assert!(args.contains(&"flacparse".to_string()));
        assert!(!args.contains(&"pipewiresrc".to_string()));
        assert!(!args.contains(&"matroskamux".to_string()));
    }

    #[test]
    fn audio_only_with_two_sources_uses_the_mixer() {
        let spec = spec(RecordingKind::Audio, "native");
        let output = PathBuf::from("/tmp/part-0000.flac");
        let args = segment_args(&GstSegmentRequest {
            spec: &spec,
            output: &output,
            stream: None,
            microphone: Some("default"),
            system: Some("alsa_output.monitor"),
        });
        assert!(args.contains(&"audiomixer".to_string()));
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "pulsesrc").count(),
            2
        );
    }

    #[test]
    fn audio_only_without_sources_is_empty() {
        let spec = spec(RecordingKind::Audio, "native");
        let output = PathBuf::from("/tmp/part-0000.flac");
        assert!(segment_args(&GstSegmentRequest {
            spec: &spec,
            output: &output,
            stream: None,
            microphone: None,
            system: None,
        })
        .is_empty());
    }

    #[test]
    fn video_without_portal_stream_is_empty() {
        let spec = spec(RecordingKind::Video, "native");
        let output = PathBuf::from("/tmp/part-0000.mkv");
        assert!(segment_args(&GstSegmentRequest {
            spec: &spec,
            output: &output,
            stream: None,
            microphone: Some("default"),
            system: None,
        })
        .is_empty());
    }
}
