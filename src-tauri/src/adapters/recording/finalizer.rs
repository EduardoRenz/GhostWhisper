//! Finalização da gravação: concatena os segmentos e remuxa para o container
//! final (BR-067).
//!
//! `-c copy` por padrão; se as dimensões dos segmentos divergirem (janela
//! redimensionada — ISS-053), reencoda com `scale`+`pad` para o tamanho do
//! primeiro segmento.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::domain::recording::{RecordingArtifact, RecordingKind, RecordingSpec};
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_recorder::{FinalizeRequest, RecordingFinalizer};

use super::ffmpeg_args::{
    concat_args, concat_line, fallback_audio_args, fallback_mux_args, normalize_args,
};

/// Entrada do `concat` do ffmpeg (usada pelos builders puros).
#[derive(Debug, Clone)]
pub struct ConcatRequest {
    pub parts_file: PathBuf,
    pub output: PathBuf,
    pub spec: RecordingSpec,
}

/// Finalizador que usa o binário do `ffmpeg`.
pub struct FfmpegRecordingFinalizer {
    ffmpeg: PathBuf,
}

impl FfmpegRecordingFinalizer {
    /// Cria o finalizador com o caminho do binário já resolvido.
    pub fn new(ffmpeg: PathBuf) -> Self {
        Self { ffmpeg }
    }

    /// Roda um comando acompanhando `out_time_us`.
    fn run(&self, args: &[String], total_ms: i64, progress: &dyn Fn(u8)) -> Result<()> {
        let mut child = Command::new(&self.ffmpeg)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                GhostWhisperError::Other(format!("não foi possível executar o ffmpeg: {error}"))
            })?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut last = 0u8;
            for line in reader.lines().map_while(std::result::Result::ok) {
                let Some(raw) = line.strip_prefix("out_time_us=") else {
                    continue;
                };
                let Ok(micros) = raw.trim().parse::<i64>() else {
                    continue;
                };
                if total_ms <= 0 {
                    continue;
                }
                let percent = ((micros / 1_000) * 100 / total_ms).clamp(0, 99) as u8;
                if percent != last {
                    last = percent;
                    progress(percent);
                }
            }
        }

        let output = child.wait_with_output().map_err(|error| {
            GhostWhisperError::Other(format!("falha ao aguardar o ffmpeg: {error}"))
        })?;
        if output.status.success() {
            progress(100);
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let summary = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("erro desconhecido")
            .trim()
            .to_string();
        Err(GhostWhisperError::Other(format!(
            "finalização falhou: {summary}"
        )))
    }
}

impl RecordingFinalizer for FfmpegRecordingFinalizer {
    fn finalize(
        &self,
        request: &FinalizeRequest,
        progress: &dyn Fn(u8),
    ) -> Result<RecordingArtifact> {
        if request.parts.is_empty() && request.audio.is_none() {
            return Err(GhostWhisperError::Other(
                "nenhum segmento para finalizar".to_string(),
            ));
        }
        if let Some(parent) = request.output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let parts_file = request.output.with_file_name("parts.txt");
        let mut content = String::new();
        for part in &request.parts {
            content.push_str(&concat_line(part));
        }
        std::fs::write(&parts_file, content)?;

        let concat = ConcatRequest {
            parts_file,
            output: request.output.clone(),
            spec: request.spec.clone(),
        };

        match (request.spec.kind, &request.audio) {
            (RecordingKind::Audio, Some(audio)) => self.run(
                &fallback_audio_args(&concat, audio, &request.output),
                request.total_ms,
                progress,
            )?,
            (RecordingKind::Video, Some(audio)) => {
                let normalize = request.normalize.then(|| normalize_size(request));
                self.run(
                    &fallback_mux_args(&concat, audio, &request.output, normalize),
                    request.total_ms,
                    progress,
                )?
            }
            _ => {
                if request.normalize {
                    let (width, height) = normalize_size(request);
                    self.run(
                        &normalize_args(&concat, width, height),
                        request.total_ms,
                        progress,
                    )?;
                } else if let Err(first_error) =
                    self.run(&concat_args(&concat), request.total_ms, progress)
                {
                    let (width, height) = normalize_size(request);
                    tracing::warn!(
                        "concat direto falhou ({first_error}); reencodando {width}x{height} (ISS-053)"
                    );
                    self.run(
                        &normalize_args(&concat, width, height),
                        request.total_ms,
                        progress,
                    )?;
                }
            }
        }

        let size_bytes = std::fs::metadata(&request.output)
            .map(|meta| meta.len())
            .unwrap_or(0);
        let (width, height) = normalize_size(request);

        Ok(RecordingArtifact {
            path: request.output.clone(),
            size_bytes,
            duration_ms: request.total_ms,
            width,
            height,
        })
    }

    fn cleanup(&self, parts: &[PathBuf], output: &Path) {
        cleanup_parts(parts, output);
    }
}

/// Dimensões-base da finalização (primeiro segmento; 1920×1080 como piso).
fn normalize_size(request: &FinalizeRequest) -> (u32, u32) {
    if request.width > 0 && request.height > 0 {
        return (request.width, request.height);
    }
    let (width, height) = request.spec.target.dimensions();
    if width > 0 && height > 0 {
        (width, height)
    } else {
        (1920, 1080)
    }
}

/// Apaga os segmentos e o `parts.txt` de uma gravação finalizada.
pub fn cleanup_parts(parts: &[PathBuf], output: &Path) {
    for part in parts {
        let _ = std::fs::remove_file(part);
    }
    let _ = std::fs::remove_file(output.with_file_name("parts.txt"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_lines_escape_single_quotes() {
        let line = concat_line(Path::new("/tmp/o'brien/part.mkv"));
        assert_eq!(line, "file '/tmp/o'\\''brien/part.mkv'\n");
    }
}
