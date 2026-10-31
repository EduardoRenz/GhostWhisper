//! Caso de uso: gravar a sessão (tela/janela + áudio) em MP4 (BR-065…BR-070).
//!
//! O serviço amarra a gravação ao ciclo da sessão, **congela** a configuração no
//! início, grava em segmentos MKV e finaliza em background com progresso.
//! Falha de gravação **nunca** interrompe a captura/transcrição (RNF-07).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::TimeZone;
use uuid::Uuid;

use crate::domain::recording::{
    clamp_audio_kbps, clamp_fps, pick_or_default, Recording, RecordingKind, RecordingSpec,
    RecordingState, RecordingStatus, RecordingTarget, DEFAULT_RECORDING_CONTAINER,
    DEFAULT_RECORDING_QUALITY, DEFAULT_RECORDING_RESOLUTION, DEFAULT_RECORDING_TARGET,
    RECORDING_CONTAINERS, RECORDING_ENCODERS, RECORDING_QUALITIES, RECORDING_RESOLUTIONS,
    RECORDING_TARGETS,
};
use crate::domain::settings::Settings;
use crate::error::{GhostWhisperError, Result};
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, RecordingProgressEvent};
use crate::ports::recording::{RecordingController, RecordingLifecycle};
use crate::ports::recordings_repository::RecordingsRepository;
use crate::ports::repositories::SessionRepository;
use crate::ports::screen_recorder::{
    FinalizeRequest, RecordingCapabilities, RecordingFinalizer, RecordingStart, ScreenRecorder,
};
use crate::ports::screen_targets::ScreenTargets;
use crate::ports::session_controller::AudioChannel;
use crate::ports::settings_store::SettingsStore;

/// Orquestra a gravação da sessão.
pub struct RecordingService {
    recorder: Arc<dyn ScreenRecorder>,
    finalizer: Arc<dyn RecordingFinalizer>,
    targets: Arc<dyn ScreenTargets>,
    recordings: Arc<dyn RecordingsRepository>,
    publisher: Arc<dyn EventPublisher>,
    settings: Arc<dyn SettingsStore>,
    clock: Arc<dyn Clock>,
    sessions: Arc<dyn SessionRepository>,
    /// Pasta padrão quando as settings não definem uma.
    default_dir: PathBuf,
    capabilities: RecordingCapabilities,
    inner: Arc<Mutex<Inner>>,
    /// Thread que segue a janela ativa (opcional).
    follow: Mutex<Option<Arc<AtomicBool>>>,
}

/// Gravação em andamento.
struct Active {
    id: String,
    session_id: String,
    kind: RecordingKind,
    spec: RecordingSpec,
    /// Arquivo final desta execução (único por gravação).
    output: PathBuf,
    /// Dimensões do primeiro segmento (base do `scale`+`pad` na finalização).
    first_size: (u32, u32),
    /// Algum segmento mudou de dimensão (troca de janela) ⇒ reencode no fim.
    normalize: bool,
    /// Tempo acumulado dos trechos já fechados (ms).
    accumulated_ms: i64,
    /// Quando o trecho atual começou (`None` = pausado).
    resumed_at_ms: Option<i64>,
}

/// Estado mutável compartilhado com a thread de finalização.
struct Inner {
    active: Option<Active>,
    last_state: RecordingState,
    last_channels: Vec<AudioChannel>,
    current_session_id: Option<String>,
}

impl Inner {
    fn empty() -> Self {
        Self {
            active: None,
            last_state: RecordingState::idle(),
            last_channels: Vec::new(),
            current_session_id: None,
        }
    }
}

