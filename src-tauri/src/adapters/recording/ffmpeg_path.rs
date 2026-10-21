//! Resolução do binário do `ffmpeg` em degraus (RNF-09).
//!
//! 1. `GHOSTWHISPER_FFMPEG` (caminho explícito — dev/teste);
//! 2. sidecar empacotado ao lado do executável;
//! 3. `ffmpeg` no `PATH`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::{GhostWhisperError, Result};

/// Variável de ambiente com o caminho explícito.
pub const FFMPEG_ENV: &str = "GHOSTWHISPER_FFMPEG";

/// Resolve o `ffmpeg` no ambiente real.
pub fn resolve_ffmpeg() -> Result<PathBuf> {
    let configured = std::env::var(FFMPEG_ENV).ok();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let path_var = std::env::var_os("PATH");
    resolve_from(
        configured.as_deref(),
        exe_dir.as_deref(),
        path_var.as_deref(),
    )
}

/// Regra pura de resolução (testável sem tocar no processo).
pub fn resolve_from(
    configured: Option<&str>,
    exe_dir: Option<&Path>,
    path_var: Option<&OsStr>,
) -> Result<PathBuf> {
    if let Some(configured) = configured {
        let path = PathBuf::from(configured.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(GhostWhisperError::Other(format!(
            "{FFMPEG_ENV} aponta para um arquivo inexistente: {}",
            path.display()
        )));
    }

    if let Some(dir) = exe_dir {
        let sidecar = dir.join("ffmpeg");
        if sidecar.is_file() {
            return Ok(sidecar);
        }
    }

    if let Some(path_var) = path_var {
        for dir in std::env::split_paths(path_var) {
            let candidate = dir.join("ffmpeg");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(GhostWhisperError::Other(
        "ffmpeg não encontrado — instale o pacote ou defina GHOSTWHISPER_FFMPEG".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_wins() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("ffmpeg");
        std::fs::write(&binary, b"#!/bin/sh\n").expect("arquivo");
        let resolved = resolve_from(Some(binary.to_str().unwrap()), None, None).expect("resolve");
        assert_eq!(resolved, binary);
    }

    #[test]
    fn explicit_missing_path_is_an_error() {
        let err = resolve_from(Some("/nao/existe/ffmpeg"), None, None).expect_err("erro");
        assert!(err.to_string().contains("inexistente"), "{err}");
    }

    #[test]
    fn sidecar_beats_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("ffmpeg");
        std::fs::write(&binary, b"x").expect("arquivo");
        let resolved = resolve_from(None, Some(dir.path()), None).expect("resolve");
        assert_eq!(resolved, binary);
    }

    #[test]
    fn falls_back_to_path_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("ffmpeg");
        std::fs::write(&binary, b"x").expect("arquivo");
        let path_var = std::env::join_paths([dir.path()]).expect("join");
        let resolved = resolve_from(None, None, Some(&path_var)).expect("resolve");
        assert_eq!(resolved, binary);
    }

    #[test]
    fn missing_everywhere_is_an_actionable_error() {
        let err = resolve_from(None, None, None).expect_err("erro");
        assert!(err.to_string().contains("GHOSTWHISPER_FFMPEG"), "{err}");
    }
}
