//! GhostWhisper — composition root.
//!
//! Único lugar do projeto onde adaptadores concretos são instanciados e
//! injetados nas portas. Toda a fiação de dependência (DI) parte daqui.

mod adapters;
mod application;
mod domain;
mod error;
mod ipc;
mod ports;
mod state;

use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;

use domain::shortcuts::ShortcutAction;

pub use error::{GhostWhisperError, Result};

/// Inicializa e executa a aplicação Tauri.
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ipc::ping::ping,
            ipc::overlay::overlay_toggle,
            ipc::overlay::overlay_set_stealth,
            ipc::overlay::overlay_set_click_through,
            ipc::overlay::overlay_set_interactive_regions,
            ipc::overlay::overlay_set_surface,
            ipc::overlay::overlay_start_drag,
            ipc::audio_commands::audio_list_devices,
            ipc::audio_commands::audio_start,
            ipc::audio_commands::audio_stop,
            ipc::audio_commands::audio_is_running,
            ipc::audio_commands::system_audio_list_devices,
            ipc::audio_commands::system_audio_start,
            ipc::audio_commands::system_audio_stop,
            ipc::audio_commands::system_audio_is_running,
            ipc::audio_commands::system_sink_create,
            ipc::audio_commands::system_sink_remove,
            ipc::audio_commands::audio_monitor_default_delay,
            ipc::audio_commands::audio_monitor_start,
            ipc::audio_commands::audio_monitor_stop,
            ipc::audio_commands::audio_monitor_is_running,
            ipc::session_commands::session_start,
            ipc::session_commands::session_stop,
            ipc::session_commands::session_pause,
            ipc::session_commands::session_resume,
            ipc::session_commands::session_set_channels,
            ipc::session_commands::session_continue,
            ipc::session_commands::session_current,
            ipc::database_commands::db_list_sessions,
            ipc::database_commands::db_get_session,
            ipc::database_commands::db_session_transcriptions,
            ipc::database_commands::db_session_messages,
            ipc::database_commands::db_session_timeline,
            ipc::database_commands::db_search_transcriptions,
            ipc::session_commands::session_rename,
            ipc::session_commands::session_delete,
            ipc::export_commands::session_export_markdown,
            ipc::export_commands::session_save_markdown,
            ipc::settings_commands::settings_open,
            ipc::settings_commands::settings_providers,
            ipc::settings_commands::settings_get,
            ipc::settings_commands::settings_set,
            ipc::settings_commands::settings_set_provider_key,
            ipc::settings_commands::settings_has_provider_key,
            ipc::settings_commands::settings_clear_provider_key,
            ipc::settings_commands::settings_validate_key,
            ipc::settings_commands::settings_provider_label,
            ipc::settings_commands::settings_key_preview,
            ipc::settings_commands::settings_secrets_persistent,
            ipc::llm_commands::llm_translate,
            ipc::llm_commands::llm_insight,
            ipc::shortcuts_commands::shortcuts_get,
            ipc::shortcuts_commands::shortcuts_active,
            ipc::shortcuts_commands::shortcuts_set,
            ipc::shortcuts_commands::shortcuts_catalog,
        ])
        .setup(|app| {
            let app_state = build_app_state(app)?;
            app.manage(app_state);
            // A janela de configurações só é escondida ao fechar (nunca
            // destruída), para poder ser reaberta quantas vezes for preciso.
            if let Some(settings_window) = app.get_webview_window("settings") {
                let window = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });
            }
            // O tray é opcional: não derruba o app se o indicador faltar.
            if let Err(error) = build_tray(app) {
                tracing::warn!("bandeja indisponível: {error}");
            }
            spawn_overlay_position_tracker(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao executar o GhostWhisper");
}

