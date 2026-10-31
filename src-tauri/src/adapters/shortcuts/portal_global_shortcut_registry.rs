//! Registro de atalhos globais no **Wayland** via XDG Desktop Portal.
//!
//! O `global-hotkey`/X11 não recebe teclas de apps Wayland nativos. O portal
//! `org.freedesktop.portal.GlobalShortcuts` pede a aprovação do usuário uma vez,
//! entrega as teclas ao compositor e devolve `Activated` mesmo com outra janela
//! em foco (Chrome, meeting etc.).
//!
//! O portal só aceita `BindShortcuts` uma vez por sessão; quando o usuário muda
//! os atalhos, uma nova sessão é criada e a anterior é fechada. O trabalho é
//! assíncrono (D-Bus) e roda em uma thread própria para não travar o boot.

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::Session;
use futures_util::StreamExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::adapters::shortcuts::tauri_global_shortcut_registry::ShortcutHandler;
use crate::domain::shortcuts::{portal_trigger, ShortcutAction, ShortcutBinding, ShortcutConflict};
use crate::ports::shortcut_registry::GlobalShortcutRegistry;

/// Implementa `GlobalShortcutRegistry` pelo portal do Wayland.
pub struct PortalGlobalShortcutRegistry {
    /// Vínculos que o portal confirmou (ou os pedidos, até a confirmação).
    active: Arc<Mutex<Vec<ShortcutBinding>>>,
    /// Comandos para a thread do portal (reaplicar vínculos).
    tx: UnboundedSender<Vec<ShortcutBinding>>,
    /// Mantém a thread viva enquanto o registro existir.
    _thread: Mutex<Option<JoinHandle<()>>>,
}

impl PortalGlobalShortcutRegistry {
    /// Cria o registro e inicia a thread do portal.
    pub fn new(handler: ShortcutHandler) -> Self {
        let (tx, rx) = unbounded_channel();
        let active = Arc::new(Mutex::new(Vec::new()));
        let worker_active = Arc::clone(&active);
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let Ok(runtime) = runtime else {
                tracing::warn!("atalhos globais: não foi possível criar o runtime do portal");
                return;
            };
            runtime.block_on(worker(rx, worker_active, handler));
        });

        Self {
            active,
            tx,
            _thread: Mutex::new(Some(thread)),
        }
    }
}

impl GlobalShortcutRegistry for PortalGlobalShortcutRegistry {
    fn apply(&self, bindings: &[ShortcutBinding]) -> Vec<ShortcutConflict> {
        // Otimista: a UI reflete a escolha enquanto o diálogo do portal não
        // responde. A thread corrige para o conjunto realmente aceito.
        if let Ok(mut active) = self.active.lock() {
            *active = bindings.to_vec();
        }
        if self.tx.send(bindings.to_vec()).is_err() {
            tracing::warn!("atalhos globais: thread do portal encerrada");
        }
        // O portal não devolve conflitos de forma síncrona; o usuário aprova ou
        // ajusta no diálogo do compositor.
        Vec::new()
    }

    fn active(&self) -> Vec<ShortcutBinding> {
        self.active
            .lock()
            .map(|active| active.clone())
            .unwrap_or_default()
    }
}

/// Corpo da thread: cria a sessão do portal e troca de sessão a cada `apply`.
async fn worker(
    mut rx: UnboundedReceiver<Vec<ShortcutBinding>>,
    active: Arc<Mutex<Vec<ShortcutBinding>>>,
    handler: ShortcutHandler,
) {
    let connection = match crate::adapters::system::portal::connection().await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::warn!("atalhos globais: conexão do portal indisponível: {error}");
            return;
        }
    };
    let portal = match GlobalShortcuts::with_connection(connection).await {
        Ok(portal) => portal,
        Err(error) => {
            tracing::warn!("atalhos globais indisponíveis no portal Wayland: {error}");
            return;
        }
    };
    let activated = match portal.receive_activated().await {
        Ok(stream) => stream,
        Err(error) => {
            tracing::warn!("atalhos globais: sinal do portal indisponível: {error}");
            return;
        }
    };
    tokio::pin!(activated);

    let mut session: Option<Session<GlobalShortcuts>> = None;
    loop {
        tokio::select! {
            command = rx.recv() => {
                let Some(bindings) = command else {
                    if let Some(previous) = session.take() {
                        let _ = previous.close().await;
                    }
                    return;
                };

                // `BindShortcuts` só vale uma vez por sessão: fecha a anterior.
                if let Some(previous) = session.take() {
                    let _ = previous.close().await;
                }
                match bind_session(&portal, &bindings).await {
                    Ok((next_session, bound)) => {
                        session = Some(next_session);
                        if let Ok(mut active) = active.lock() {
                            *active = bound;
                        }
                    }
                    Err(error) => {
                        tracing::warn!("atalhos globais: falha ao registrar no portal: {error}");
                        if let Ok(mut active) = active.lock() {
                            active.clear();
                        }
                    }
                }
            }
            Some(event) = activated.next() => {
                if let Some(action) = ShortcutAction::from_str(event.shortcut_id()) {
                    handler(action);
                }
            }
        }
    }
}

/// Cria uma sessão e registra os vínculos; devolve os que o portal aceitou.
async fn bind_session(
    portal: &GlobalShortcuts,
    bindings: &[ShortcutBinding],
) -> Result<(Session<GlobalShortcuts>, Vec<ShortcutBinding>), ashpd::Error> {
    let session = portal.create_session(Default::default()).await?;
    let shortcuts: Vec<NewShortcut> = bindings
        .iter()
        .map(|binding| {
            let shortcut = NewShortcut::new(binding.action.as_str(), binding.action.label());
            match portal_trigger(&binding.accelerator) {
                Some(trigger) => shortcut.preferred_trigger(trigger.as_str()),
                None => shortcut,
            }
        })
        .collect();

    let response = portal
        .bind_shortcuts(&session, &shortcuts, None, Default::default())
        .await?
        .response()?;
    let accepted: Vec<&str> = response
        .shortcuts()
        .iter()
        .map(|shortcut| shortcut.id())
        .collect();
    let bound = bindings
        .iter()
        .filter(|binding| accepted.contains(&binding.action.as_str()))
        .cloned()
        .collect();
    Ok((session, bound))
}