impl RecordingService {
    /// Cria o serviço com as portas resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        recorder: Arc<dyn ScreenRecorder>,
        finalizer: Arc<dyn RecordingFinalizer>,
        targets: Arc<dyn ScreenTargets>,
        recordings: Arc<dyn RecordingsRepository>,
        publisher: Arc<dyn EventPublisher>,
        settings: Arc<dyn SettingsStore>,
        clock: Arc<dyn Clock>,
        sessions: Arc<dyn SessionRepository>,
        default_dir: PathBuf,
    ) -> Self {
        let capabilities = recorder.capabilities();
        Self {
            recorder,
            finalizer,
            targets,
            recordings,
            publisher,
            settings,
            clock,
            sessions,
            default_dir,
            capabilities,
            inner: Arc::new(Mutex::new(Inner::empty())),
            follow: Mutex::new(None),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock da gravação envenenado".to_string()))
    }

    /// Monta a configuração congelada a partir das settings.
    fn build_spec(
        &self,
        settings: &Settings,
        capabilities: &RecordingCapabilities,
        channels: &[AudioChannel],
        requested: Option<RecordingTarget>,
    ) -> Result<RecordingSpec> {
        let kind = RecordingKind::parse(&settings.recording_mode);
        let target_setting = pick_or_default(
            &settings.recording_target,
            &RECORDING_TARGETS,
            DEFAULT_RECORDING_TARGET,
        );
        let target = match requested {
            Some(target) => Some(target),
            None if target_setting == "window" => self
                .targets
                .active_window()
                .or_else(|| self.targets.screen()),
            None => self.targets.screen(),
        }
        .ok_or_else(|| {
            GhostWhisperError::Config(
                "não foi possível determinar a área de captura (X11 indisponível)".to_string(),
            )
        })?;

        let encoder_setting =
            pick_or_default(&settings.recording_encoder, &RECORDING_ENCODERS, "auto");
        let encoder = match encoder_setting.as_str() {
            "x264" => "x264",
            "vaapi" if capabilities.encoder == "vaapi" => "vaapi",
            _ => capabilities.encoder.as_str(),
        };

        let dir = settings
            .recording_dir
            .as_deref()
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_dir.clone());

        Ok(RecordingSpec {
            kind,
            fps: clamp_fps(settings.recording_fps),
            resolution: pick_or_default(
                &settings.recording_resolution,
                &RECORDING_RESOLUTIONS,
                DEFAULT_RECORDING_RESOLUTION,
            ),
            quality: pick_or_default(
                &settings.recording_quality,
                &RECORDING_QUALITIES,
                DEFAULT_RECORDING_QUALITY,
            ),
            encoder: encoder.to_string(),
            audio_kbps: clamp_audio_kbps(settings.recording_audio_kbps),
            container: pick_or_default(
                &settings.recording_container,
                &RECORDING_CONTAINERS,
                DEFAULT_RECORDING_CONTAINER,
            ),
            target,
            keep_segments: settings.recording_keep_segments,
            microphone: channels.contains(&AudioChannel::Microphone),
            system: channels.contains(&AudioChannel::System),
            output_dir: dir,
        })
    }

    /// Prefixo amigável do arquivo (`Título-AAAAMMDD-HHMMSS`).
    fn file_stem(&self, session_id: &str, started_at_ms: i64) -> String {
        let stamp = chrono::Local
            .timestamp_millis_opt(started_at_ms)
            .single()
            .map(|moment| moment.format("%Y%m%d-%H%M%S").to_string())
            .unwrap_or_else(|| started_at_ms.to_string());
        let subject = self
            .sessions
            .get(session_id)
            .ok()
            .flatten()
            .and_then(|session| session.subject);
        let slug = subject
            .as_deref()
            .and_then(slugify)
            .unwrap_or_else(|| session_id.chars().take(8).collect::<String>());
        format!("{slug}-{stamp}")
    }

    fn state_of(active: &Active, now_ms: i64, status: &str) -> RecordingState {
        let running = active
            .resumed_at_ms
            .map(|started| now_ms.saturating_sub(started))
            .unwrap_or(0);
        RecordingState {
            status: status.to_string(),
            kind: Some(active.kind.as_str().to_string()),
            session_id: Some(active.session_id.clone()),
            target_label: Some(active.spec.target.label().to_string()),
            elapsed_ms: active.accumulated_ms + running,
            size_bytes: 0,
        }
    }
}