/// Composition root: monta adaptadores concretos e injeta nas portas.
fn build_app_state(app: &tauri::App) -> Result<state::AppState> {
    use adapters::ai::default_chat_bot_provider_factory::DefaultChatBotProviderFactory;
    use adapters::audio::cpal_microphone_source::CpalMicrophoneSource;
    use adapters::audio::microphone_front_end::MicrophoneFrontEnd;
    use adapters::audio::monitored_audio_source::MonitoredAudioSource;
    use adapters::audio::pactl_sink_helper::PactlVirtualSinkManager;
    use adapters::audio::pulse_delayed_monitor::PulseDelayedMonitor;
    use adapters::audio::pulse_system_audio_source::PulseSystemAudioSource;
    use adapters::events::tauri_event_publisher::TauriEventPublisher;
    use adapters::overlay::cursor_click_through::CursorClickThroughController;
    use adapters::overlay::tauri_overlay_window::TauriOverlayWindow;
    use adapters::overlay::x11_input_shape_click_through::X11InputShapeClickThrough;
    use adapters::persistence::sqlite::connection as db_connection;
    use adapters::persistence::sqlite::local_messages_repository::LocalMessagesRepository;
    use adapters::persistence::sqlite::local_session_repository::LocalSessionRepository;
    use adapters::persistence::sqlite::local_transcriptions_repository::LocalTranscriptionsRepository;
    use adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use adapters::secrets::keyring_secret_store::KeyringSecretStore;
    use adapters::settings::file_settings_store::FileSettingsStore;
    use adapters::shortcuts::tauri_global_shortcut_registry::{
        ShortcutHandler, TauriGlobalShortcutRegistry,
    };
    use adapters::speech::whisper_model_store::FileModelStore;
    use adapters::speech::whisper_speech_engine_factory::WhisperSpeechEngineFactory;
    use adapters::system::system_clock::SystemClock;
    use adapters::system::x11_pointer_source::{NullPointerSource, X11PointerSource};
    use application::insight_service::InsightService;
    use application::llm_router::LlmRouter;
    use application::session_service::SessionService;
    use application::translation_service::TranslationService;
    use ports::audio_monitor::AudioMonitor;
    use ports::audio_source::AudioSource;
    use ports::click_through::ClickThroughController;
    use ports::clock::Clock;
    use ports::event_publisher::EventPublisher;
    use ports::insight_generator::InsightGenerator;
    use ports::llm_controller::LlmController;
    use ports::llm_provider_factory::ChatBotProviderFactory;
    use ports::model_store::ModelStore;
    use ports::overlay_window::OverlayWindowController;
    use ports::pointer_source::PointerSource;
    use ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
    use ports::secret_store::SecretStore;
    use ports::session_controller::SessionController;
    use ports::settings_store::SettingsStore;
    use ports::shortcut_registry::GlobalShortcutRegistry;
    use ports::speech_engine_factory::SpeechEngineFactory;
    use ports::translator::Translator;
    use ports::virtual_sink::VirtualSinkManager;

    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| GhostWhisperError::Other("janela 'overlay' não encontrada".to_string()))?;
    let overlay_window = window.clone();

    let overlay: Arc<dyn OverlayWindowController> = Arc::new(TauriOverlayWindow::new(window));

    // Click-through: input-shape por região no X11; polling como fallback.
    let click_through: Arc<dyn ClickThroughController> =
        match X11InputShapeClickThrough::connect(&overlay_window, Arc::clone(&overlay)) {
            Some(shape) => {
                tracing::info!("click-through via X11 input-shape (sem polling)");
                Arc::new(shape)
            }
            None => {
                let pointer: Arc<dyn PointerSource> = match X11PointerSource::connect() {
                    Some(source) => Arc::new(source),
                    None => {
                        tracing::warn!("X11 indisponível; click-through automático desativado");
                        Arc::new(NullPointerSource)
                    }
                };
                let polling = Arc::new(CursorClickThroughController::new(
                    Arc::clone(&overlay),
                    pointer,
                ));
                polling.spawn();
                polling
            }
        };

    // Diretório de dados (banco + modelos + settings).
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| GhostWhisperError::Other(format!("diretório de dados indisponível: {e}")))?;
    migrate_legacy_data_dir(&data_dir)?;

    // Configurações primeiro: o estágio de ganho do microfone as lê na captura.
    let settings: Arc<dyn SettingsStore> =
        Arc::new(FileSettingsStore::new(data_dir.join("settings.json")));

    // Captura: o microfone passa pelo estágio de ganho (BR-018) e por um ponto
    // de escuta para o monitor atrasado (task 15); o áudio do sistema (loopback)
    // segue sem amplificação.
    let monitor: Arc<dyn AudioMonitor> = Arc::new(PulseDelayedMonitor::new());
    let audio: Arc<dyn AudioSource> = Arc::new(MonitoredAudioSource::new(
        Arc::new(MicrophoneFrontEnd::new(
            Arc::new(CpalMicrophoneSource::new()),
            Arc::clone(&settings),
        )),
        Arc::clone(&monitor),
    ));
    let system_audio: Arc<dyn AudioSource> = Arc::new(PulseSystemAudioSource::new());
    let virtual_sink: Arc<dyn VirtualSinkManager> = Arc::new(PactlVirtualSinkManager);

    let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(data_dir.join("models")));
    let publisher: Arc<dyn EventPublisher> =
        Arc::new(TauriEventPublisher::new(app.handle().clone()));

    // Banco de dados (SQLite) e repositórios.
    let connection = db_connection::shared(db_connection::open(&data_dir.join("ghostwhisper.db"))?);
    let sessions: Arc<dyn SessionRepository> =
        Arc::new(LocalSessionRepository::new(Arc::clone(&connection)));
    let transcriptions: Arc<dyn TranscriptionsRepository> =
        Arc::new(LocalTranscriptionsRepository::new(Arc::clone(&connection)));
    let messages: Arc<dyn MessagesRepository> =
        Arc::new(LocalMessagesRepository::new(Arc::clone(&connection)));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    // Sessões órfãs: app fechado durante uma gravação deixa a sessão `active` no
    // banco. Encerra no boot para o histórico (task 17) não mentir (BR-019).
    match sessions.end_all_active(clock.now_ms()) {
        Ok(0) => {}
        Ok(count) => tracing::info!("{count} sessão(ões) órfã(s) encerrada(s) no boot"),
        Err(error) => tracing::warn!("falha ao encerrar sessões órfãs: {error}"),
    }

    // Segredos (BYO key). Sem Secret Service, a chave fica apenas em memória
    // durante a execução (BR-003).
    let secrets: Arc<dyn SecretStore> = {
        let keyring = KeyringSecretStore::new();
        if keyring.is_available() {
            Arc::new(keyring)
        } else {
            tracing::warn!(
                "Secret Service indisponível; chaves ficarão apenas em memória nesta execução"
            );
            Arc::new(InMemorySecretStore::new())
        }
    };
    let factory: Arc<dyn ChatBotProviderFactory> = Arc::new(DefaultChatBotProviderFactory);
    let llm: Arc<dyn LlmController> = Arc::new(LlmRouter::new(
        factory,
        Arc::clone(&settings),
        Arc::clone(&secrets),
    ));

    // Casos de uso de LLM (tradução e insights).
    let translator: Arc<dyn Translator> = Arc::new(TranslationService::new(
        Arc::clone(&llm),
        Arc::clone(&settings),
    ));
    let insight: Arc<dyn InsightGenerator> = Arc::new(InsightService::new(
        Arc::clone(&llm),
        Arc::clone(&settings),
        Arc::clone(&sessions),
        Arc::clone(&transcriptions),
        Arc::clone(&messages),
        Arc::clone(&clock),
    ));

    // Atalhos globais: o despachante resolve o `AppState` em tempo de execução.
    let shortcuts: Arc<dyn GlobalShortcutRegistry> = {
        let app_handle = app.handle().clone();
        let handler: ShortcutHandler = Arc::new(move |action: ShortcutAction| {
            dispatch_shortcut(&app_handle, action);
        });
        Arc::new(TauriGlobalShortcutRegistry::new(
            app.handle().clone(),
            handler,
        ))
    };

    // Aplica settings persistidas: atalhos, stealth e posição do overlay.
    let current_settings = settings.load().unwrap_or_default();
    let conflicts = shortcuts.apply(&current_settings.shortcuts);
    if !conflicts.is_empty() {
        tracing::warn!("{} atalho(s) não registrado(s)", conflicts.len());
    }
    overlay.set_stealth(current_settings.stealth);
    if let Some((x, y)) = current_settings.overlay_position {
        let _ = overlay_window.set_position(tauri::PhysicalPosition::new(x, y));
        // Posição escolhida pelo usuário: não reancorar pela base.
        overlay.set_anchor_bottom(false);
    }

    // Motor de fala por canal: cada pipeline recebe seu próprio contexto.
    // Modelo e idioma do whisper vêm das configurações (lidas a cada sessão).
    let engines: Arc<dyn SpeechEngineFactory> = Arc::new(
        WhisperSpeechEngineFactory::new(Arc::clone(&models), Arc::clone(&settings))
            .with_publisher(Arc::clone(&publisher)),
    );

    let session: Arc<dyn SessionController> = Arc::new(SessionService::new(
        Arc::clone(&audio),
        Arc::clone(&system_audio),
        engines,
        Arc::clone(&publisher),
        Arc::clone(&sessions),
        Arc::clone(&transcriptions),
        Arc::clone(&clock),
    ));

    Ok(state::AppState::new(
        overlay,
        click_through,
        audio,
        system_audio,
        monitor,
        virtual_sink,
        session,
        sessions,
        transcriptions,
        messages,
        clock,
        settings,
        secrets,
        llm,
        translator,
        insight,
        publisher,
        shortcuts,
    ))
}

