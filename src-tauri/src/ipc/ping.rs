//! Comando de verificação de vida (`ping`).
//!
//! Serve como smoke test do canal IPC e é substituído por comandos reais
//! conforme as features chegam.

use serde::Serialize;

use crate::error::Result;

/// Resposta de `ping`.
#[derive(Debug, Serialize)]
pub struct PingResponse {
    pub name: &'static str,
    pub version: &'static str,
}

/// Retorna a identidade e a versão do GhostWhisper.
#[tauri::command]
pub fn ping() -> Result<PingResponse> {
    Ok(PingResponse {
        name: "GhostWhisper",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_name_and_version() {
        let response = ping().expect("ping deve responder");
        assert_eq!(response.name, "GhostWhisper");
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }
}
