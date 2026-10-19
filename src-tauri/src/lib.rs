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

use tauri::{Emitter, Manager};

use domain::shortcuts::ShortcutAction;

pub use error::{GhostWhisperError, Result};

/// Inicializa e executa a aplicação Tauri.
pub fn run() {
    init_tracing();

    let app = tauri::Builder::default()
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
            ipc::settings_commands::settings_gpu_info,
            ipc::llm_commands::llm_translate,
            ipc::suggestion_commands::assistant_modes_list,
            ipc::suggestion_commands::suggestion_generate,
            ipc::suggestion_commands::suggestion_pin,
            ipc::suggestion_commands::llm_insight,
            ipc::summary_commands::summary_get,
            ipc::summary_commands::summary_generate,
            ipc::panel_commands::panel_open,
            ipc::panel_commands::panel_hide,
            ipc::panel_commands::panel_start_drag,
            ipc::panel_commands::panel_is_open,
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
            // Painéis: fechar esconde e avisa a UI.
            for label in domain::settings::PANEL_LABELS {
                if let Some(panel) = app.get_webview_window(label) {
                    let window = panel.clone();
                    panel.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = window.hide();
                            let _ = window
                                .emit(ipc::panel_commands::PANEL_HIDDEN_EVENT, label.to_string());
                        }
                    });
                }
            }
            // O tray é opcional: não derruba o app se o indicador faltar.
            if let Err(error) = build_tray(app) {
                tracing::warn!("bandeja indisponível: {error}");
            }
            spawn_overlay_position_tracker(app.handle().clone());
            for label in domain::settings::PANEL_LABELS {
                spawn_panel_position_tracker(app.handle().clone(), label);
            }
            spawn_suggestion_scheduler(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("falha ao construir o GhostWhisper");

    // Saída limpa: apaga a marca de GPU (BR-029). Um crash não passa por aqui, e
    // é justamente isso que o próximo boot detecta.
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(path) = gpu_marker_path(app_handle) {
                let _ = std::fs::remove_file(path);
            }
        }
    });
}

/// Arquivo-marca de "GPU ativa" (padrão Perssua: `whisper-gpu-active.json`).
const GPU_MARKER_FILE: &str = "whisper-gpu-active.json";

/// Caminho da marca de GPU no diretório de dados.
fn gpu_marker_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join(GPU_MARKER_FILE))
}

