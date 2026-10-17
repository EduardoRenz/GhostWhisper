//! Registro de atalhos globais sobre `tauri-plugin-global-shortcut`.

use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::domain::shortcuts::{ShortcutAction, ShortcutBinding, ShortcutConflict};
use crate::ports::shortcut_registry::GlobalShortcutRegistry;

/// Callback chamado quando um atalho é pressionado.
pub type ShortcutHandler = Arc<dyn Fn(ShortcutAction) + Send + Sync>;

/// Implementa `GlobalShortcutRegistry` via plugin do Tauri.
pub struct TauriGlobalShortcutRegistry {
    app: AppHandle,
    handler: ShortcutHandler,
    active: Mutex<Vec<ShortcutBinding>>,
}

impl TauriGlobalShortcutRegistry {
    /// Cria o registro com o `AppHandle` e o despachante de ações.
    pub fn new(app: AppHandle, handler: ShortcutHandler) -> Self {
        Self {
            app,
            handler,
            active: Mutex::new(Vec::new()),
        }
    }
}

impl GlobalShortcutRegistry for TauriGlobalShortcutRegistry {
    fn apply(&self, bindings: &[ShortcutBinding]) -> Vec<ShortcutConflict> {
        let manager = self.app.global_shortcut();
        let _ = manager.unregister_all();
        if let Ok(mut active) = self.active.lock() {
            active.clear();
        }

        let mut conflicts = Vec::new();
        for binding in bindings {
            let accelerator = binding.accelerator.trim();
            if accelerator.is_empty() {
                continue;
            }

            let handler = Arc::clone(&self.handler);
            let action = binding.action;
            let result = manager.on_shortcut(accelerator, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    handler(action);
                }
            });

            match result {
                Ok(()) => {
                    if let Ok(mut active) = self.active.lock() {
                        active.push(binding.clone());
                    }
                }
                Err(error) => conflicts.push(ShortcutConflict {
                    action,
                    accelerator: accelerator.to_string(),
                    reason: error.to_string(),
                }),
            }
        }
        conflicts
    }

    fn active(&self) -> Vec<ShortcutBinding> {
        self.active
            .lock()
            .map(|active| active.clone())
            .unwrap_or_default()
    }
}