impl RecordingService {
    /// Marca a thread de follow para parar.
    fn stop_follow(&self) {
        if let Ok(mut guard) = self.follow.lock() {
            if let Some(flag) = guard.take() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    /// (Re)inicia o follow se a setting está ligada e o alvo atual é janela.
    fn restart_follow(&self) {
        self.stop_follow();
        let enabled = self
            .settings
            .load()
            .map(|settings| settings.recording_follow_active)
            .unwrap_or(false);
        if !enabled {
            return;
        }
        let is_window = self
            .lock()
            .ok()
            .and_then(|inner| {
                inner
                    .active
                    .as_ref()
                    .map(|active| matches!(active.spec.target, RecordingTarget::Window { .. }))
            })
            .unwrap_or(false);
        if !is_window {
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        if let Ok(mut guard) = self.follow.lock() {
            *guard = Some(Arc::clone(&stop));
        }
        let targets = Arc::clone(&self.targets);
        let recorder = Arc::clone(&self.recorder);
        let publisher = Arc::clone(&self.publisher);
        let clock = Arc::clone(&self.clock);
        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            // Exige a mesma janela em dois ticks (1 s): alt-tab rápido não corta.
            let mut candidate: Option<(u32, RecordingTarget, u8)> = None;
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(500));
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let Some(target) = targets.active_window() else {
                    candidate = None;
                    continue;
                };
                let Some(id) = target.window_id() else {
                    candidate = None;
                    continue;
                };
                let (width, height) = target.dimensions();
                if width < 320 || height < 240 {
                    candidate = None;
                    continue;
                }
                let current = inner.lock().ok().and_then(|guard| {
                    guard
                        .active
                        .as_ref()
                        .and_then(|active| active.spec.target.window_id())
                });
                if current == Some(id) {
                    candidate = None;
                    continue;
                }
                let hits = match &candidate {
                    Some((candidate_id, _, hits)) if *candidate_id == id => hits.saturating_add(1),
                    _ => 1,
                };
                candidate = Some((id, target.clone(), hits));
                if hits < 2 {
                    continue;
                }
                candidate = None;

                let Ok(mut guard) = inner.lock() else {
                    continue;
                };
                let Some(active) = guard.active.as_mut() else {
                    break;
                };
                if active.resumed_at_ms.is_none() {
                    continue;
                }
                if recorder.switch_target(target.clone()).is_err() {
                    continue;
                }
                active.spec.target = target;
                let state = Self::state_of(active, clock.now_ms(), "recording");
                guard.last_state = state.clone();
                drop(guard);
                publisher.emit_recording_state(&state);
            }
        });
    }
}

impl RecordingController for RecordingService {
    fn capabilities(&self) -> RecordingCapabilities {
        self.capabilities.clone()
    }

    fn targets(&self) -> Vec<RecordingTarget> {
        let monitors = self.targets.monitors();
        let mut out = Vec::new();
        match self.targets.screen() {
            // Com mais de um monitor, "Todos os monitores" + cada um.
            Some(screen) if monitors.len() > 1 => {
                out.push(screen);
                out.extend(monitors);
            }
            Some(screen) => out.push(screen),
            None => out.extend(monitors),
        }
        out.extend(self.targets.windows());
        out
    }