/// Cria o ícone de bandeja (sempre visível) com o menu de ações.
fn build_tray(app: &tauri::App) -> Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let to_err = |error: tauri::Error| GhostWhisperError::Other(error.to_string());

    let toggle = MenuItem::with_id(
        app,
        "toggle_overlay",
        "Mostrar/ocultar overlay",
        true,
        None::<&str>,
    )
    .map_err(to_err)?;
    let session = MenuItem::with_id(
        app,
        "toggle_session",
        "Iniciar/encerrar sessão",
        true,
        None::<&str>,
    )
    .map_err(to_err)?;
    let settings =
        MenuItem::with_id(app, "settings", "Configurações", true, None::<&str>).map_err(to_err)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>).map_err(to_err)?;

    let menu = Menu::with_items(app, &[&toggle, &session, &settings, &quit]).map_err(to_err)?;

    let mut builder = TrayIconBuilder::with_id("ghostwhisper-tray")
        .menu(&menu)
        .tooltip("GhostWhisper")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle_overlay" => dispatch_shortcut(app, ShortcutAction::ToggleOverlay),
            "toggle_session" => dispatch_shortcut(app, ShortcutAction::ToggleSession),
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app).map_err(to_err)?;
    Ok(())
}

/// Mostra e foca a janela de configurações.
fn open_settings(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("settings") else {
        tracing::warn!("janela 'settings' não encontrada");
        return;
    };
    let _ = window.unminimize();
    if let Err(error) = window.show() {
        tracing::warn!("falha ao mostrar configurações: {error}");
    }
    if let Err(error) = window.set_focus() {
        tracing::warn!("falha ao focar configurações: {error}");
    }
}

