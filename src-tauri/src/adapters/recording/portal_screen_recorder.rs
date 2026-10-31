//! Captura de tela/janela no **Wayland** via XDG Desktop Portal ScreenCast.
//!
//! O compositor não expõe janelas nativas ao XWayland, então a captura usa o
//! portal: o usuário escolhe **uma janela ou um monitor** no diálogo do GNOME e
//! o portal devolve um nó PipeWire. Cada segmento abre um novo fd remoto
//! (`OpenPipeWireRemote`) e é gravado por um `gst-launch-1.0` próprio
//! (`pipewiresrc` + `x264enc` + `matroskamux`), com o áudio no mesmo pipeline
//! (`pulsesrc` para microfone e monitor do sistema).
//!
//! Pausar fecha o segmento atual (SIGINT + `-e`, que finaliza o MKV) e mantém a
//! sessão do portal viva; retomar abre outro segmento com um fd novo. Isso evita
//! reabrir o seletor do GNOME no meio da reunião. Parar fecha a sessão do portal
//! (some o indicador de compartilhamento) e entrega os segmentos ao finalizador.
//!
//! A porta é síncrona; o D-Bus e o runtime do portal vivem numa thread própria
//! (mesmo padrão de `PortalScreenCapture`/`PortalGlobalShortcutRegistry`).

use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ashpd::desktop::screencast::{
    CursorMode, Screencast, SelectSourcesOptions, SourceType, StartCastOptions,
};
use ashpd::desktop::{PersistMode, Session};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::domain::recording::{RecordingKind, RecordingSpec, RecordingTarget};
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_recorder::{
    FallbackAudio, RecordingCapabilities, RecordingStart, RecordingUnavailableCode, ScreenRecorder,
};

use super::ffmpeg_args::{segment_path, DEFAULT_MICROPHONE};
use super::gstreamer_args::{segment_args, GstSegmentRequest, PortalStream};

/// Tempo máximo para o `gst-launch` finalizar o segmento após o SIGINT.
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

/// fd fixo no processo filho (o `pipewiresrc` recebe `fd=198`).
///
/// O fd original do portal é `CLOEXEC`; o `pre_exec` duplica para este número
/// (e limpa o `CLOEXEC` do destino) sem mexer na tabela de fds do app.
const GST_FD: i32 = 198;

/// Elementos usados pelo pipeline de gravação.
const REQUIRED_ELEMENTS: [&str; 15] = [
    "pipewiresrc",
    "videoconvert",
    "videoscale",
    "videorate",
    "x264enc",
    "h264parse",
    "matroskamux",
    "pulsesrc",
    "audioconvert",
    "audioresample",
    "audiomixer",
    "avenc_aac",
    "aacparse",
    "flacenc",
    "flacparse",
];

/// Captura no Wayland pelo portal ScreenCast + GStreamer.
pub struct PortalScreenRecorder {
    capabilities: RecordingCapabilities,
    commands: UnboundedSender<WorkerCommand>,
    inner: Mutex<Inner>,
}

/// Uma sessão de ScreenCast aberta (vive entre segmentos/pausas).
struct Portal {
    screencast: Screencast,
    session: Session<Screencast>,
    node: u32,
}

/// O que abrir no início de um segmento.
struct SegmentPlan {
    spec: RecordingSpec,
    output: PathBuf,
    microphone: Option<String>,
    system: Option<String>,
}

/// Comandos para a thread do portal/GStreamer.
enum WorkerCommand {
    Open {
        plan: Box<SegmentPlan>,
        reply: SyncSender<Result<u32>>,
    },
    Close {
        stop: bool,
        reply: SyncSender<Result<()>>,
    },
    Shutdown,
}

/// Segmento em aberto (o processo é da thread; o pid permite matá-lo no drop).
struct GstChild {
    child: Child,
    pid: u32,
}

struct Inner {
    spec: Option<RecordingSpec>,
    segments_dir: Option<PathBuf>,
    segments: Vec<PathBuf>,
    durations: Vec<i64>,
    segment_index: u32,
    started: Option<Instant>,
    active: bool,
    /// Pid do `gst-launch` atual (para o `Drop` não deixar órfão).
    pid: Option<u32>,
}