    fn start(&self, session_id: &str, target: Option<RecordingTarget>) -> Result<()> {
        let capabilities = self.capabilities.clone();
        if !capabilities.available {
            return Err(GhostWhisperError::Config(
                capabilities
                    .reason
                    .unwrap_or_else(|| "gravação indisponível".to_string()),
            ));
        }
        let settings = self.settings.load()?;
        let (channels, current) = {
            let inner = self.lock()?;
            (
                inner.last_channels.clone(),
                inner.current_session_id.clone(),
            )
        };
        match current.as_deref() {
            Some(current) if current == session_id => {}
            Some(_) => {
                return Err(GhostWhisperError::Config(
                    "a gravação pertence a outra sessão".to_string(),
                ))
            }
            None => {
                return Err(GhostWhisperError::Config(
                    "não há sessão ativa para gravar".to_string(),
                ))
            }
        }
        let channels = if channels.is_empty() {
            vec![AudioChannel::Microphone, AudioChannel::System]
        } else {
            channels
        };
        let spec = self.build_spec(&settings, &capabilities, &channels, target)?;
        let (width, height) = spec.target.dimensions();

        let id = Uuid::new_v4().to_string();
        let started_at_ms = self.clock.now_ms();
        // Cada execução tem sua pasta e seu arquivo: continuar uma sessão
        // antiga **não** sobrescreve a gravação anterior (BR-065).
        let session_dir = spec.output_dir.join(session_id);
        let stem = self.file_stem(session_id, started_at_ms);
        let segments_dir = session_dir.join(&stem);
        let output = session_dir.join(format!("{stem}.{}", spec.extension()));
        let recording = Recording {
            id: id.clone(),
            session_id: session_id.to_string(),
            kind: spec.kind,
            status: RecordingStatus::Recording,
            path: None,
            segments_dir: Some(segments_dir.to_string_lossy().into_owned()),
            size_bytes: 0,
            duration_ms: 0,
            width,
            height,
            fps: spec.fps,
            started_at_ms,
            finished_at_ms: None,
            error: None,
        };
        self.recordings.upsert(&recording)?;

        let start = RecordingStart {
            spec: spec.clone(),
            segments_dir,
        };
        if let Err(error) = self.recorder.start(&start) {
            let _ = self.recordings.update_status(
                &id,
                RecordingStatus::Failed,
                Some(&error.to_string()),
            );
            self.publisher.emit_recording_state(&RecordingState::idle());
            return Err(error);
        }

        let state = RecordingState {
            status: "recording".to_string(),
            kind: Some(spec.kind.as_str().to_string()),
            session_id: Some(session_id.to_string()),
            target_label: Some(spec.target.label().to_string()),
            elapsed_ms: 0,
            size_bytes: 0,
        };
        // No Wayland o `start` espera o usuário escolher a fonte no portal.
        // O cronômetro começa quando a captura realmente começa (o `started_at`
        // do arquivo continua sendo o do clique, para o nome do arquivo).
        let resumed_at_ms = self.clock.now_ms();
        {
            let mut inner = self.lock()?;
            inner.active = Some(Active {
                id,
                session_id: session_id.to_string(),
                kind: spec.kind,
                spec,
                output,
                first_size: (width, height),
                normalize: false,
                accumulated_ms: 0,
                resumed_at_ms: Some(resumed_at_ms),
            });
            inner.last_state = state.clone();
            inner.current_session_id = Some(session_id.to_string());
        }
        self.publisher.emit_recording_state(&state);
        self.restart_follow();
        tracing::info!(session_id, "gravação iniciada");
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let mut inner = self.lock()?;
        let Some(active) = inner.active.as_mut() else {
            return Ok(());
        };
        let Some(resumed_at) = active.resumed_at_ms else {
            return Ok(());
        };
        self.recorder.pause()?;
        let now = self.clock.now_ms();
        active.accumulated_ms += now.saturating_sub(resumed_at);
        active.resumed_at_ms = None;
        let state = Self::state_of(active, now, "paused");
        let id = active.id.clone();
        inner.last_state = state.clone();
        drop(inner);
        self.stop_follow();

        let _ = self
            .recordings
            .update_status(&id, RecordingStatus::Paused, None);
        self.publisher.emit_recording_state(&state);
        Ok(())
    }

    fn switch_target(&self, target: RecordingTarget) -> Result<()> {
        let mut inner = self.lock()?;
        let Some(active) = inner.active.as_mut() else {
            return Err(GhostWhisperError::Config(
                "não há gravação em andamento".to_string(),
            ));
        };
        self.recorder.switch_target(target.clone())?;
        if target.dimensions() != active.first_size {
            active.normalize = true;
        }
        active.spec.target = target;
        let now = self.clock.now_ms();
        let status = if active.resumed_at_ms.is_some() {
            "recording"
        } else {
            "paused"
        };
        let state = Self::state_of(active, now, status);
        inner.last_state = state.clone();
        drop(inner);
        self.publisher.emit_recording_state(&state);
        Ok(())
    }

    fn resume(&self) -> Result<()> {
        let mut inner = self.lock()?;
        let Some(active) = inner.active.as_mut() else {
            return Ok(());
        };
        if active.resumed_at_ms.is_some() {
            return Ok(());
        }
        self.recorder.resume()?;
        let now = self.clock.now_ms();
        active.resumed_at_ms = Some(now);
        let state = Self::state_of(active, now, "recording");
        let id = active.id.clone();
        inner.last_state = state.clone();
        drop(inner);

        let _ = self
            .recordings
            .update_status(&id, RecordingStatus::Recording, None);
        self.publisher.emit_recording_state(&state);
        self.restart_follow();
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.stop_follow();
        let active = {
            let mut inner = self.lock()?;
            inner.active.take()
        };
        let Some(mut active) = active else {
            return Ok(());
        };

        if let Some(resumed_at) = active.resumed_at_ms.take() {
            active.accumulated_ms += self.clock.now_ms().saturating_sub(resumed_at);
        }
        self.recorder.stop()?;
        let parts = self.recorder.segments();
        let audio = self.recorder.fallback_audio();
        let durations = self.recorder.segment_durations_ms();
        let total_ms = durations
            .iter()
            .copied()
            .sum::<i64>()
            .max(active.accumulated_ms);

        if parts.is_empty() && audio.is_none() {
            let _ = self.recordings.update_status(
                &active.id,
                RecordingStatus::Failed,
                Some("nenhum segmento"),
            );
            self.publish_idle();
            return Ok(());
        }

        self.recordings
            .update_status(&active.id, RecordingStatus::Finalizing, None)?;
        let output = active.output.clone();
        let request = FinalizeRequest {
            parts,
            output,
            spec: active.spec.clone(),
            total_ms,
            width: active.first_size.0,
            height: active.first_size.1,
            normalize: active.normalize,
            audio,
        };

        let state = RecordingState {
            status: "finalizing".to_string(),
            kind: Some(active.kind.as_str().to_string()),
            session_id: Some(active.session_id.clone()),
            target_label: Some(active.spec.target.label().to_string()),
            elapsed_ms: total_ms,
            size_bytes: 0,
        };
        {
            let mut inner = self.lock()?;
            inner.last_state = state.clone();
        }
        self.publisher.emit_recording_state(&state);

        self.spawn_finalize(active, request);
        Ok(())
    }