/// Grava a marca enquanto a GPU estiver habilitada nesta execução.
fn write_gpu_marker(data_dir: &std::path::Path, model: &str) {
    let payload = serde_json::json!({
        "pid": std::process::id(),
        "model": model,
        "started_at_ms": chrono::Utc::now().timestamp_millis(),
    });
    if let Err(error) = std::fs::write(data_dir.join(GPU_MARKER_FILE), payload.to_string()) {
        tracing::warn!("não foi possível gravar a marca de GPU: {error}");
    }
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
    use adapters::audio::shared_audio_reference::{SharedAudioReference, SystemAudioReferenceTap};
    use adapters::events::tauri_event_publisher::TauriEventPublisher;
    use adapters::overlay::cursor_click_through::CursorClickThroughController;
    use adapters::overlay::tauri_overlay_window::TauriOverlayWindow;
    use adapters::overlay::x11_input_shape_click_through::X11InputShapeClickThrough;
    use adapters::persistence::sqlite::connection as db_connection;
    use adapters::persistence::sqlite::local_meeting_summaries_repository::LocalMeetingSummariesRepository;
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
    use adapters::system::vulkan_gpu_probe::VulkanGpuProbe;
    use adapters::system::x11_pointer_source::{NullPointerSource, X11PointerSource};
    use application::llm_router::LlmRouter;
    use application::session_service::SessionService;
    use application::suggestion_service::SuggestionService;
    use application::summary_service::SummaryService;
    use application::translation_service::TranslationService;
    use ports::audio_monitor::AudioMonitor;
    use ports::audio_reference::AudioReference;
    use ports::audio_source::AudioSource;
    use ports::click_through::ClickThroughController;
    use ports::clock::Clock;
    use ports::event_publisher::EventPublisher;
    use ports::gpu_probe::GpuProbe;
    use ports::llm_controller::LlmController;
    use ports::llm_provider_factory::ChatBotProviderFactory;
    use ports::meeting_summaries_repository::MeetingSummariesRepository;
    use ports::model_store::ModelStore;
    use ports::overlay_window::OverlayWindowController;
    use ports::pointer_source::PointerSource;
    use ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
    use ports::secret_store::SecretStore;
    use ports::session_controller::SessionController;
    use ports::settings_store::SettingsStore;
    use ports::shortcut_registry::GlobalShortcutRegistry;
    use ports::speech_engine_factory::SpeechEngineFactory;
    use ports::suggestion_generator::SuggestionGenerator;
    use ports::summary_generator::SummaryGenerator;
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

    // Marca de crash de GPU (BR-029): se sobrou no disco, o app morreu enquanto a
    // GPU estava ativa — desliga a GPU nesta execução e limpa a marca.
    let gpu_marker = data_dir.join(GPU_MARKER_FILE);
    let force_cpu_after_crash = if gpu_marker.exists() {
        tracing::warn!("encerramento anterior com GPU ativa; desabilitando a GPU nesta execução");
        let _ = std::fs::remove_file(&gpu_marker);
        true
    } else {
        false
    };

    // Configurações primeiro: o estágio de ganho do microfone as lê na captura.
    let settings: Arc<dyn SettingsStore> =
        Arc::new(FileSettingsStore::new(data_dir.join("settings.json")));

    // Captura: o microfone passa pelo estágio de ganho (BR-018) e por um ponto
    // de escuta para o monitor atrasado (task 15); o áudio do sistema (loopback)
    // segue sem amplificação.
    let monitor: Arc<dyn AudioMonitor> = Arc::new(PulseDelayedMonitor::new());
    // Referência do AEC (BR-029): o áudio do sistema (loopback) é guardado numa
    // janela recente e consumido pelo cancelador de eco do microfone.
    let reference: Arc<dyn AudioReference> = Arc::new(SharedAudioReference::new());
    let audio: Arc<dyn AudioSource> = Arc::new(MonitoredAudioSource::new(
        Arc::new(
            MicrophoneFrontEnd::new(Arc::new(CpalMicrophoneSource::new()), Arc::clone(&settings))
                .with_reference(Arc::clone(&reference)),
        ),
        Arc::clone(&monitor),
    ));
    let system_audio: Arc<dyn AudioSource> = Arc::new(SystemAudioReferenceTap::new(
        Arc::new(PulseSystemAudioSource::new()),
        Arc::clone(&reference),
    ));
    let virtual_sink: Arc<dyn VirtualSinkManager> = Arc::new(PactlVirtualSinkManager);

    let models: Arc<dyn ModelStore> = Arc::new(FileModelStore::new(data_dir.join("models")));
    let publisher: Arc<dyn EventPublisher> =
        Arc::new(TauriEventPublisher::new(app.handle().clone()));

    // Detecção de GPU (BR-027): informa a UI para sugerir o modelo local. Loga
    // uma vez no boot para facilitar o diagnóstico.
    let gpu: Arc<dyn GpuProbe> = Arc::new(VulkanGpuProbe::new());
    let gpu_info = gpu.probe();
    tracing::info!(
        available = gpu_info.available,
        backend = %gpu_info.backend,
        acceleration_enabled = gpu_info.acceleration_enabled,
        recommended_model = %gpu_info.recommended_model,
        "detecção de GPU"
    );

    // Banco de dados (SQLite) e repositórios.
    let connection = db_connection::shared(db_connection::open(&data_dir.join("ghostwhisper.db"))?);
    let sessions: Arc<dyn SessionRepository> =
        Arc::new(LocalSessionRepository::new(Arc::clone(&connection)));
    let transcriptions: Arc<dyn TranscriptionsRepository> =
        Arc::new(LocalTranscriptionsRepository::new(Arc::clone(&connection)));
    let messages: Arc<dyn MessagesRepository> =
        Arc::new(LocalMessagesRepository::new(Arc::clone(&connection)));
    let summaries: Arc<dyn MeetingSummariesRepository> = Arc::new(
        LocalMeetingSummariesRepository::new(Arc::clone(&connection)),
    );
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

    // Casos de uso de LLM (tradução, resumo e sugestões).
    let translator: Arc<dyn Translator> = Arc::new(TranslationService::new(
        Arc::clone(&llm),
        Arc::clone(&settings),
    ));
    let summary: Arc<dyn SummaryGenerator> = Arc::new(SummaryService::new(
        Arc::clone(&llm),
        Arc::clone(&settings),
        Arc::clone(&sessions),
        Arc::clone(&transcriptions),
        Arc::clone(&summaries),
        Arc::clone(&clock),
    ));
    let suggestions: Arc<dyn SuggestionGenerator> = Arc::new(SuggestionService::new(
        Arc::clone(&llm),
        Arc::clone(&settings),
        Arc::clone(&sessions),
        Arc::clone(&transcriptions),
        Arc::clone(&messages),
        Arc::clone(&clock),
        Arc::clone(&publisher),
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
    // Painéis: reaplica a última posição e o último tamanho (px físicos).
    for label in domain::settings::PANEL_LABELS {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        if let Some((x, y)) = current_settings.panel_position(label) {
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
        if let Some((width, height)) = current_settings.panel_size(label) {
            let _ = window.set_size(tauri::PhysicalSize::new(width, height));
        }
    }

    // Motor de fala por canal: cada pipeline recebe seu próprio contexto.
    // Modelo e idioma do whisper vêm das configurações (lidas a cada sessão).
    let engines: Arc<dyn SpeechEngineFactory> = Arc::new(
        WhisperSpeechEngineFactory::new(Arc::clone(&models), Arc::clone(&settings))
            .with_publisher(Arc::clone(&publisher))
            .with_force_cpu(force_cpu_after_crash),
    );

    // GPU ligada nesta execução ⇒ marca no disco até a saída limpa (BR-029).
    if gpu_info.acceleration_enabled
        && !force_cpu_after_crash
        && current_settings.acceleration != "cpu"
    {
        write_gpu_marker(&data_dir, &current_settings.whisper_model);
    }

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
        gpu,
        llm,
        translator,
        summary,
        suggestions,
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
    let summary =
        MenuItem::with_id(app, "summary", "Abrir resumo", true, None::<&str>).map_err(to_err)?;
    let suggestions = MenuItem::with_id(app, "suggestions", "Abrir sugestões", true, None::<&str>)
        .map_err(to_err)?;
    let chat = MenuItem::with_id(app, "chat", "Abrir chat", true, None::<&str>).map_err(to_err)?;
    let history =
        MenuItem::with_id(app, "history", "Abrir histórico", true, None::<&str>).map_err(to_err)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>).map_err(to_err)?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &session,
            &summary,
            &suggestions,
            &chat,
            &history,
            &settings,
            &quit,
        ],
    )
    .map_err(to_err)?;

    let mut builder = TrayIconBuilder::with_id("ghostwhisper-tray")
        .menu(&menu)
        .tooltip("GhostWhisper")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle_overlay" => dispatch_shortcut(app, ShortcutAction::ToggleOverlay),
            "toggle_session" => dispatch_shortcut(app, ShortcutAction::ToggleSession),
            "settings" => open_settings(app),
            "summary" => open_panel(app, "summary"),
            "suggestions" => open_panel(app, "suggestions"),
            "chat" => open_panel(app, "chat"),
            "history" => open_panel(app, "history"),
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

/// Abre (ou foca) uma janela de painel a partir do tray.
fn open_panel(app: &tauri::AppHandle, label: &str) {
    if let Err(error) = ipc::panel_commands::panel_open(app.clone(), label.to_string()) {
        tracing::warn!("falha ao abrir painel {label}: {error}");
    }
}

/// Intervalo de verificação do agendador de sugestões (ms).
const SUGGESTION_TICK_MS: u64 = 1_000;
/// Espera depois do início da última fala finalizada antes de sugerir.
const SUGGESTION_DEBOUNCE_MS: i64 = 700;
/// Frequência máxima das sugestões automáticas (limita custo de API).
const SUGGESTION_MIN_INTERVAL_MS: i64 = 3_000;

/// Gera sugestões automáticas **sem depender da janela aberta** (BR-028).
///
/// Roda no composition root: a decisão é pura
/// ([`application::suggestion_scheduler::should_trigger`]) e aqui só há o polling
/// e a chamada ao caso de uso. A sugestão é efêmera e chega à UI pelo evento
/// `suggestion`.
fn spawn_suggestion_scheduler(app: tauri::AppHandle) {
    use application::suggestion_scheduler::{should_trigger, SchedulerTick};
    use ports::suggestion_generator::SuggestionRequest;

    std::thread::spawn(move || {
        let mut cursor: Option<i64> = None;
        let mut current_session: Option<String> = None;
        let mut last_run: Option<std::time::Instant> = None;
        loop {
            std::thread::sleep(Duration::from_millis(SUGGESTION_TICK_MS));
            let state = app.state::<state::AppState>();
            let Ok(settings) = state.settings.load() else {
                continue;
            };
            let session_id = state.session.current_session_id();
            if session_id != current_session {
                current_session = session_id.clone();
                cursor = None;
            }
            let Some(session_id) = session_id else {
                continue;
            };
            let Ok(Some(session)) = state.sessions.get(&session_id) else {
                continue;
            };
            let latest_final_ms = state
                .transcriptions
                .latest_final_ms(&session_id)
                .unwrap_or(None);
            let now_rel = (state.clock.now_ms() - session.started_at_ms).max(0);
            let tick = SchedulerTick {
                enabled: settings.suggestion_auto_refresh,
                has_session: true,
                cursor_ms: cursor,
                latest_final_ms,
                ms_since_last_run: last_run
                    .map(|instant| instant.elapsed().as_millis() as i64)
                    .unwrap_or(i64::MAX),
                ms_since_latest_final: latest_final_ms
                    .map(|latest| now_rel - latest)
                    .unwrap_or(i64::MAX),
            };
            if !should_trigger(&tick, SUGGESTION_DEBOUNCE_MS, SUGGESTION_MIN_INTERVAL_MS) {
                continue;
            }
            last_run = Some(std::time::Instant::now());
            let request =
                SuggestionRequest::ephemeral(Some(session_id), settings.assistant_mode_id.clone())
                    .with_since_ms(cursor)
                    .with_fast_first(true);
            match state.suggestions.generate(request) {
                Ok(suggestion) if !suggestion.skipped => {
                    cursor = Some(suggestion.context_until_ms);
                }
                // `skipped` = nada novo; erro (sem chave, provider fora) não deve
                // repetir a mesma fala a cada tick: avança o cursor e segue.
                Ok(_) => {}
                Err(error) => {
                    tracing::debug!("sugestão automática indisponível: {error}");
                    cursor = latest_final_ms;
                }
            }
        }
    });
}

/// Persiste a posição das janelas de painel enquanto o usuário as move.
fn spawn_panel_position_tracker(app: tauri::AppHandle, label: &'static str) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(900));

        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        if !window.is_visible().unwrap_or(false) {
            continue;
        }
        let Ok(position) = window.outer_position() else {
            continue;
        };
        let Ok(size) = window.inner_size() else {
            continue;
        };

        let state = app.state::<state::AppState>();
        let Ok(mut settings) = state.settings.load() else {
            continue;
        };
        let current = (position.x, position.y);
        let current_size = (size.width, size.height);
        let mut changed = false;
        if settings.panel_position(label) != Some(current) {
            settings.set_panel_position(label, Some(current));
            changed = true;
        }
        if settings.panel_size(label) != Some(current_size) {
            settings.set_panel_size(label, Some(current_size));
            changed = true;
        }
        if changed {
            let _ = state.settings.save(&settings);
        }
    });
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
    match action {
        // Rápidos e ligados a janela/arquivo pequeno (alguns ms): seguem na thread
        // que recebeu o evento, sem custo de thread nova.
        ShortcutAction::ToggleOverlay | ShortcutAction::ToggleStealth => run_shortcut(app, action),
        // Demorados: o menu da bandeja roda na **main thread** do GTK e o listener
        // de atalhos do X11 na thread dele. Parar a sessão espera o passe final do
        // turno (até 20 s) e pedir insight faz HTTP bloqueante (até 60 s) — na main
        // thread isso congela a interface; no listener, engole os próximos atalhos
        // (ISS-020). Thread própria para os dois.
        ShortcutAction::ToggleSession | ShortcutAction::RequestInsight => {
            let app = app.clone();
            std::thread::spawn(move || run_shortcut(&app, action));
        }
    }
}

/// Executa a ação do atalho (pode demorar; chame pela thread certa).
fn run_shortcut(app: &tauri::AppHandle, action: ShortcutAction) {
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
        ShortcutAction::RequestInsight => {
            let request = crate::ports::suggestion_generator::SuggestionRequest::persist(None);
            match state.suggestions.generate(request) {
                Ok(suggestion) => state.publisher.emit_insight(&suggestion),
                Err(error) => tracing::warn!("falha ao gerar sugestão pelo atalho: {error}"),
            }
        }
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