impl Inner {
    fn empty() -> Self {
        Self {
            spec: None,
            segments_dir: None,
            segments: Vec::new(),
            durations: Vec::new(),
            segment_index: 0,
            started: None,
            active: false,
            pid: None,
        }
    }
}

impl PortalScreenRecorder {
    /// Cria o capturador; o `ffmpeg` já resolvido é usado só para capacidades
    /// (a finalização continua no `FfmpegRecordingFinalizer`).
    pub fn new(ffmpeg: PathBuf) -> Self {
        let gst_launch = find_in_path("gst-launch-1.0");
        let gst_inspect = find_in_path("gst-inspect-1.0");
        let missing = match (&gst_launch, &gst_inspect) {
            (Some(_), Some(inspect)) => missing_elements(inspect, &REQUIRED_ELEMENTS),
            _ => Vec::new(),
        };
        let capabilities = decide_capabilities(
            ffmpeg.is_file(),
            gst_launch.is_some(),
            gst_inspect.is_some(),
            &missing,
        );
        let gst_launch = gst_launch.unwrap_or_else(|| PathBuf::from("gst-launch-1.0"));
        let (commands, worker) = spawn_worker(gst_launch.clone());
        // A thread do portal termina no `Shutdown`; se o app sair antes, o
        // processo encerra as threads junto (não bloqueia o shutdown).
        drop(worker);
        Self {
            capabilities,
            commands,
            inner: Mutex::new(Inner::empty()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock da gravação envenenado".to_string()))
    }

    /// Monta o plano do próximo segmento a partir do estado congelado.
    fn plan(&self, inner: &Inner) -> Result<SegmentPlan> {
        let spec = inner
            .spec
            .clone()
            .ok_or_else(|| GhostWhisperError::Other("gravação sem configuração".to_string()))?;
        let dir = inner
            .segments_dir
            .clone()
            .ok_or_else(|| GhostWhisperError::Other("gravação sem pasta".to_string()))?;
        let output = segment_path(&dir, inner.segment_index, spec.kind);
        let microphone = spec.microphone.then(|| DEFAULT_MICROPHONE.to_string());
        let system = spec.system.then(system_device).flatten();
        if spec.kind == RecordingKind::Audio && microphone.is_none() && system.is_none() {
            return Err(GhostWhisperError::Config(
                "nenhum canal de áudio para gravar".to_string(),
            ));
        }
        Ok(SegmentPlan {
            spec,
            output,
            microphone,
            system,
        })
    }

    /// Abre o próximo segmento (chamado pelo `start` e pelo `resume`).
    fn open_segment(&self, inner: &mut Inner) -> Result<()> {
        let plan = self.plan(inner)?;
        let output = plan.output.clone();
        let kind = plan.spec.kind;
        let (reply, response) = sync_channel(1);
        self.commands
            .send(WorkerCommand::Open {
                plan: Box::new(plan),
                reply,
            })
            .map_err(|_| {
                GhostWhisperError::Other("thread da gravação no portal encerrada".to_string())
            })?;
        let pid = response.recv().map_err(|_| {
            GhostWhisperError::Other("thread da gravação no portal não respondeu".to_string())
        })??;
        inner.pid = Some(pid);
        inner.started = Some(Instant::now());
        inner.segment_index = inner.segment_index.saturating_add(1);
        tracing::info!(
            path = %output.display(),
            kind = kind.as_str(),
            "segmento de gravação aberto (portal)"
        );
        Ok(())
    }

    /// Fecha o segmento atual e contabiliza a duração/arquivo.
    fn close_segment(&self, inner: &mut Inner, stop: bool) -> Result<()> {
        let (reply, response) = sync_channel(1);
        self.commands
            .send(WorkerCommand::Close { stop, reply })
            .map_err(|_| {
                GhostWhisperError::Other("thread da gravação no portal encerrada".to_string())
            })?;
        let result = response.recv().map_err(|_| {
            GhostWhisperError::Other("thread da gravação no portal não respondeu".to_string())
        })?;
        inner.pid = None;

        if let Some(started) = inner.started.take() {
            inner.durations.push(started.elapsed().as_millis() as i64);
            let index = inner.segment_index.saturating_sub(1);
            if let (Some(dir), Some(spec)) = (inner.segments_dir.as_ref(), inner.spec.as_ref()) {
                let path = segment_path(dir, index, spec.kind);
                if std::fs::metadata(&path)
                    .map(|meta| meta.len() > 0)
                    .unwrap_or(false)
                {
                    inner.segments.push(path);
                } else {
                    tracing::warn!(
                        path = %path.display(),
                        "segmento do portal vazio; não será concatenado"
                    );
                }
            }
        }
        if stop {
            inner.active = false;
            inner.spec = None;
            inner.pid = None;
        }
        result
    }
}

impl ScreenRecorder for PortalScreenRecorder {
    fn capabilities(&self) -> RecordingCapabilities {
        self.capabilities.clone()
    }

    fn start(&self, request: &RecordingStart) -> Result<()> {
        let mut inner = self.lock()?;
        if inner.active {
            return Err(GhostWhisperError::Other(
                "já existe uma gravação em andamento".to_string(),
            ));
        }
        std::fs::create_dir_all(&request.segments_dir)?;
        inner.spec = Some(request.spec.clone());
        inner.segments_dir = Some(request.segments_dir.clone());
        inner.segments.clear();
        inner.durations.clear();
        inner.segment_index = 0;
        inner.started = None;
        inner.active = true;
        if let Err(error) = self.open_segment(&mut inner) {
            inner.active = false;
            inner.spec = None;
            inner.started = None;
            inner.pid = None;
            // O portal pode ter aberto antes de o pipeline falhar; fecha para
            // sumir o indicador de compartilhamento e permitir nova tentativa.
            let (reply, response) = sync_channel(1);
            if self
                .commands
                .send(WorkerCommand::Close { stop: true, reply })
                .is_ok()
            {
                let _ = response.recv();
            }
            return Err(error);
        }
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let mut inner = self.lock()?;
        if !inner.active || inner.started.is_none() {
            return Ok(());
        }
        self.close_segment(&mut inner, false)
    }

    fn resume(&self) -> Result<()> {
        let mut inner = self.lock()?;
        if !inner.active || inner.started.is_some() {
            return Ok(());
        }
        self.open_segment(&mut inner)
    }

    fn stop(&self) -> Result<()> {
        let mut inner = self.lock()?;
        if !inner.active && inner.started.is_none() {
            return Ok(());
        }
        self.close_segment(&mut inner, true)
    }

    fn switch_target(&self, target: RecordingTarget) -> Result<()> {
        let inner = self.lock()?;
        let current = inner.spec.as_ref().map(|spec| spec.target.clone());
        if current.as_ref() == Some(&target) {
            return Ok(());
        }
        Err(GhostWhisperError::Config(
            "no Wayland a fonte é escolhida no portal e não muda durante a gravação".to_string(),
        ))
    }

    fn segments(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .map(|inner| inner.segments.clone())
            .unwrap_or_default()
    }

    fn segment_durations_ms(&self) -> Vec<i64> {
        self.inner
            .lock()
            .map(|inner| inner.durations.clone())
            .unwrap_or_default()
    }

    fn fallback_audio(&self) -> Option<FallbackAudio> {
        // O áudio vai no próprio segmento do GStreamer.
        None
    }
}

impl Drop for PortalScreenRecorder {
    fn drop(&mut self) {
        // Sem `stop()` (app fechando): finaliza o MKV e evita processo órfão.
        if let Ok(inner) = self.inner.lock() {
            if let Some(pid) = inner.pid {
                unsafe {
                    libc::kill(pid as i32, libc::SIGINT);
                }
            }
        }
        let _ = self.commands.send(WorkerCommand::Shutdown);
    }
}

/// Cria a thread do portal e devolve o canal de comandos.
fn spawn_worker(gst_launch: PathBuf) -> (UnboundedSender<WorkerCommand>, JoinHandle<()>) {
    let (tx, rx) = unbounded_channel();
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let Ok(runtime) = runtime else {
            tracing::warn!("gravação no portal: não foi possível criar o runtime");
            return;
        };
        runtime.block_on(worker(rx, gst_launch));
    });
    (tx, thread)
}