    fn discard(&self, session_id: &str) -> Result<()> {
        self.stop_follow();
        {
            let mut inner = self.lock()?;
            if inner
                .active
                .as_ref()
                .is_some_and(|active| active.session_id == session_id)
            {
                inner.active = None;
                let _ = self.recorder.stop();
            }
        }

        // Descarta **todas** as execuções da sessão (a antiga e a atual).
        for recording in self.recordings.list_by_session(session_id)? {
            if let Some(path) = &recording.path {
                let _ = std::fs::remove_file(path);
            }
            if let Some(dir) = &recording.segments_dir {
                let _ = std::fs::remove_dir_all(dir);
            }
            self.recordings
                .update_status(&recording.id, RecordingStatus::Discarded, None)?;
        }
        self.publish_idle();
        Ok(())
    }

    fn state(&self) -> RecordingState {
        let Ok(inner) = self.lock() else {
            return RecordingState::idle();
        };
        match inner.active.as_ref() {
            Some(active) => {
                let now = self.clock.now_ms();
                let status = if active.resumed_at_ms.is_some() {
                    "recording"
                } else {
                    "paused"
                };
                Self::state_of(active, now, status)
            }
            None => inner.last_state.clone(),
        }
    }

    fn get_by_session(&self, session_id: &str) -> Result<Option<Recording>> {
        self.recordings.get_by_session(session_id)
    }
}

impl RecordingService {
    /// Finaliza em background: concat/remux, persistência e eventos.
    fn spawn_finalize(&self, active: Active, request: FinalizeRequest) {
        let finalizer = Arc::clone(&self.finalizer);
        let recordings = Arc::clone(&self.recordings);
        let publisher = Arc::clone(&self.publisher);
        let clock = Arc::clone(&self.clock);
        let inner = Arc::clone(&self.inner);
        let id = active.id;
        let session_id = active.session_id;
        let keep_segments = active.spec.keep_segments;
        let duration_ms = request.total_ms;

        std::thread::spawn(move || {
            let emit = |phase: &str,
                        percent: u8,
                        path: Option<String>,
                        size_bytes: u64,
                        error: Option<String>| {
                publisher.emit_recording_progress(&RecordingProgressEvent {
                    session_id: session_id.clone(),
                    phase: phase.to_string(),
                    percent,
                    path,
                    size_bytes,
                    duration_ms,
                    error,
                });
            };

            let progress = |percent: u8| emit("finalizing", percent, None, 0, None);
            match finalizer.finalize(&request, &progress) {
                Ok(artifact) => {
                    // O usuário pode ter descartado durante a finalização.
                    if recordings
                        .get_by_session(&session_id)
                        .ok()
                        .flatten()
                        .is_some_and(|recording| recording.status == RecordingStatus::Discarded)
                    {
                        finalizer.cleanup(&request.parts, &request.output);
                        let _ = std::fs::remove_file(&artifact.path);
                        return;
                    }
                    if !keep_segments {
                        finalizer.cleanup(&request.parts, &request.output);
                    }
                    let path = artifact.path.to_string_lossy().into_owned();
                    let _ = recordings.set_artifact(
                        &id,
                        &path,
                        artifact.size_bytes,
                        artifact.duration_ms,
                        clock.now_ms(),
                    );
                    emit("ready", 100, Some(path), artifact.size_bytes, None);
                    // Só volta para ocioso se nenhuma gravação nova começou
                    // (continuar a sessão durante a finalização da anterior).
                    if let Ok(mut guard) = inner.lock() {
                        if guard.active.is_none() {
                            guard.last_state = RecordingState::idle();
                            drop(guard);
                            publisher.emit_recording_state(&RecordingState::idle());
                        }
                    }
                    tracing::info!(session_id, "gravação finalizada");
                }
                Err(error) => {
                    let message = error.to_string();
                    let discarded = recordings
                        .get_by_session(&session_id)
                        .ok()
                        .flatten()
                        .is_some_and(|recording| recording.status == RecordingStatus::Discarded);
                    if discarded {
                        return;
                    }
                    let _ = recordings.update_status(&id, RecordingStatus::Failed, Some(&message));
                    emit("failed", 0, None, 0, Some(message.clone()));
                    if let Ok(mut guard) = inner.lock() {
                        if guard.active.is_none() {
                            guard.last_state = RecordingState::idle();
                            drop(guard);
                            publisher.emit_recording_state(&RecordingState::idle());
                        }
                    }
                    tracing::warn!(session_id, "finalização falhou: {message}");
                }
            }
        });
    }

