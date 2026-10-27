//! Consulta local dos logs do app.

use crate::error::{GhostWhisperError, Result};

/// Caminho absoluto do arquivo de log local.
#[tauri::command(async)]
pub fn logs_path() -> Result<String> {
    crate::log_file_path()
        .map(|path| path.display().to_string())
        .ok_or_else(|| GhostWhisperError::Other("diretório de logs indisponível".to_string()))
}

/// Últimas `limit` linhas do log (padrão 300, teto 2.000).
#[tauri::command(async)]
pub fn logs_recent(limit: Option<usize>) -> Result<String> {
    let path = crate::log_file_path()
        .ok_or_else(|| GhostWhisperError::Other("diretório de logs indisponível".to_string()))?;
    if !path.is_file() {
        return Ok(String::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(tail_lines(&content, limit.unwrap_or(300).clamp(1, 2_000)))
}

fn tail_lines(content: &str, limit: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(limit);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_only_the_requested_lines() {
        let content = "a\nb\nc\nd\ne\n";
        assert_eq!(tail_lines(content, 2), "d\ne");
        assert_eq!(tail_lines(content, 99), "a\nb\nc\nd\ne");
        assert_eq!(tail_lines("", 10), "");
    }
}