/// Corpo da thread: mantém a sessão do portal e um `gst-launch` por segmento.
async fn worker(mut rx: UnboundedReceiver<WorkerCommand>, gst_launch: PathBuf) {
    let mut portal: Option<Portal> = None;
    let mut child: Option<GstChild> = None;

    while let Some(command) = rx.recv().await {
        match command {
            WorkerCommand::Open { plan, reply } => {
                let result = open_gst_segment(&gst_launch, &mut portal, &mut child, &plan).await;
                let _ = reply.send(result);
            }
            WorkerCommand::Close { stop, reply } => {
                let result = close_gst_segment(&mut child);
                if stop {
                    close_portal(&mut portal).await;
                }
                let _ = reply.send(result);
            }
            WorkerCommand::Shutdown => {
                let _ = close_gst_segment(&mut child);
                close_portal(&mut portal).await;
                break;
            }
        }
    }
}

/// Abre a sessão do portal (uma vez por gravação) e inicia o `gst-launch`.
async fn open_gst_segment(
    gst_launch: &Path,
    portal: &mut Option<Portal>,
    child: &mut Option<GstChild>,
    plan: &SegmentPlan,
) -> Result<u32> {
    if child.is_some() {
        return Err(GhostWhisperError::Other(
            "já existe um segmento aberto".to_string(),
        ));
    }

    let spawned = if plan.spec.kind == RecordingKind::Video {
        ensure_portal(portal).await?;
        let portal = portal.as_ref().expect("portal aberto");
        let fd = portal
            .screencast
            .open_pipe_wire_remote(&portal.session, Default::default())
            .await
            .map_err(portal_error)?;
        let raw_fd = fd.as_raw_fd();
        let args = segment_args(&GstSegmentRequest {
            spec: &plan.spec,
            output: &plan.output,
            stream: Some(PortalStream {
                fd: GST_FD,
                node: portal.node,
            }),
            microphone: plan.microphone.as_deref(),
            system: plan.system.as_deref(),
        });
        let spawned = spawn_gst(gst_launch, &args, Some(raw_fd));
        drop(fd);
        spawned?
    } else {
        let args = segment_args(&GstSegmentRequest {
            spec: &plan.spec,
            output: &plan.output,
            stream: None,
            microphone: plan.microphone.as_deref(),
            system: plan.system.as_deref(),
        });
        spawn_gst(gst_launch, &args, None)?
    };

    let pid = spawned.pid;
    *child = Some(spawned);
    Ok(pid)
}