    fn publish_idle(&self) {
        if let Ok(mut inner) = self.lock() {
            inner.last_state = RecordingState::idle();
        }
        self.publisher.emit_recording_state(&RecordingState::idle());
    }

    /// `true` quando há uma gravação em andamento.
    fn has_active(&self) -> bool {
        self.lock()
            .map(|inner| inner.active.is_some())
            .unwrap_or(false)
    }
}

impl RecordingLifecycle for RecordingService {
    fn on_session_started(
        &self,
        session_id: &str,
        channels: &[AudioChannel],
        capturing: bool,
    ) -> Result<()> {
        {
            let mut inner = self.lock()?;
            inner.last_channels = channels.to_vec();
            inner.current_session_id = Some(session_id.to_string());
        }
        let auto = self
            .settings
            .load()
            .map(|settings| settings.recording_auto_start)
            .unwrap_or(false);
        if capturing && auto {
            if let Err(error) = self.start(session_id, None) {
                tracing::warn!("gravação automática não iniciou: {error}");
            }
        }
        Ok(())
    }

    fn on_session_paused(&self) -> Result<()> {
        if let Err(error) = self.pause() {
            tracing::warn!("falha ao pausar a gravação: {error}");
        }
        Ok(())
    }

    fn on_session_resumed(&self) -> Result<()> {
        if self.has_active() {
            if let Err(error) = self.resume() {
                tracing::warn!("falha ao retomar a gravação: {error}");
            }
            return Ok(());
        }
        let auto = self
            .settings
            .load()
            .map(|settings| settings.recording_auto_start)
            .unwrap_or(false);
        let session_id = self
            .lock()
            .ok()
            .and_then(|inner| inner.current_session_id.clone());
        if auto {
            if let Some(session_id) = session_id {
                if let Err(error) = self.start(&session_id, None) {
                    tracing::warn!("gravação automática não iniciou: {error}");
                }
            }
        }
        Ok(())
    }

    fn on_session_stopped(&self) -> Result<()> {
        if let Err(error) = self.stop() {
            tracing::warn!("falha ao finalizar a gravação: {error}");
        }
        Ok(())
    }
}