/// Persiste a posição do overlay enquanto o usuário a controla.
fn spawn_overlay_position_tracker(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(700));

        let Some(window) = app.get_webview_window("overlay") else {
            continue;
        };
        if !window.is_visible().unwrap_or(false) {
            continue;
        }

        let state = app.state::<state::AppState>();
        // Só persiste depois que o usuário moveu (ancoragem desligada).
        if state.overlay.anchor_bottom() {
            continue;
        }

        let Ok(position) = window.outer_position() else {
            continue;
        };
        let current = (position.x, position.y);
        if let Ok(mut settings) = state.settings.load() {
            if settings.overlay_position != Some(current) {
                settings.overlay_position = Some(current);
                let _ = state.settings.save(&settings);
            }
        }
    });
}

/// Executa a ação de um atalho global usando o estado gerenciado.
fn dispatch_shortcut(app: &tauri::AppHandle, action: ShortcutAction) {
    use crate::ports::session_controller::AudioChannel;

    let state = app.state::<state::AppState>();
    match action {
        ShortcutAction::ToggleOverlay => {
            state.overlay.toggle();
        }
        ShortcutAction::ToggleStealth => {
            if let Ok(mut settings) = state.settings.load() {
                settings.stealth = !settings.stealth;
                state.overlay.set_stealth(settings.stealth);
                let _ = state.settings.save(&settings);
            }
        }
        ShortcutAction::ToggleSession => {
            if state.session.current_session_id().is_some() {
                if let Err(error) = state.session.stop() {
                    tracing::warn!("falha ao parar sessão pelo atalho: {error}");
                }
            } else if let Err(error) = state
                .session
                .start(&[AudioChannel::Microphone, AudioChannel::System])
            {
                tracing::warn!("falha ao iniciar sessão pelo atalho: {error}");
            }
        }
        ShortcutAction::RequestInsight => match state.insight.generate(None) {
            Ok(suggestion) => state.publisher.emit_insight(&suggestion),
            Err(error) => tracing::warn!("falha ao gerar insight pelo atalho: {error}"),
        },
    }
}