/// Garante a sessão de ScreenCast, pedindo a fonte ao usuário na primeira vez.
async fn ensure_portal(portal: &mut Option<Portal>) -> Result<()> {
    if portal.is_some() {
        return Ok(());
    }
    let connection = crate::adapters::system::portal::connection().await?;
    let screencast = Screencast::with_connection(connection)
        .await
        .map_err(portal_error)?;
    let session = screencast
        .create_session(Default::default())
        .await
        .map_err(portal_error)?;
    screencast
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_sources(SourceType::Monitor | SourceType::Window)
                .set_multiple(false)
                .set_cursor_mode(CursorMode::Embedded)
                .set_persist_mode(PersistMode::DoNot),
        )
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)?;
    let streams = screencast
        .start(&session, None, StartCastOptions::default())
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)?;
    let node = streams
        .streams()
        .first()
        .map(|stream| stream.pipe_wire_node_id())
        .ok_or_else(|| {
            GhostWhisperError::Other("o portal não devolveu nenhum stream de tela".to_string())
        })?;
    tracing::info!(node, "sessão de ScreenCast aberta");
    *portal = Some(Portal {
        screencast,
        session,
        node,
    });
    Ok(())
}

/// Fecha a sessão do portal (fim da gravação ou falha).
async fn close_portal(portal: &mut Option<Portal>) {
    if let Some(portal) = portal.take() {
        if let Err(error) = portal.session.close().await {
            tracing::warn!("não foi possível fechar a sessão de ScreenCast: {error}");
        }
    }
}

/// Cria um `Command` para um binário **do sistema** do GStreamer.
///
/// O `AppRun` do AppImage exporta `LD_LIBRARY_PATH` para o AppDir e
/// `GST_PLUGIN_SYSTEM_PATH[_1_0]` para `$APPDIR/usr/lib/gstreamer-1.0`. O
/// `gst-launch`/`gst-inspect` do sistema herdariam esse ambiente e deixariam de
/// carregar os plugins do host (`pipewiresrc`, `x264enc`, …). O GStreamer usado
/// aqui é o do sistema, então o filho roda com o ambiente limpo.
fn system_gst_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    for variable in GST_ENV_BLOCKLIST {
        command.env_remove(variable);
    }
    command
}

