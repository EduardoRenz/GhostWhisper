//! Execução do utilitário `pactl` (PulseAudio/PipeWire).

use crate::error::{GhostWhisperError, Result};

/// Executa `pactl` com os argumentos dados e retorna o stdout.
pub fn run_pactl(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| GhostWhisperError::Other(format!("pactl indisponível: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GhostWhisperError::Other(format!(
            "pactl {args:?} falhou: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extrai o índice do módulo impresso por `pactl load-module`.
pub fn parse_module_index(output: &str) -> Result<u32> {
    output
        .trim()
        .parse::<u32>()
        .map_err(|_| GhostWhisperError::Other(format!("índice de módulo inválido: {output:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module_index() {
        assert_eq!(parse_module_index("42\n").expect("índice"), 42);
    }

    #[test]
    fn rejects_invalid_module_index() {
        assert!(parse_module_index("boom").is_err());
    }
}