/// Identificador do app antes da renomeação (OpenSua → GhostWhisper).
const LEGACY_IDENTIFIER: &str = "com.opensua.desktop";
/// Nome do banco antes da renomeação.
const LEGACY_DB: &str = "opensua.db";

/// Move os dados do diretório antigo para o novo após a renomeação.
///
/// O diretório de dados deriva do *identifier*, então sem esta migração o app
/// perderia os modelos já baixados (centenas de MB), as configurações e o
/// histórico de sessões — tudo seria baixado de novo.
fn migrate_legacy_data_dir(dir: &std::path::Path) -> crate::error::Result<()> {
    std::fs::create_dir_all(dir)?;
    // Já migrado (banco novo existe) ou instalação nova: nada a fazer.
    if dir.join("ghostwhisper.db").exists() {
        return Ok(());
    }
    let Some(parent) = dir.parent() else {
        return Ok(());
    };
    let legacy = parent.join(LEGACY_IDENTIFIER);
    if !legacy.is_dir() {
        return Ok(());
    }

    // Modelos (arquivos grandes) e configurações.
    for name in ["models", "settings.json"] {
        let from = legacy.join(name);
        let to = dir.join(name);
        if !from.exists() || to.exists() {
            continue;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => tracing::info!("migrado do diretório anterior: {name}"),
            Err(error) => tracing::warn!("não foi possível migrar {name}: {error}"),
        }
    }

    // Banco: renomeado em vez de movido (fica no diretório antigo).
    let legacy_db = legacy.join(LEGACY_DB);
    if legacy_db.exists() {
        match std::fs::rename(&legacy_db, dir.join("ghostwhisper.db")) {
            Ok(()) => tracing::info!("banco migrado do diretório anterior"),
            Err(error) => tracing::warn!("não foi possível migrar o banco: {error}"),
        }
    }
    Ok(())
}

/// Configura o subsistema de logs (`RUST_LOG` controla o nível).
fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    /// Renomeação não pode custar o modelo baixado nem as configurações.
    #[test]
    fn migrates_legacy_data_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = tmp.path().join(LEGACY_IDENTIFIER);
        let models = legacy.join("models");
        std::fs::create_dir_all(&models).expect("mkdir models");
        std::fs::write(models.join("ggml-tiny.bin"), b"falso").expect("modelo");
        std::fs::write(legacy.join("settings.json"), "{}").expect("settings");
        std::fs::write(legacy.join(LEGACY_DB), b"sqlite").expect("banco");

        let dir = tmp.path().join("com.ghostwhisper.desktop");
        migrate_legacy_data_dir(&dir).expect("migra");

        assert_eq!(
            std::fs::read(dir.join("models/ggml-tiny.bin")).expect("modelo migrado"),
            b"falso"
        );
        assert!(dir.join("settings.json").exists(), "settings migrados");
        assert!(dir.join("ghostwhisper.db").exists(), "banco renomeado");
    }

    /// Instalações novas não-create nada legado e não falham.
    #[test]
    fn fresh_install_has_nothing_to_migrate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("com.ghostwhisper.desktop");
        migrate_legacy_data_dir(&dir).expect("migra");
        assert!(dir.is_dir());
        assert!(!dir.join("ghostwhisper.db").exists());
    }

    /// Migração é idempotente (não sobrescreve dados já existentes).
    #[test]
    fn migration_does_not_overwrite_existing_data() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = tmp.path().join(LEGACY_IDENTIFIER);
        std::fs::create_dir_all(&legacy).expect("mkdir");
        std::fs::write(legacy.join("settings.json"), "{\"antigo\":true}").expect("settings");

        let dir = tmp.path().join("com.ghostwhisper.desktop");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("settings.json"), "{\"novo\":true}").expect("settings");
        std::fs::write(dir.join("ghostwhisper.db"), b"sqlite").expect("banco");

        migrate_legacy_data_dir(&dir).expect("migra");
        let settings = std::fs::read_to_string(dir.join("settings.json")).expect("settings");
        assert!(settings.contains("novo"), "settings atuais preservados");
    }
}
