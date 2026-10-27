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

use std::path::PathBuf;
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
            ipc::log_commands::logs_path,
            ipc::log_commands::logs_recent,
            ipc::overlay::overlay_toggle,
            ipc::overlay::overlay_set_stealth,
            ipc::overlay::overlay_set_click_through,
            ipc::overlay::overlay_set_interactive_regions,
            ipc::overlay::overlay_set_surface,
            ipc::overlay::overlay_set_actionbar_width,
            ipc::overlay::overlay_start_drag,
            ipc::menu::menu_open,
            ipc::menu::menu_resize,
            ipc::menu::menu_close,
            ipc::menu::menu_state,
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
            ipc::session_commands::session_set_language,
            ipc::ask_commands::ask_open,
            ipc::ask_commands::ask_hide,
            ipc::ask_commands::ask_start_drag,
            ipc::ask_commands::ask_is_open,
            ipc::ask_commands::ask_focus,
            ipc::screen_commands::screen_capture,
            ipc::screen_commands::capture_pending,
            ipc::screen_commands::capture_clear,
            ipc::journal_commands::journal_list,
            ipc::journal_commands::journal_get,
            ipc::journal_commands::journal_search,
            ipc::journal_commands::journal_images,
            ipc::journal_commands::journal_delete,
            ipc::journal_commands::journal_ask,
            ipc::journal_commands::journal_cancel,
            ipc::database_commands::db_list_sessions,
            ipc::database_commands::db_get_session,
            ipc::database_commands::db_session_transcriptions,
            ipc::database_commands::db_session_messages,
            ipc::database_commands::db_session_timeline,
            ipc::database_commands::db_search_transcriptions,
            ipc::session_commands::session_rename,
            ipc::session_commands::session_delete,
            ipc::recording_commands::recording_capabilities,
            ipc::recording_commands::recording_start,
            ipc::recording_commands::recording_pause,
            ipc::recording_commands::recording_resume,
            ipc::recording_commands::recording_stop,
            ipc::recording_commands::recording_state,
            ipc::recording_commands::recording_discard,
            ipc::recording_commands::recording_switch_target,
            ipc::recording_commands::recording_targets,
            ipc::recording_commands::recording_picker_open,
            ipc::recording_commands::recording_picker_close,
            ipc::session_end_commands::session_end_open,
            ipc::session_end_commands::session_end_context,
            ipc::session_end_commands::session_end_close,
            ipc::export_commands::session_export_markdown,
            ipc::export_commands::session_save_markdown,
            ipc::settings_commands::settings_open,
            ipc::settings_commands::settings_close,
            ipc::settings_commands::settings_providers,
            ipc::settings_commands::settings_get,
            ipc::settings_commands::settings_defaults,
            ipc::settings_commands::settings_set,
            ipc::settings_commands::settings_set_assistant_mode,
            ipc::settings_commands::settings_preview_overlay_opacity,
            ipc::settings_commands::settings_set_overlay_opacity,
            ipc::settings_commands::settings_preview_font_scales,
            ipc::settings_commands::settings_set_font_scales,
            ipc::settings_commands::settings_validate_key,
            ipc::settings_commands::settings_provider_label,
            ipc::settings_commands::settings_secrets_persistent,
            ipc::settings_commands::settings_gpu_info,
            ipc::ai_source_commands::ai_sources_list,
            ipc::ai_source_commands::ai_sources_health,
            ipc::ai_source_commands::ai_source_set_key,
            ipc::ai_source_commands::ai_source_has_key,
            ipc::ai_source_commands::ai_source_clear_key,
            ipc::ai_source_commands::ai_source_key_preview,
            ipc::ai_source_commands::ai_source_validate,
            ipc::llm_commands::llm_translate,
            ipc::suggestion_commands::assistant_modes_list,
            ipc::suggestion_commands::suggestion_generate,
            ipc::suggestion_commands::suggestion_pin,
            ipc::suggestion_commands::suggestion_prepare_manual,
            ipc::suggestion_commands::suggestion_manual_context,
            ipc::suggestion_commands::suggestion_clear_manual_context,
            ipc::suggestion_commands::suggestion_last,
            ipc::suggestion_commands::suggestion_classification_last,
            ipc::suggestion_commands::suggestion_classify_now,
            ipc::model_commands::classifier_backend_status,
            ipc::model_commands::classifier_model_catalog,
            ipc::model_commands::classifier_model_installed,
            ipc::model_commands::classifier_model_active,
            ipc::model_commands::classifier_model_install,
            ipc::model_commands::classifier_model_activate,
            ipc::model_commands::classifier_model_remove,
            ipc::suggestion_commands::llm_insight,
            ipc::summary_commands::summary_get,
            ipc::summary_commands::summary_generate,
            ipc::panel_commands::panel_open,
            ipc::panel_commands::panel_hide,
            ipc::panel_commands::panel_is_open,
            ipc::window_commands::window_start_drag,
            ipc::window_commands::window_start_resize,
            ipc::shortcuts_commands::shortcuts_get,
            ipc::shortcuts_commands::shortcuts_active,
            ipc::shortcuts_commands::shortcuts_set,
            ipc::shortcuts_commands::shortcuts_catalog,
        ])
        .setup(|app| {
            let app_state = build_app_state(app)?;
            app.manage(app_state);
            app.manage(ipc::ask_commands::AskDragState::default());
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
            // Seletor de janelas: fechar esconde.
            if let Some(picker) = app.get_webview_window("recording-picker") {
                let window = picker.clone();
                picker.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });
            }
            // Modal de fim de sessão: fechar esconde (nunca destrói), para
            // poder ser reaberto na próxima sessão (BR-068).
            if let Some(session_end) = app.get_webview_window("session-end") {
                let window = session_end.clone();
                session_end.on_window_event(move |event| {
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
            spawn_ask_position_tracker(app.handle().clone());
            // ISS-043: o GTK/Mutter pode mapear o overlay **sem** o
            // `_NET_WM_STATE_ABOVE`; reafirma logo após o boot.
            spawn_overlay_stacking_reassertion(app.handle().clone());
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

/// Modelo Laya empacotado nos resources (AppImage/deb), se existir.
#[cfg(feature = "laya")]
///
/// O mapa `bundle.resources` copia `resources/laya/multilingual` para
/// `<resource_dir>/laya/multilingual`; é o último fallback depois do override
/// de dev e da versão ativa do model manager.
fn bundled_laya_model(app: &tauri::App) -> Option<PathBuf> {
    let directory = app
        .path()
        .resource_dir()
        .ok()?
        .join("laya")
        .join("multilingual");
    directory.join("model.onnx").is_file().then_some(directory)
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
    use adapters::manual::in_memory_manual_suggestion_context_store::InMemoryManualSuggestionContextStore;
    use adapters::menu::tauri_menu_window::TauriMenuWindow;
    use adapters::overlay::cursor_click_through::CursorClickThroughController;
    use adapters::overlay::tauri_overlay_window::TauriOverlayWindow;
    use adapters::overlay::x11_input_shape_click_through::X11InputShapeClickThrough;
    use adapters::pending::in_memory_pending_session_end::InMemoryPendingSessionEnd;
    use adapters::persistence::sqlite::connection as db_connection;
    use adapters::persistence::sqlite::local_journal_repository::LocalJournalRepository;
    use adapters::persistence::sqlite::local_meeting_summaries_repository::LocalMeetingSummariesRepository;
    use adapters::persistence::sqlite::local_messages_repository::LocalMessagesRepository;
    use adapters::persistence::sqlite::local_recordings_repository::LocalRecordingsRepository;
    use adapters::persistence::sqlite::local_session_repository::LocalSessionRepository;
    use adapters::persistence::sqlite::local_transcriptions_repository::LocalTranscriptionsRepository;
    use adapters::recording::ffmpeg_path::resolve_ffmpeg;
    use adapters::recording::ffmpeg_screen_recorder::FfmpegScreenRecorder;
    use adapters::recording::finalizer::FfmpegRecordingFinalizer;
    use adapters::recording::x11_targets::X11ScreenTargets;
    use adapters::screen::x11_screen_capture::X11ScreenCapture;
    use adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use adapters::secrets::keyring_secret_store::KeyringSecretStore;
    use adapters::settings::file_settings_store::FileSettingsStore;
    use adapters::shortcuts::tauri_global_shortcut_registry::{
        ShortcutHandler, TauriGlobalShortcutRegistry,
    };
    use adapters::speech::whisper_model_store::FileModelStore;
    use adapters::speech::whisper_speech_engine_factory::WhisperSpeechEngineFactory;
    use adapters::system::env_locale_source::EnvLocaleSource;
    use adapters::system::system_clock::SystemClock;
    use adapters::system::vulkan_gpu_probe::VulkanGpuProbe;
    use adapters::system::x11_pointer_source::{NullPointerSource, X11PointerSource};
    use application::ask_service::AskService;
    use application::llm_router::LlmRouter;
    use application::recording_service::RecordingService;
    use application::session_service::SessionService;
    use application::suggestion_service::SuggestionService;
    use application::summary_service::SummaryService;
    use application::translation_service::TranslationService;
    use ports::ask_controller::AskController;
    use ports::audio_monitor::AudioMonitor;
    use ports::audio_reference::AudioReference;
    use ports::audio_source::AudioSource;
    use ports::click_through::ClickThroughController;
    use ports::clock::Clock;
    use ports::event_publisher::EventPublisher;
    use ports::gpu_probe::GpuProbe;
    use ports::journal_repository::JournalRepository;
    use ports::llm_controller::LlmController;
    use ports::llm_provider_factory::ChatBotProviderFactory;
    use ports::locale_source::LocaleSource;
    use ports::manual_suggestion_context::ManualSuggestionContextStore;
    use ports::meeting_summaries_repository::MeetingSummariesRepository;
    use ports::menu_window::MenuWindowController;
    use ports::model_store::ModelStore;
    use ports::overlay_window::OverlayWindowController;
    use ports::pending_session_end::PendingSessionEnd;
    use ports::pointer_source::PointerSource;
    use ports::recording::{RecordingController, RecordingLifecycle};
    use ports::recordings_repository::RecordingsRepository;
    use ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
    use ports::screen_capture::ScreenCapture;
    use ports::screen_recorder::{RecordingFinalizer, ScreenRecorder};
    use ports::screen_targets::ScreenTargets;
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

    // Menu flutuante (agentes/canais) em janela própria: o notch não muda de
    // tamanho nem de posição ao abrir/fechar (BR-072/ISS-063).
    let menu_window = app
        .get_webview_window("menu")
        .ok_or_else(|| GhostWhisperError::Other("janela 'menu' não encontrada".to_string()))?;
    let menu: Arc<dyn MenuWindowController> =
        Arc::new(TauriMenuWindow::new(menu_window, overlay_window.clone()));

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

    // Idioma da interface (BR-085): resolve o padrão do sistema **uma vez** e
    // persiste. Depois disso, a escolha do usuário nunca é sobrescrita.
    let locales: Arc<dyn LocaleSource> = Arc::new(EnvLocaleSource);
    match application::ui_language_migration::UiLanguageMigration::new(
        Arc::clone(&settings),
        locales,
    )
    .migrate()
    {
        Ok(outcome) => {
            if outcome.resolved {
                tracing::info!(
                    language = outcome.language.as_deref().unwrap_or("?"),
                    "idioma da interface resolvido pelo sistema"
                );
            }
        }
        Err(error) => tracing::warn!("falha ao resolver o idioma da interface: {error}"),
    }

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
    let journal: Arc<dyn JournalRepository> =
        Arc::new(LocalJournalRepository::new(Arc::clone(&connection)));
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

    // Migração das configurações/chaves anteriores às fontes de IA (BR-044):
    // `provider`/`model`/`base_url` viram uma fonte e a chave legada
    // (`llm.openai`) é copiada para `llm.source.{id}`. Idempotente.
    match application::ai_source_migration::AiSourceMigration::new(
        Arc::clone(&settings),
        Arc::clone(&secrets),
    )
    .migrate()
    {
        Ok(outcome) => {
            if outcome.source_created || outcome.key_copied {
                tracing::info!(
                    source_created = outcome.source_created,
                    key_copied = outcome.key_copied,
                    "configuração de IA migrada para fontes"
                );
            }
        }
        Err(error) => tracing::warn!("falha ao migrar as fontes de IA: {error}"),
    }

    // Migração do agente padrão (BR-059): o agente **Padrão** (contextual)
    // substitui a "Reunião" como padrão — uma vez só, para não sobrescrever a
    // escolha do usuário depois.
    match application::assistant_mode_migration::AssistantModeMigration::new(Arc::clone(&settings))
        .migrate()
    {
        Ok(outcome) => {
            if outcome.mode_changed {
                tracing::info!("agente padrão migrado para o agente Padrão");
            }
        }
        Err(error) => tracing::warn!("falha ao migrar o agente padrão: {error}"),
    }

    // Semeadura única dos agentes de fábrica (BR-075): os quatro viram agentes
    // do usuário (editáveis e excluíveis); o Padrão continua protegido.
    match application::assistant_catalog_migration::AssistantCatalogMigration::new(Arc::clone(
        &settings,
    ))
    .seed()
    {
        Ok(outcome) => {
            if outcome.upgraded {
                tracing::info!("catálogo de agentes atualizado");
            }
            if !outcome.added.is_empty() {
                tracing::info!(
                    agents = ?outcome.added,
                    "agentes de fábrica materializados na lista do usuário"
                );
            }
        }
        Err(error) => tracing::warn!("falha ao semear os agentes de fábrica: {error}"),
    }

    // Preset novo das perguntas prontas (BR-054): entra uma vez em quem já
    // tinha a lista persistida, sem ressuscitar as que o usuário apagou.
    match application::saved_questions_migration::SavedQuestionsMigration::new(Arc::clone(
        &settings,
    ))
    .migrate()
    {
        Ok(outcome) => {
            if outcome.question_added {
                tracing::info!("pergunta pronta nova adicionada");
            }
        }
        Err(error) => tracing::warn!("falha ao migrar as perguntas prontas: {error}"),
    }

    // Registro de saúde + circuit breaker das fontes (BR-045): compartilhado
    // entre o roteador e o serviço de fontes.
    let ai_source_health = Arc::new(application::ai_source_health::AiSourceHealthRegistry::new(
        Arc::clone(&clock),
    ));
    let ai_sources: Arc<dyn ports::ai_source_service::AiSourceService> = Arc::new(
        application::ai_source_service::LocalAiSourceService::with_publisher(
            Arc::clone(&settings),
            Arc::clone(&secrets),
            Arc::clone(&factory),
            Arc::clone(&ai_source_health),
            Arc::clone(&clock),
            Arc::clone(&publisher),
        ),
    );
    let llm: Arc<dyn LlmController> = Arc::new(LlmRouter::with_telemetry(
        factory,
        Arc::clone(&settings),
        Arc::clone(&secrets),
        ai_source_health,
        Arc::clone(&clock),
        Arc::clone(&publisher),
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
    let manual_context: Arc<dyn ManualSuggestionContextStore> =
        Arc::new(InMemoryManualSuggestionContextStore::new());

    // Captura de tela (print para a pergunta, BR-055). O print fica **pendente**
    // até o usuário confirmar a pergunta; a prévia no canto lê o mesmo estado.
    let screen: Arc<dyn ScreenCapture> = Arc::new(X11ScreenCapture::new());
    let capture: Arc<dyn crate::ports::pending_capture::PendingCapture> = Arc::new(
        crate::adapters::pending::in_memory_pending_capture::InMemoryPendingCapture::new(),
    );

    // Perguntas do usuário (aba "Sessão", BR-053).
    let ask: Arc<dyn AskController> = Arc::new(AskService::new(
        Arc::clone(&llm),
        Arc::clone(&sessions),
        Arc::clone(&transcriptions),
        Arc::clone(&journal),
        Arc::clone(&summary),
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
    // HUD (BR-062): densidade e ancoragem. "Flutuante Livre" restaura a
    // posição lembrada; as âncoras fixas posicionam na área útil do monitor.
    let density = crate::domain::hud_layout::HudDensity::parse(&current_settings.hud_density);
    let anchor = crate::domain::hud_layout::NotchAnchor::parse(&current_settings.notch_anchor);
    overlay.set_hud_layout(density, anchor);
    if !anchor.is_anchored() {
        let saved_position = current_settings
            .overlay_position
            .filter(|(x, y)| *x >= 0 && *y >= 0);
        if let Some((x, y)) = saved_position {
            let _ = overlay_window.set_position(tauri::PhysicalPosition::new(x, y));
            overlay.set_anchor_bottom(false);
        }
    }
    // Barra "Perguntar": restaura a posição arrastada; só centraliza na
    // primeira execução. Abrir/fechar/capturar não a move (BR-072).
    crate::ipc::ask_commands::restore_ask_window_position(
        app.handle(),
        current_settings.ask_position,
    );

    // Prévia do print: canto **superior direito** (BR-055).
    if let Some(window) = app.get_webview_window("capture") {
        if let Ok(Some(monitor)) = app.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let width = (PREVIEW_WIDTH * scale).round() as i32;
            let margin = (16.0 * scale).round() as i32;
            let x = size.width as i32 - width - margin;
            let _ = window.set_position(tauri::PhysicalPosition::new(x, margin));
        }
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

    // Gravação da sessão (BR-065…BR-071): adaptador `ffmpeg` atrás da porta.
    let recordings: Arc<dyn RecordingsRepository> =
        Arc::new(LocalRecordingsRepository::new(Arc::clone(&connection)));
    // Gravação interrompida por um crash: marca para recuperação no modal
    // (CA-02.5). Os segmentos em disco são preservados.
    match recordings.list_unfinished() {
        Ok(items) => {
            for item in items {
                let _ = recordings.update_status(
                    &item.id,
                    crate::domain::recording::RecordingStatus::Interrupted,
                    Some("app encerrado durante a gravação"),
                );
            }
        }
        Err(error) => tracing::warn!("falha ao recuperar gravações: {error}"),
    }
    let ffmpeg_path = match resolve_ffmpeg() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!("gravação indisponível: {error}");
            PathBuf::from("ffmpeg")
        }
    };
    let recorder: Arc<dyn ScreenRecorder> =
        Arc::new(FfmpegScreenRecorder::new(ffmpeg_path.clone()));
    let finalizer: Arc<dyn RecordingFinalizer> =
        Arc::new(FfmpegRecordingFinalizer::new(ffmpeg_path));
    let targets: Arc<dyn ScreenTargets> = Arc::new(X11ScreenTargets::new());
    let recordings_dir = app
        .path()
        .video_dir()
        .map(|dir| dir.join("GhostWhisper"))
        .unwrap_or_else(|_| data_dir.join("recordings"));
    let recording_impl = Arc::new(RecordingService::new(
        recorder,
        finalizer,
        targets,
        recordings,
        Arc::clone(&publisher),
        Arc::clone(&settings),
        Arc::clone(&clock),
        Arc::clone(&sessions),
        recordings_dir,
    ));
    let recording: Arc<dyn RecordingController> = recording_impl.clone();
    let recording_lifecycle: Arc<dyn RecordingLifecycle> = recording_impl.clone();
    let pending_session_end: Arc<dyn PendingSessionEnd> =
        Arc::new(InMemoryPendingSessionEnd::new());

    let session: Arc<dyn SessionController> = Arc::new(
        SessionService::new(
            Arc::clone(&audio),
            Arc::clone(&system_audio),
            engines,
            Arc::clone(&publisher),
            Arc::clone(&sessions),
            Arc::clone(&transcriptions),
            Arc::clone(&clock),
        )
        .with_recording(recording_lifecycle),
    );

    // Model manager do classificador (BR-099): o artefato ONNX vive em
    // `app_data/models/laya` e é baixado sob ação explícita.
    let classifier_models: Arc<dyn ports::classifier_model_manager::ClassifierModelManager> =
        Arc::new(
            adapters::models::local_classifier_model_manager::LocalClassifierModelManager::new(
                data_dir.join("models").join("laya"),
                Arc::clone(&settings),
                Arc::new(
                    adapters::models::local_classifier_model_manager::UreqClassifierModelSource,
                ),
                Arc::clone(&publisher),
                Arc::clone(&clock),
            ),
        );

    // Gate por blocos (BR-093/BR-094). O seletor aplica o backend salvo nas
    // settings sem reiniciar: `heuristic` hoje; `laya` quando o engine ONNX
    // estiver disponível (task 06 — bloqueada no artefato/crate publicado).
    let heuristic_classifier: Arc<dyn ports::suggestion_cue_classifier::SuggestionCueClassifier> =
        Arc::new(
            adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier::default(),
        );
    #[cfg(feature = "laya")]
    let laya_classifier: Option<
        Arc<dyn ports::suggestion_cue_classifier::SuggestionCueClassifier>,
    > = {
        use adapters::ai::laya_onnx_predictor::OnnxLayaPredictor;
        use adapters::ai::laya_suggestion_cue_classifier::{
            resolve_model_dir, LayaClassifierConfig, LayaSuggestionCueClassifier,
        };

        let predictor = resolve_model_dir(
            &current_settings.suggestion_classifier_model_path,
            classifier_models.active_path(),
            bundled_laya_model(app),
        )
        .ok()
        .and_then(|directory| match OnnxLayaPredictor::new(directory) {
            Ok(predictor) => Some(predictor),
            Err(error) => {
                tracing::warn!("backend Laya indisponível: {error}");
                None
            }
        });

        predictor.map(|predictor| {
            let classifier = Arc::new(LayaSuggestionCueClassifier::new(
                Box::new(predictor),
                LayaClassifierConfig::from_settings(&current_settings),
            ));
            let warm = Arc::clone(&classifier);
            std::thread::spawn(move || warm.warm_up());
            classifier as Arc<dyn ports::suggestion_cue_classifier::SuggestionCueClassifier>
        })
    };
    #[cfg(not(feature = "laya"))]
    let laya_classifier: Option<
        Arc<dyn ports::suggestion_cue_classifier::SuggestionCueClassifier>,
    > = None;
    let classifier: Arc<dyn ports::suggestion_cue_classifier::SuggestionCueClassifier> = Arc::new(
        adapters::ai::settings_suggestion_cue_classifier::SettingsSuggestionCueClassifier::new(
            Arc::clone(&settings),
            heuristic_classifier,
            laya_classifier,
        ),
    );
    let suggestion_gate: Arc<dyn ports::suggestion_gate::SuggestionGate> = Arc::new(
        application::suggestion_gate_service::SuggestionGateService::new(
            classifier,
            Arc::clone(&transcriptions),
            Arc::clone(&sessions),
            Arc::clone(&clock),
            Arc::clone(&publisher),
            application::suggestion_gate_service::config_from_settings(&current_settings),
        ),
    );

    Ok(state::AppState::new(
        overlay,
        click_through,
        menu,
        audio,
        system_audio,
        monitor,
        virtual_sink,
        session,
        sessions,
        transcriptions,
        messages,
        journal,
        ask,
        screen,
        capture,
        clock,
        settings,
        secrets,
        gpu,
        llm,
        ai_sources,
        translator,
        summary,
        suggestions,
        suggestion_gate,
        classifier_models,
        manual_context,
        publisher,
        shortcuts,
        recording,
        pending_session_end,
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
    let chat =
        MenuItem::with_id(app, "chat", "Abrir transcrição", true, None::<&str>).map_err(to_err)?;
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
            "summary" => {
                // O resumo virou aba do painel `chat` (BR-033).
                open_panel_tab(app, "chat", "summary");
            }
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
    if let Err(error) = ipc::window_visibility::show_above(&window) {
        tracing::warn!("falha ao mostrar configurações: {error}");
    }
    if let Err(error) = window.set_focus() {
        tracing::warn!("falha ao focar configurações: {error}");
    }
}

/// Abre (ou foca) uma janela de painel a partir do tray.
fn open_panel(app: &tauri::AppHandle, label: &str) {
    if let Err(error) = ipc::panel_commands::panel_open(app.clone(), label.to_string(), None) {
        tracing::warn!("falha ao abrir painel {label}: {error}");
    }
}

/// Abre o painel já numa aba (ex.: tray "Abrir resumo").
fn open_panel_tab(app: &tauri::AppHandle, label: &str, tab: &str) {
    if let Err(error) =
        ipc::panel_commands::panel_open(app.clone(), label.to_string(), Some(tab.to_string()))
    {
        tracing::warn!("falha ao abrir painel {label} na aba {tab}: {error}");
    }
}

/// Intervalo de verificação do agendador de sugestões (ms).
const SUGGESTION_TICK_MS: u64 = 1_000;
/// Piso do intervalo do auto-sugerir (ms).
///
/// O intervalo real vem das settings (`suggestion_interval_secs`, 10–120 s) —
/// é o **cooldown** mostrado na contagem regressiva (BR-039).
const SUGGESTION_MIN_INTERVAL_MS: i64 = 1_000;

/// Gera sugestões automáticas com a janela de Sugestões **aberta** (BR-028).
///
/// Roda no composition root: a decisão é pura
/// ([`application::suggestion_scheduler::should_trigger`]) e aqui só há o polling,
/// a checagem da janela e a chamada ao caso de uso. A sugestão é efêmera e chega
/// à UI pelo evento `suggestion`.
fn spawn_suggestion_scheduler(app: tauri::AppHandle) {
    use application::suggestion_scheduler::{should_trigger, SchedulerTick};
    use domain::suggestion_gate::GateDecision;
    use ports::event_publisher::{SuggestionClassificationEvent, SuggestionClassifierState};
    use ports::suggestion_generator::{SuggestionRequest, SuggestionTrigger};

    std::thread::spawn(move || {
        let mut cursor: Option<i64> = None;
        let mut current_session: Option<String> = None;
        // Início do ciclo atual (cooldown): última geração, abertura da janela,
        // sessão nova ou reativação do automático.
        let mut cycle_started = std::time::Instant::now();
        let mut window_was_open = false;
        let mut auto_was_enabled = false;
        loop {
            std::thread::sleep(Duration::from_millis(SUGGESTION_TICK_MS));
            let state = app.state::<state::AppState>();
            let Ok(settings) = state.settings.load() else {
                continue;
            };
            // Salvar sensibilidade/cues/cooldown passa a valer sem reiniciar.
            state.suggestion_gate.reconfigure(
                application::suggestion_gate_service::config_from_settings(&settings),
            );
            let interval_ms = (settings.suggestion_interval_secs as i64)
                .saturating_mul(1_000)
                .max(SUGGESTION_MIN_INTERVAL_MS);
            // Abrir a janela reinicia o ciclo (a janela não gera ao abrir).
            let window_open = suggestions_window_open(&app);
            if window_open && !window_was_open {
                cycle_started = std::time::Instant::now();
            }
            window_was_open = window_open;
            // Ligar o automático também começa um ciclo cheio.
            if settings.suggestion_auto_refresh && !auto_was_enabled {
                cycle_started = std::time::Instant::now();
            }
            auto_was_enabled = settings.suggestion_auto_refresh;

            let session_id = state.session.current_session_id();
            if session_id != current_session {
                // Sessao nova: o cursor recomeca (a primeira sugestao nao
                // compara com a fala da sessao anterior) e o ciclo tambem.
                if let Some(previous) = current_session.as_deref() {
                    state.suggestion_gate.reset(previous);
                }
                current_session = session_id.clone();
                cursor = None;
                cycle_started = std::time::Instant::now();
            }

            // Modo classificador (BR-092/BR-094): a decisão é do gate, não do
            // timer. Mantém a janela aberta como o modo automático.
            if settings.suggestion_mode == "classifier"
                && settings.suggestion_auto_refresh
                && window_open
            {
                let Some(session_id) = session_id.clone() else {
                    continue;
                };
                let now_ms = state.clock.now_ms();
                let fallback_auto = settings.suggestion_classifier_fallback == "auto";

                // Bloco fechado → fallback (application); sem sonda aberta.
                match application::classifier_tick::classify_once(
                    state.suggestion_gate.as_ref(),
                    &session_id,
                    fallback_auto,
                ) {
                    application::classifier_tick::ClassifierTick::Classified(outcome) => {
                        let decision = state
                            .suggestion_gate
                            .should_generate(&outcome, now_ms, false);
                        if decision != GateDecision::Generate {
                            continue;
                        }
                        tracing::info!(
                            "sugestão do classificador: cue={} confiança={:.2} bloco={}",
                            outcome.cue.as_str(),
                            outcome.confidence,
                            outcome.block_id
                        );

                        state.publisher.emit_suggestion_classification(
                            &SuggestionClassificationEvent {
                                session_id: session_id.clone(),
                                state: SuggestionClassifierState::Ready,
                                outcome: Some(outcome.clone()),
                                error: None,
                                generating: true,
                            },
                        );

                        // O gate já decidiu; `force` evita que o serviço
                        // reaproveite o cache porque o bloco termina no mesmo
                        // instante do cursor (`recent.until_ms <= since`).
                        let request = SuggestionRequest::ephemeral(
                            Some(session_id.clone()),
                            settings.assistant_mode_id.clone(),
                        )
                        .with_since_ms(Some(outcome.block_end_ms))
                        .with_force(true)
                        .with_fast_first(true)
                        .with_cue_focus(Some(outcome.cue), outcome.focus.clone())
                        .with_trigger(SuggestionTrigger::Classifier);
                        if let Err(error) = state.suggestions.generate(request) {
                            tracing::debug!("sugestão do classificador indisponível: {error}");
                        }
                        state.suggestion_gate.mark_generated(
                            &session_id,
                            &outcome.block_id,
                            now_ms,
                        );
                        state.publisher.emit_suggestion_classification(
                            &SuggestionClassificationEvent {
                                session_id,
                                state: SuggestionClassifierState::Ready,
                                outcome: Some(outcome),
                                error: None,
                                generating: false,
                            },
                        );
                        continue;
                    }
                    application::classifier_tick::ClassifierTick::Skip => continue,
                    application::classifier_tick::ClassifierTick::FallbackAuto => {
                        // Erro do classificador: cai no caminho automático.
                    }
                }
            }

            let Some(session_id) = session_id else {
                // Sem sessao: publica a cadencia ociosa (a UI mostra "sem escuta").
                publish_cadence(
                    &state,
                    None,
                    interval_ms,
                    settings.suggestion_auto_refresh,
                    false,
                );
                continue;
            };
            // Fim da última fala final: o conteúdo novo é o que libera a geração.
            let latest_final_ms = state
                .transcriptions
                .latest_final_end_ms(&session_id)
                .unwrap_or(None);
            let tick = SchedulerTick {
                enabled: settings.suggestion_auto_refresh,
                has_session: true,
                cursor_ms: cursor,
                latest_final_ms,
                elapsed_ms: cycle_started.elapsed().as_millis() as i64,
            };
            // BR-039: o agendador publica o proprio timer; a UI apenas mostra.
            let due = window_open && should_trigger(&tick, interval_ms);
            if !due {
                publish_cadence(
                    &state,
                    Some(&tick),
                    interval_ms,
                    settings.suggestion_auto_refresh,
                    false,
                );
                continue;
            }

            // Feedback imediato: o LLM pode demorar; a UI precisa saber que a
            // geração automática começou antes do primeiro evento `suggestion`.
            publish_cadence(
                &state,
                Some(&tick),
                interval_ms,
                settings.suggestion_auto_refresh,
                true,
            );
            let request =
                SuggestionRequest::ephemeral(Some(session_id), settings.assistant_mode_id.clone())
                    .with_since_ms(cursor)
                    .with_fast_first(true);
            match state.suggestions.generate(request) {
                // Sucesso ou `skipped` (nada novo): avança o cursor para o
                // ponto coberto pela resposta/cache. Sem isso, o `skipped`
                // repetia a mesma tentativa a cada intervalo.
                Ok(suggestion) => {
                    cursor = Some(suggestion.context_until_ms);
                }
                Err(error) => {
                    tracing::debug!("sugestão automática indisponível: {error}");
                    cursor = latest_final_ms;
                }
            }
            // O próximo ciclo começa depois da geração (o usuário tem o
            // intervalo cheio para ler a sugestão). Publica já, para limpar o
            // "Gerando" sem esperar o próximo tick e refletir uma pausa feita
            // durante a geração.
            cycle_started = std::time::Instant::now();
            let enabled_now = state
                .settings
                .load()
                .map(|current| current.suggestion_auto_refresh)
                .unwrap_or(settings.suggestion_auto_refresh);
            let fresh = SchedulerTick {
                enabled: enabled_now,
                elapsed_ms: 0,
                ..tick
            };
            publish_cadence(&state, Some(&fresh), interval_ms, enabled_now, false);
        }
    });
}

/// A janela de Sugestões está visível (aberta) agora?
fn suggestions_window_open(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("suggestions")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// Publica a cadencia do auto-sugerir para a janela de sugestoes (BR-039).
///
/// Publica **sempre** (o evento e barato e a UI so assina quando a janela esta
/// montada): assim a tela fica reativa a fala nova sem depender do estado de
/// visibilidade do SO.
fn publish_cadence(
    state: &state::AppState,
    tick: Option<&application::suggestion_scheduler::SchedulerTick>,
    interval_ms: i64,
    enabled: bool,
    generating: bool,
) {
    use application::suggestion_scheduler::cadence;
    use ports::event_publisher::SuggestionCadenceEvent;

    let event = match tick {
        Some(tick) => {
            let cadence = cadence(tick, interval_ms);
            SuggestionCadenceEvent {
                enabled: cadence.enabled,
                running: cadence.running,
                remaining_ms: cadence.remaining_ms,
                awaiting_speech: cadence.awaiting_speech,
                window_ms: cadence.window_ms,
                generating,
            }
        }
        None => SuggestionCadenceEvent {
            enabled,
            running: false,
            remaining_ms: 0,
            awaiting_speech: false,
            window_ms: interval_ms.max(1),
            generating: false,
        },
    };
    tracing::debug!(
        enabled = event.enabled,
        running = event.running,
        remaining_ms = event.remaining_ms,
        awaiting_speech = event.awaiting_speech,
        generating = event.generating,
        "cadencia do auto-sugerir"
    );
    state.publisher.emit_suggestion_cadence(&event);
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

/// Persiste a posição da barra "Perguntar" enquanto visível.
///
/// A barra só muda de posição por arraste explícito (BR-072); este tracker
/// apenas lembra o ponto em que o usuário a deixou, sem reposicioná-la.
fn spawn_ask_position_tracker(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(700));

        let Some(window) = app.get_webview_window(crate::ipc::ask_commands::ASK_WINDOW) else {
            continue;
        };
        if !window.is_visible().unwrap_or(false) {
            continue;
        }
        let Ok(position) = window.outer_position() else {
            continue;
        };

        let current = (position.x, position.y);
        if crate::ipc::ask_commands::observe_ask_position(&app, current) {
            let now = app.state::<state::AppState>().clock.now_ms() as u64;
            app.state::<crate::ipc::ask_commands::AskDragState>()
                .touch(now);
        }
    });
}

/// Reafirma o `always_on_top` do notch depois do boot (ISS-043).
///
/// No GTK/Mutter o hint `_NET_WM_STATE_ABOVE` pode se perder quando a janela é
/// mapeada; as janelas abertas depois (`show()`) reaplicam sozinhas, mas o
/// overlay nasce visível e não passava por esse caminho.
fn spawn_overlay_stacking_reassertion(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        // Curto o suficiente para não aparecer, longo o suficiente para o
        // compositor já ter mapeado a janela.
        std::thread::sleep(Duration::from_millis(900));
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        if window.is_visible().unwrap_or(false) {
            let _ = window.set_always_on_top(true);
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

        // `set_position` do Tauri no Linux posiciona o **cliente** (inner); o
        // tracker precisa salvar a mesma origem, senão o frame/CSD (37 px no
        // topo) faz a janela subir um frame a cada boot.
        let Ok(position) = window.inner_position().or_else(|_| window.outer_position()) else {
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
        ShortcutAction::ToggleOverlay
        | ShortcutAction::ToggleStealth
        | ShortcutAction::FocusAsk
        | ShortcutAction::CaptureScreen => run_shortcut(app, action),
        // Demorados: o menu da bandeja roda na **main thread** do GTK e o listener
        // de atalhos do X11 na thread dele. Parar a sessão espera o passe final do
        // turno (até 20 s) e pedir insight faz HTTP bloqueante (até 60 s) — na main
        // thread isso congela a interface; no listener, engole os próximos atalhos
        // (ISS-020). Thread própria para os dois.
        ShortcutAction::ToggleSession
        | ShortcutAction::RequestInsight
        | ShortcutAction::RecordingToggle => {
            let app = app.clone();
            std::thread::spawn(move || run_shortcut(&app, action));
        }
    }
}

/// Mostra a janela da barra de pesquisa e devolve o foco ao campo.
fn open_ask_window(app: &tauri::AppHandle) {
    crate::ipc::ask_commands::show_ask_window(app);
}

/// Executa a ação do atalho (pode demorar; chame pela thread certa).
fn run_shortcut(app: &tauri::AppHandle, action: ShortcutAction) {
    use crate::ports::session_controller::AudioChannel;

    let state = app.state::<state::AppState>();
    match action {
        ShortcutAction::ToggleOverlay => {
            let _ = state.overlay.toggle();
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
        ShortcutAction::RecordingToggle => {
            let current = state.recording.state();
            let result = match current.status.as_str() {
                "recording" => state.recording.pause(),
                "paused" => state.recording.resume(),
                _ => match state.session.current_session_id() {
                    Some(session_id) => state.recording.start(&session_id, None),
                    None => Err(GhostWhisperError::Config(
                        "inicie uma sessão antes de gravar".to_string(),
                    )),
                },
            };
            if let Err(error) = result {
                tracing::warn!("falha no atalho de gravação: {error}");
            }
        }
        ShortcutAction::FocusAsk => {
            // Abre a barra de pesquisa e pede o foco do campo (BR-052).
            open_ask_window(app);
        }
        ShortcutAction::CaptureScreen => {
            // A própria barra faz a captura + pergunta (um caminho só).
            open_ask_window(app);
            let _ = app.emit_to("ask", "capture_screen", ());
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

/// Largura lógica da prévia do print (janela `capture`).
const PREVIEW_WIDTH: f64 = 240.0;

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

/// Caminho do log local (`<app_data>/logs/ghostwhisper.log`).
///
/// O mesmo diretório que o Tauri usa para os dados do app no Linux. Criado no
/// boot; o arquivo é rotacionado ao passar de 5 MB.
pub(crate) fn log_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    let dir = base.join("com.ghostwhisper.desktop").join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("ghostwhisper.log");
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 5 * 1024 * 1024) {
        let _ = std::fs::rename(&path, dir.join("ghostwhisper.log.1"));
    }
    Some(path)
}

struct SharedLogFile(std::sync::Arc<std::sync::Mutex<std::fs::File>>);

struct SharedLogGuard<'a>(std::sync::MutexGuard<'a, std::fs::File>);

impl std::io::Write for SharedLogGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedLogFile {
    type Writer = SharedLogGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        SharedLogGuard(
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

/// Configura o subsistema de logs (`RUST_LOG` controla o nível).
///
/// Além do stderr, grava em `<app_data>/logs/ghostwhisper.log` para consulta
/// local. Nenhum texto de transcrição/segredo é logado por contrato.
fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    // whisper.cpp/ggml são barulhentos (um `whisper_full_with_state` por token).
    // O hook do `whisper-rs` manda esses logs para o `tracing`; aqui eles ficam
    // em `warn` por padrão mesmo quando o usuário sobe o nível global com
    // `RUST_LOG=info` (o caso comum). Para depurar o ASR:
    // `RUST_LOG=info,whisper_rs=debug`.
    let raw = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let directives = if raw.contains("whisper_rs") {
        raw
    } else {
        format!("{raw},whisper_rs=warn")
    };
    let filter =
        EnvFilter::try_new(directives).unwrap_or_else(|_| EnvFilter::new("info,whisper_rs=warn"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let file_layer = log_file_path().and_then(|path| {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(SharedLogFile(std::sync::Arc::new(std::sync::Mutex::new(
                    file,
                ))))
                .with_ansi(false),
        )
    });

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer);
    match file_layer {
        Some(layer) => {
            let _ = registry.with(layer).try_init();
        }
        None => {
            let _ = registry.try_init();
        }
    }
    if let Some(path) = log_file_path() {
        tracing::info!("logs: {}", path.display());
    }
    whisper_rs::install_logging_hooks();
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
