//! Adaptador de sink virtual via `pactl` (módulo `module-null-sink`).
//!
//! Permite capturar o áudio do sistema quando não há monitor de saída, roteando
//! a saída do app de reunião para um sink "GhostWhisper".

use crate::error::Result;
use crate::ports::virtual_sink::VirtualSinkManager;

use super::pactl;

/// Implementa `VirtualSinkManager` criando/removendo um sink nulo.
pub struct PactlVirtualSinkManager;

impl VirtualSinkManager for PactlVirtualSinkManager {
    fn create(&self, name: &str) -> Result<u32> {
        let sink_name = format!("sink_name={name}");
        let session_name = format!("session.sink_name={name}");
        let output = pactl::run_pactl(&[
            "load-module",
            "module-null-sink",
            sink_name.as_str(),
            session_name.as_str(),
        ])?;
        pactl::parse_module_index(&output)
    }

    fn remove(&self, module_index: u32) -> Result<()> {
        pactl::run_pactl(&["unload-module", &module_index.to_string()])?;
        Ok(())
    }
}