/// Variáveis do `AppRun`/cache que fariam o GStreamer do sistema carregar
/// bibliotecas ou plugins do AppDir em vez dos do host.
const GST_ENV_BLOCKLIST: [&str; 8] = [
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_SCANNER",
    "GST_REGISTRY",
    "GST_REGISTRY_1_0",
];

/// Inicia o `gst-launch` herdando o fd do PipeWire em `GST_FD`.
fn spawn_gst(gst_launch: &Path, args: &[String], fd: Option<i32>) -> Result<GstChild> {
    if args.is_empty() {
        return Err(GhostWhisperError::Config(
            "pipeline de gravação vazio".to_string(),
        ));
    }
    let mut command = system_gst_command(gst_launch);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(fd) = fd {
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(fd, GST_FD) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // `dup2` limpa o CLOEXEC do destino, exceto quando fd == GST_FD.
                if fd == GST_FD && libc::fcntl(GST_FD, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut child = command.spawn().map_err(|error| {
        GhostWhisperError::Other(format!("não foi possível iniciar o gst-launch: {error}"))
    })?;
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || drain_stderr(stderr));
    }
    Ok(GstChild {
        pid: child.id(),
        child,
    })
}

/// Lê o stderr do GStreamer sem deixar o pipe encher; loga só o final.
fn drain_stderr(mut stderr: impl Read) {
    let mut tail: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                tail.extend_from_slice(&buffer[..read]);
                if tail.len() > 8192 {
                    let excess = tail.len() - 8192;
                    tail.drain(..excess);
                }
            }
        }
    }
    if !tail.is_empty() {
        tracing::warn!(
            message = %String::from_utf8_lossy(&tail),
            "gst-launch encerrou com avisos"
        );
    }
}

/// Fecha o segmento: SIGINT (EOS do `-e`), espera e mata se travar.
fn close_gst_segment(child: &mut Option<GstChild>) -> Result<()> {
    let Some(mut running) = child.take() else {
        return Ok(());
    };
    unsafe {
        libc::kill(running.pid as i32, libc::SIGINT);
    }
    let deadline = Instant::now() + STOP_TIMEOUT;
    loop {
        match running.child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = running.child.kill();
                let _ = running.child.wait();
                break;
            }
        }
    }
    Ok(())
}

/// Cancelamento do usuário no diálogo do portal tem mensagem própria.
fn portal_error(error: ashpd::Error) -> GhostWhisperError {
    if let ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled) = &error {
        return GhostWhisperError::Config(
            "gravação cancelada: escolha uma janela ou tela no diálogo do GNOME".to_string(),
        );
    }
    GhostWhisperError::Other(format!("portal ScreenCast indisponível: {error}"))
}

/// Decisão pura de capacidades (testável sem tocar no ambiente).
fn decide_capabilities(
    ffmpeg: bool,
    gst_launch: bool,
    gst_inspect: bool,
    missing: &[String],
) -> RecordingCapabilities {
    let capabilities = if !ffmpeg {
        RecordingCapabilities::unavailable(
            RecordingUnavailableCode::FfmpegMissing,
            "ffmpeg não encontrado — necessário para finalizar a gravação",
        )
    } else if !gst_launch || !gst_inspect {
        RecordingCapabilities::unavailable(
            RecordingUnavailableCode::GStreamerMissing,
            "GStreamer (gst-launch-1.0/gst-inspect-1.0) não encontrado",
        )
    } else if !missing.is_empty() {
        RecordingCapabilities::unavailable(
            RecordingUnavailableCode::GStreamerMissing,
            format!("GStreamer sem os elementos: {}", missing.join(", ")),
        )
    } else {
        RecordingCapabilities::available("x264")
    };
    capabilities.with_portal_targets()
}

/// Elementos do GStreamer ausentes no sistema.
fn missing_elements(gst_inspect: &Path, required: &[&str]) -> Vec<String> {
    required
        .iter()
        .filter(|element| {
            !system_gst_command(gst_inspect)
                .arg(element)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        })
        .map(|element| (*element).to_string())
        .collect()
}

/// Dispositivo do áudio do sistema (`<sink>.monitor`).
fn system_device() -> Option<String> {
    crate::adapters::audio::pulse_system_audio_source::default_monitor_name().ok()
}