/// Nome de arquivo seguro a partir do título da sessão.
fn slugify(value: &str) -> Option<String> {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in value.trim().chars().take(60) {
        if character.is_alphanumeric() {
            for lower in character.to_lowercase() {
                slug.push(lower);
            }
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    (!slug.is_empty()).then_some(slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::memory::in_memory_recordings_repository::InMemoryRecordingsRepository;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::domain::recording::{RecordingArtifact, RecordingTarget, DEFAULT_RECORDING_FPS};
    use crate::domain::settings::Settings;
    use crate::ports::screen_recorder::RecordingCapabilities;

    struct FakeClock;
    impl Clock for FakeClock {
        fn now_ms(&self) -> i64 {
            1_000
        }
    }

    struct FakeSettings(Settings);
    impl SettingsStore for FakeSettings {
        fn load(&self) -> Result<Settings> {
            Ok(self.0.clone())
        }
        fn save(&self, _settings: &Settings) -> Result<()> {
            Ok(())
        }
    }

    struct FakeTargets;
    impl ScreenTargets for FakeTargets {
        fn active_window(&self) -> Option<RecordingTarget> {
            Some(RecordingTarget::Window {
                id: 7,
                label: "Reunião".into(),
                width: 1920,
                height: 1080,
            })
        }
        fn screen(&self) -> Option<RecordingTarget> {
            Some(RecordingTarget::Screen {
                label: "Tela inteira".into(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
            })
        }
        fn windows(&self) -> Vec<RecordingTarget> {
            Vec::new()
        }
        fn monitors(&self) -> Vec<RecordingTarget> {
            vec![RecordingTarget::Screen {
                label: "Monitor 1 — 1920×1080".into(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
            }]
        }
    }

    struct FakeRecorder {
        capabilities: RecordingCapabilities,
        capturing: Mutex<bool>,
        segments: Mutex<Vec<PathBuf>>,
        switches: Mutex<Vec<RecordingTarget>>,
    }
    impl FakeRecorder {
        fn available() -> Self {
            Self {
                capabilities: RecordingCapabilities::available("x264"),
                capturing: Mutex::new(false),
                segments: Mutex::new(Vec::new()),
                switches: Mutex::new(Vec::new()),
            }
        }
    }
    impl ScreenRecorder for FakeRecorder {
        fn capabilities(&self) -> RecordingCapabilities {
            self.capabilities.clone()
        }
        fn start(&self, request: &RecordingStart) -> Result<()> {
            *self.capturing.lock().unwrap() = true;
            let mut segments = self.segments.lock().unwrap();
            segments.clear();
            segments.push(request.segments_dir.join("part-0000.mkv"));
            Ok(())
        }
        fn pause(&self) -> Result<()> {
            *self.capturing.lock().unwrap() = false;
            Ok(())
        }
        fn resume(&self) -> Result<()> {
            *self.capturing.lock().unwrap() = true;
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            *self.capturing.lock().unwrap() = false;
            Ok(())
        }
        fn segments(&self) -> Vec<PathBuf> {
            self.segments.lock().unwrap().clone()
        }
        fn segment_durations_ms(&self) -> Vec<i64> {
            vec![1_000]
        }
        fn switch_target(&self, target: RecordingTarget) -> Result<()> {
            self.switches.lock().unwrap().push(target);
            Ok(())
        }
        fn fallback_audio(&self) -> Option<crate::ports::screen_recorder::FallbackAudio> {
            None
        }
    }

    struct FakeFinalizer;
    impl RecordingFinalizer for FakeFinalizer {
        fn finalize(
            &self,
            request: &FinalizeRequest,
            progress: &dyn Fn(u8),
        ) -> Result<RecordingArtifact> {
            progress(100);
            if let Some(parent) = request.output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&request.output, b"mp4")?;
            Ok(RecordingArtifact {
                path: request.output.clone(),
                size_bytes: 3,
                duration_ms: request.total_ms,
                width: 1920,
                height: 1080,
            })
        }
        fn cleanup(&self, _parts: &[std::path::PathBuf], _output: &std::path::Path) {}
    }

    struct NoopPublisher;
    impl EventPublisher for NoopPublisher {
        fn emit_segment(&self, _segment: &crate::domain::transcription::TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &crate::ports::event_publisher::LlmSourceUsage) {}
        fn emit_journal(&self, _update: &crate::ports::event_publisher::JournalUpdate) {}
    }

    fn service(dir: &std::path::Path) -> RecordingService {
        let recordings = Arc::new(InMemoryRecordingsRepository::new());
        RecordingService::new(
            Arc::new(FakeRecorder::available()),
            Arc::new(FakeFinalizer),
            Arc::new(FakeTargets),
            recordings,
            Arc::new(NoopPublisher),
            Arc::new(FakeSettings(Settings::default())),
            Arc::new(FakeClock),
            Arc::new(InMemorySessionRepository::new()),
            dir.to_path_buf(),
        )
    }

    #[test]
    fn start_builds_the_frozen_spec_and_persists() {
        let dir = tempfile::tempdir().expect("dir");
        let service = service(dir.path());
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");
        let state = service.state();
        assert_eq!(state.status, "recording");
        assert_eq!(state.target_label.as_deref(), Some("Reunião"));
        let stored = service.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Recording);
        assert_eq!(stored.fps, DEFAULT_RECORDING_FPS);
    }

    #[test]
    fn pause_and_resume_change_the_state_without_ending_the_session() {
        let dir = tempfile::tempdir().expect("dir");
        let service = service(dir.path());
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");
        service.pause().expect("pause");
        assert_eq!(service.state().status, "paused");
        service.resume().expect("resume");
        assert_eq!(service.state().status, "recording");
    }

    #[test]
    fn stop_finalizes_and_marks_ready() {
        let dir = tempfile::tempdir().expect("dir");
        let service = service(dir.path());
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");
        service.stop().expect("stop");
        // A finalização é assíncrona: aguarda o estado virar `ready`.
        for _ in 0..50 {
            if service
                .get_by_session("s1")
                .expect("get")
                .is_some_and(|recording| recording.status == RecordingStatus::Ready)
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let stored = service.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Ready);
        assert_eq!(stored.size_bytes, 3);
    }

    #[test]
    fn continuing_a_session_starts_a_new_recording() {
        let dir = tempfile::tempdir().expect("dir");
        let recordings = Arc::new(InMemoryRecordingsRepository::new());
        let repository: Arc<dyn RecordingsRepository> = recordings.clone();
        let service = RecordingService::new(
            Arc::new(FakeRecorder::available()),
            Arc::new(FakeFinalizer),
            Arc::new(FakeTargets),
            Arc::clone(&repository),
            Arc::new(NoopPublisher),
            Arc::new(FakeSettings(Settings::default())),
            Arc::new(FakeClock),
            Arc::new(InMemorySessionRepository::new()),
            dir.path().to_path_buf(),
        );

        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("primeira gravação");
        service.stop().expect("finaliza a primeira");
        service
            .start("s1", None)
            .expect("gravação nova na mesma sessão");

        let stored = recordings.list_by_session("s1").expect("lista");
        assert_eq!(stored.len(), 2, "duas execuções, dois arquivos");
        assert_ne!(stored[0].id, stored[1].id);
    }

    #[test]
    fn slugify_makes_a_safe_file_name() {
        assert_eq!(
            slugify("Reunião com o Cliente!"),
            Some("reunião-com-o-cliente".to_string())
        );
        assert_eq!(slugify("a/b:c*d"), Some("a-b-c-d".to_string()));
        assert_eq!(slugify("   "), None);
    }

    #[test]
    fn switch_target_updates_the_state_and_the_recorder() {
        let dir = tempfile::tempdir().expect("dir");
        let recorder = Arc::new(FakeRecorder::available());
        let recorder_port: Arc<dyn ScreenRecorder> = recorder.clone();
        let service = RecordingService::new(
            recorder_port,
            Arc::new(FakeFinalizer),
            Arc::new(FakeTargets),
            Arc::new(InMemoryRecordingsRepository::new()),
            Arc::new(NoopPublisher),
            Arc::new(FakeSettings(Settings::default())),
            Arc::new(FakeClock),
            Arc::new(InMemorySessionRepository::new()),
            dir.path().to_path_buf(),
        );
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");

        let target = RecordingTarget::Screen {
            label: "Monitor 2 — 1920×1080".to_string(),
            width: 1920,
            height: 1080,
            x: 1920,
            y: 0,
        };
        service.switch_target(target.clone()).expect("switch");

        assert_eq!(
            service.state().target_label.as_deref(),
            Some("Monitor 2 — 1920×1080")
        );
        assert_eq!(recorder.switches.lock().unwrap().as_slice(), &[target]);
    }

    #[test]
    fn start_without_a_session_is_refused() {
        let dir = tempfile::tempdir().expect("dir");
        let service = service(dir.path());
        let error = service.start("s1", None).expect_err("erro");
        assert!(error.to_string().contains("sessão ativa"), "{error}");
    }

    #[test]
    fn discard_removes_the_artifact() {
        let dir = tempfile::tempdir().expect("dir");
        let service = service(dir.path());
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");
        service.stop().expect("stop");
        service.discard("s1").expect("discard");
        let stored = service.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Discarded);
    }

    #[test]
    fn settings_are_frozen_at_start() {
        let dir = tempfile::tempdir().expect("dir");
        let settings = Settings::default();
        let recordings = Arc::new(InMemoryRecordingsRepository::new());
        let store = Arc::new(FakeSettings(settings.clone()));
        let service = RecordingService::new(
            Arc::new(FakeRecorder::available()),
            Arc::new(FakeFinalizer),
            Arc::new(FakeTargets),
            recordings,
            Arc::new(NoopPublisher),
            store,
            Arc::new(FakeClock),
            Arc::new(InMemorySessionRepository::new()),
            dir.path().to_path_buf(),
        );
        service
            .on_session_started("s1", &[AudioChannel::Microphone], true)
            .expect("hook");
        service.start("s1", None).expect("start");
        let stored = service.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.fps, 8);
    }
}