/// Procura um binário no `PATH`.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_require_ffmpeg_before_gstreamer() {
        let missing = Vec::new();
        let capabilities = decide_capabilities(false, false, false, &missing);
        assert!(!capabilities.available);
        assert_eq!(
            capabilities.reason_code,
            Some(RecordingUnavailableCode::FfmpegMissing)
        );
    }

    #[test]
    fn capabilities_require_gst_launch_and_inspect() {
        let capabilities = decide_capabilities(true, false, true, &[]);
        assert!(!capabilities.available);
        assert_eq!(
            capabilities.reason_code,
            Some(RecordingUnavailableCode::GStreamerMissing)
        );
    }

    #[test]
    fn capabilities_list_missing_elements() {
        let missing = vec!["pipewiresrc".to_string(), "x264enc".to_string()];
        let capabilities = decide_capabilities(true, true, true, &missing);
        assert!(!capabilities.available);
        assert_eq!(
            capabilities.reason_code,
            Some(RecordingUnavailableCode::GStreamerMissing)
        );
        assert!(capabilities
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("pipewiresrc"));
    }

    #[test]
    fn sanitizes_the_appimage_gstreamer_environment() {
        // Regressão do AppImage: o AppRun aponta LD_LIBRARY_PATH/GST_PLUGIN_*
        // para o AppDir e o gst do sistema deixa de achar pipewiresrc/x264enc.
        assert!(GST_ENV_BLOCKLIST.contains(&"LD_LIBRARY_PATH"));
        assert!(GST_ENV_BLOCKLIST.contains(&"LD_PRELOAD"));
        assert!(GST_ENV_BLOCKLIST.contains(&"GST_PLUGIN_SYSTEM_PATH_1_0"));
        assert!(GST_ENV_BLOCKLIST.contains(&"GST_PLUGIN_PATH"));
    }

    #[test]
    fn capabilities_are_available_with_a_complete_gstreamer() {
        let capabilities = decide_capabilities(true, true, true, &[]);
        assert!(capabilities.available);
        assert_eq!(capabilities.encoder, "x264");
        assert_eq!(capabilities.reason_code, None);
        assert!(!capabilities.follow_supported);
        assert_eq!(capabilities.target_mode, "portal");
    }

    #[test]
    fn unavailable_portal_capabilities_keep_the_portal_target_mode() {
        let capabilities = decide_capabilities(false, true, true, &[]);
        assert_eq!(capabilities.target_mode, "portal");
        assert!(!capabilities.follow_supported);
    }

    fn real_spec(kind: RecordingKind, dir: &Path) -> RecordingSpec {
        RecordingSpec {
            kind,
            fps: 8,
            resolution: "native".to_string(),
            quality: "balanced".to_string(),
            encoder: "x264".to_string(),
            audio_kbps: 96,
            container: "mp4".to_string(),
            target: RecordingTarget::Screen {
                label: "Tela (portal)".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
            },
            keep_segments: true,
            microphone: true,
            system: true,
            output_dir: dir.to_path_buf(),
        }
    }

    /// Teste manual do caminho completo: portal + pausa/retomada + finalização.
    ///
    /// Roda com `cargo test --lib -- --ignored records_and_finalizes_a_real_portal_stream`.
    /// O seletor do GNOME aparece; escolha uma tela/janela e autorize.
    #[test]
    #[ignore = "abre o seletor do GNOME e grava ~10 s"]
    fn records_and_finalizes_a_real_portal_stream() {
        use crate::adapters::recording::ffmpeg_path::resolve_ffmpeg;
        use crate::adapters::recording::finalizer::FfmpegRecordingFinalizer;
        use crate::ports::screen_recorder::{FinalizeRequest, RecordingFinalizer};

        let ffmpeg = resolve_ffmpeg().expect("ffmpeg");
        let recorder = PortalScreenRecorder::new(ffmpeg.clone());
        let capabilities = recorder.capabilities();
        assert!(
            capabilities.available,
            "capabilities: {:?}",
            capabilities.reason
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let spec = real_spec(RecordingKind::Video, dir.path());
        let parts_dir = dir.path().join("parts");
        recorder
            .start(&RecordingStart {
                spec: spec.clone(),
                segments_dir: parts_dir,
            })
            .expect("start (escolha a fonte no GNOME)");
        std::thread::sleep(Duration::from_secs(5));
        recorder.pause().expect("pause");
        std::thread::sleep(Duration::from_secs(1));
        recorder.resume().expect("resume");
        std::thread::sleep(Duration::from_secs(4));
        recorder.stop().expect("stop");

        let parts = recorder.segments();
        assert_eq!(parts.len(), 2, "um segmento por trecho: {parts:?}");
        for part in &parts {
            let size = std::fs::metadata(part).map(|meta| meta.len()).unwrap_or(0);
            assert!(size > 0, "segmento vazio: {part:?}");
        }
        let durations = recorder.segment_durations_ms();
        assert!(durations.iter().sum::<i64>() > 5_000, "{durations:?}");

        let finalizer = FfmpegRecordingFinalizer::new(ffmpeg);
        let output = dir.path().join("saida.mp4");
        let artifact = finalizer
            .finalize(
                &FinalizeRequest {
                    parts,
                    output,
                    spec,
                    total_ms: durations.iter().sum(),
                    width: 1920,
                    height: 1080,
                    normalize: false,
                    audio: None,
                },
                &|_| {},
            )
            .expect("finalize");
        assert!(artifact.size_bytes > 0, "arquivo final vazio");
        assert!(artifact.path.exists());

        // Inspeção externa: o MP4 precisa ter vídeo, áudio e a duração dos
        // dois trechos (5 s + 4 s), não só um arquivo com tamanho > 0.
        let probe_streams = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(&artifact.path)
            .output()
            .expect("ffprobe");
        let streams = String::from_utf8_lossy(&probe_streams.stdout);
        assert!(streams.contains("video"), "sem vídeo: {streams}");
        assert!(streams.contains("audio"), "sem áudio: {streams}");

        let probe_duration = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(&artifact.path)
            .output()
            .expect("ffprobe");
        let seconds: f64 = String::from_utf8_lossy(&probe_duration.stdout)
            .trim()
            .parse()
            .unwrap_or(0.0);
        assert!(seconds > 6.0, "duração inesperada: {seconds}");

        let sample = PathBuf::from("/tmp/opencode/portal-recording-test.mp4");
        let _ = std::fs::copy(&artifact.path, &sample);
        eprintln!(
            "gravação validada: {} bytes, {seconds:.1} s, amostra em {}",
            artifact.size_bytes,
            sample.display()
        );
    }

    /// Teste manual só de áudio (mic via GStreamer, sem portal).
    ///
    /// Roda com `cargo test --lib -- --ignored records_audio_only_with_gstreamer`.
    #[test]
    #[ignore = "grava o microfone por 3 s"]
    fn records_audio_only_with_gstreamer() {
        use crate::adapters::recording::ffmpeg_path::resolve_ffmpeg;
        use crate::adapters::recording::finalizer::FfmpegRecordingFinalizer;
        use crate::ports::screen_recorder::{FinalizeRequest, RecordingFinalizer};

        let ffmpeg = resolve_ffmpeg().expect("ffmpeg");
        let recorder = PortalScreenRecorder::new(ffmpeg.clone());
        let capabilities = recorder.capabilities();
        assert!(
            capabilities.available,
            "capabilities: {:?}",
            capabilities.reason
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let mut spec = real_spec(RecordingKind::Audio, dir.path());
        spec.microphone = true;
        spec.system = false;
        spec.container = "mp4".to_string();
        recorder
            .start(&RecordingStart {
                spec: spec.clone(),
                segments_dir: dir.path().join("parts"),
            })
            .expect("start");
        std::thread::sleep(Duration::from_secs(3));
        recorder.stop().expect("stop");

        let parts = recorder.segments();
        assert_eq!(parts.len(), 1, "{parts:?}");
        let finalizer = FfmpegRecordingFinalizer::new(ffmpeg);
        let output = dir.path().join("saida.flac");
        let artifact = finalizer
            .finalize(
                &FinalizeRequest {
                    parts,
                    output,
                    spec,
                    total_ms: recorder.segment_durations_ms().iter().sum(),
                    width: 0,
                    height: 0,
                    normalize: false,
                    audio: None,
                },
                &|_| {},
            )
            .expect("finalize");
        assert!(artifact.size_bytes > 0);
    }
}
