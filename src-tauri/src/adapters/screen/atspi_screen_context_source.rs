//! Adaptador de contexto de tela via **AT-SPI2** (D-Bus, BR-105).
//!
//! Lê a árvore de acessibilidade da janela **focada** (o evento
//! `object:state-changed:focus` funciona em X11 **e** Wayland — ele é gerado
//! pelo toolkit do app, não pelo compositor; ver `architecture/screen-context.md`
//! §3). A conexão é aberta **só** quando o adaptador é instanciado, e o
//! composition root só o instancia com `screen_context_enabled` ligada.
//!
//! Limites: caminhada com teto de nós e profundidade; texto extraído pelo
//! domain (`extract_text`). Logs nunca contêm conteúdo — só app, nós, chars e
//! latência (RNF-03).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use atspi::events::object::StateChangedEvent;
use atspi::events::{DBusMatchRule, RegistryEventString};
use atspi::proxy::accessible::AccessibleProxy;
use atspi::state::State;
use futures_util::StreamExt;

use crate::domain::screen_context::{
    extract_text, node_line, truncate_title, AccessibleNode, NodeRole, ScreenContextSnapshot,
    CONTEXT_MAX_CHARS,
};
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_context_source::ScreenContextSource;

/// Teto de nós visitados na caminhada (páginas enormes).
const MAX_NODES: usize = 5_000;
/// Profundidade máxima da árvore (segurança contra ciclos).
const MAX_DEPTH: usize = 64;
/// Orçamento interno do adaptador (o serviço espera 2 s; termina antes para
/// não deixar thread presa em D-Bus — ISS-098).
const ADAPTER_BUDGET: Duration = Duration::from_millis(1_800);

/// Callback chamado quando o foco muda (o serviço republica a pill).
pub type FocusCallback = Arc<dyn Fn() + Send + Sync>;

/// Nome estável publicado por [`ScreenContextSource::name`].
pub const ATSPI_SOURCE_NAME: &str = "atspi";

/// Erro genérico da camada AT-SPI (a mensagem nunca contém conteúdo).
type A11yResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Conexão AT-SPI **sem `p2p`** (ISS-098).
///
/// O crate `atspi` habilita a feature `p2p` por padrão e, com ela,
/// `A11yBus::connect()` enumera **todos os apps do registry**
/// (`Peers::initialize_peers`) a cada conexão: dezenas de roundtrips D-Bus que
/// podiam travar por tempo indeterminado. Como o snapshot abre uma conexão por
/// leitura, após alguns toggles sobravam threads presas e o contexto parava.
///
/// Aqui a conexão é só bus + registry + DBus; o proxy de cada objeto é montado
/// com o **unique name** do app como destino (o mesmo fallback de bus que o
/// `p2p` fazia, sem a enumeração).
struct A11yBus {
    bus: zbus::Connection,
    registry: atspi::proxy::registry::RegistryProxy<'static>,
    dbus: zbus::fdo::DBusProxy<'static>,
}

impl A11yBus {
    /// Conecta ao bus de acessibilidade (sessão → `org.a11y.Bus.GetAddress`).
    async fn connect() -> A11yResult<Self> {
        let session = zbus::Connection::session().await?;
        let reply = session
            .call_method(
                Some("org.a11y.Bus"),
                "/org/a11y/bus",
                Some("org.a11y.Bus"),
                "GetAddress",
                &(),
            )
            .await?;
        let address: String = reply.body().deserialize()?;
        let bus = zbus::connection::Builder::address(address.as_str())?
            .build()
            .await?;
        let registry = atspi::proxy::registry::RegistryProxy::new(&bus).await?;
        let dbus = zbus::fdo::DBusProxy::new(&bus).await?;
        Ok(Self {
            bus,
            registry,
            dbus,
        })
    }

    /// Conexão zbus subjacente.
    fn connection(&self) -> &zbus::Connection {
        &self.bus
    }

    /// Proxy `org.a11y.atspi.Accessible` para um nó (destino = unique name).
    async fn object_as_accessible<'a>(
        &'a self,
        object: &atspi::ObjectRefOwned,
    ) -> A11yResult<AccessibleProxy<'a>> {
        if object.is_null() {
            return Err("atspi: object ref nulo".to_string().into());
        }
        let name = object
            .name()
            .ok_or_else(|| Box::<dyn std::error::Error>::from("atspi: object ref sem nome"))?
            .to_owned();
        let path = object.path();
        Ok(AccessibleProxy::builder(&self.bus)
            .destination(zbus::names::BusName::from(name))?
            .path(path)?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?)
    }

    /// Raiz acessível do registry (lista de aplicações).
    async fn root_accessible_on_registry(&self) -> A11yResult<AccessibleProxy<'_>> {
        // O registry implementa `Accessible` na **raiz acessível**
        // (`/org/a11y/atspi/accessible/root`), não no path do `Registry`.
        Ok(AccessibleProxy::builder(&self.bus)
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?)
    }

    /// Assina um evento no registry e adiciona a regra de match no bus.
    async fn register_event<T: RegistryEventString + DBusMatchRule>(&self) -> A11yResult<()> {
        self.registry
            .register_event(<T as RegistryEventString>::REGISTRY_EVENT_STRING)
            .await?;
        let rule = zbus::MatchRule::try_from(<T as DBusMatchRule>::MATCH_RULE_STRING)?;
        self.dbus.add_match_rule(rule).await?;
        Ok(())
    }

    /// Stream de sinais AT-SPI já convertidos em [`atspi::events::Event`].
    fn event_stream(
        &self,
    ) -> impl futures_util::Stream<Item = A11yResult<atspi::events::Event>> + '_ {
        use zbus::message::Type as MessageType;
        zbus::MessageStream::from(&self.bus).filter_map(|message| async move {
            let message = match message {
                Ok(message) => message,
                Err(error) => return Some(Err(error.into())),
            };
            if message.message_type() != MessageType::Signal {
                return None;
            }
            let header = message.header();
            let Some(interface) = header.interface() else {
                return Some(Err("atspi: evento sem interface".to_string().into()));
            };
            if interface.as_str().starts_with("org.a11y.atspi") {
                Some(atspi::events::Event::try_from(&message).map_err(Into::into))
            } else {
                None
            }
        })
    }
}

/// Fonte de contexto de tela lida do AT-SPI.
pub struct AtspiScreenContextSource {
    /// Último nó focado observado (atualizado pela thread de eventos).
    focused: Arc<Mutex<Option<atspi::ObjectRefOwned>>>,
    /// Callback de mudança de foco (pill acompanha sem reabrir a barra).
    /// Compartilhado com a thread de eventos (BR-106).
    on_focus: Arc<Mutex<Option<FocusCallback>>>,
}

impl AtspiScreenContextSource {
    /// Cria a fonte e inicia a thread de escuta de foco.
    ///
    /// A thread conecta ao bus de acessibilidade e guarda o último elemento
    /// com `State::Focused`; sem conexão, o adaptador permanece sem foco
    /// observado (`snapshot()` devolve `None` → pill "Sem contexto").
    pub fn new() -> Self {
        let focused = Arc::new(Mutex::new(None));
        let on_focus: Arc<Mutex<Option<FocusCallback>>> = Arc::new(Mutex::new(None));
        spawn_focus_listener(Arc::clone(&focused), Arc::clone(&on_focus));
        Self { focused, on_focus }
    }

    /// Registra o callback de mudança de foco (chamado no boot pelo
    /// composition root, após criar o serviço).
    pub fn set_focus_callback(&self, callback: FocusCallback) {
        if let Ok(mut slot) = self.on_focus.lock() {
            *slot = Some(callback);
        }
    }

    /// Node do AT-SPI → DTO do domain (papéis estruturais só navegam).
    fn map_role(role: atspi::Role) -> NodeRole {
        use atspi::Role;
        match role {
            // Containers: navegam, mas não contribuem texto por si.
            Role::Application
            | Role::Frame
            | Role::Window
            | Role::Dialog
            | Role::Panel
            | Role::Filler
            | Role::ScrollPane
            | Role::RootPane
            | Role::InternalFrame
            | Role::LayeredPane
            | Role::PageTabList
            | Role::Viewport
            | Role::ToolBar
            | Role::MenuBar
            | Role::StatusBar
            | Role::TitleBar
            | Role::Separator
            // `PasswordText` é tratado como estrutura: nunca extrair valor
            // de campo de senha (RNF-03).
            | Role::PasswordText => NodeRole::Structure,
            // Texto semântico: contribui `name`.
            Role::Static
            | Role::Label
            | Role::Heading
            | Role::Link
            | Role::Paragraph
            | Role::BlockQuote
            | Role::Caption
            | Role::ListItem
            | Role::Article
            | Role::Section
            | Role::Comment
            | Role::Definition
            | Role::Footer
            | Role::Header
            | Role::Landmark => NodeRole::Text,
            // Campos editáveis: contribui `value`.
            Role::Entry | Role::Text | Role::Editbar => NodeRole::Field,
            // O resto (botões, menus, células…): contribui `name` se houver.
            _ => NodeRole::Other,
        }
    }

    /// Caminha a árvore a partir de `root` montando o DTO do domain.
    ///
    /// Limites: `MAX_NODES` visitados e `MAX_DEPTH` de profundidade — o que
    /// vier depois é cortado (o orçamento de chars é do domain).
    async fn walk(
        connection: &A11yBus,
        root: &AccessibleProxy<'_>,
        visited: &mut usize,
        text_chars: &mut usize,
        depth: usize,
    ) -> AccessibleNode {
        let role = root.get_role().await.unwrap_or(atspi::Role::Unknown);
        // `Defunct` = o widget foi destruído; `Visible` sem `Showing` é
        // conteúdo fora do viewport e **entra** (é o documento todo).
        let invisible = root
            .get_state()
            .await
            .map(|states| states.contains(State::Defunct))
            .unwrap_or(false);

        let mut node = AccessibleNode {
            role: Self::map_role(role),
            name: root.name().await.unwrap_or_default(),
            value: String::new(),
            invisible,
            children: Vec::new(),
        };
        *visited += 1;

        // Campos: o valor vem da interface Text quando existir; caso
        // contrário, o `name` já cobre (o domain faz fallback).
        if node.role == NodeRole::Field {
            let destination = root.inner().destination().clone();
            let path = root.inner().path().clone();
            let built = atspi::proxy::text::TextProxy::builder(connection.connection())
                .destination(destination)
                .and_then(|builder| builder.path(path));
            if let Ok(builder) = built {
                if let Ok(text_proxy) = builder.build().await {
                    if let Ok(text) = text_proxy.get_text(0, -1).await {
                        node.value = text;
                    }
                }
            }
        }

        // Parada antecipada (ISS-092): o texto útil acumulado já cobre o
        // orçamento do domain — não precisa visitar o resto da árvore (o
        // Chrome tem milhares de nós; cada um são roundtrips D-Bus). O
        // critério é o mesmo do domain (`node_line`), sem divergir da poda.
        if let Some(line) = node_line(&node) {
            *text_chars += line.chars().count();
            if *text_chars >= CONTEXT_MAX_CHARS {
                return node;
            }
        }

        if *visited >= MAX_NODES || depth >= MAX_DEPTH {
            return node;
        }

        if let Ok(children) = root.get_children().await {
            for child in children {
                if *visited >= MAX_NODES || *text_chars >= CONTEXT_MAX_CHARS {
                    break;
                }
                // `ObjectRef` nulo: app registrou um filho vazio.
                let Ok(child_proxy) = connection.object_as_accessible(&child).await else {
                    continue;
                };
                // Recursão async exige boxing (E0733).
                let child_node = Box::pin(Self::walk(
                    connection,
                    &child_proxy,
                    visited,
                    text_chars,
                    depth + 1,
                ))
                .await;
                node.children.push(child_node);
            }
        }
        node
    }

    /// Resolve a janela ativa quando **nenhum foco foi observado** (ISS-093).
    ///
    /// O Chrome já focado antes do boot do listener não gera evento — sem
    /// isso a primeira AskBar ficaria sem contexto. Caminha as aplicações do
    /// registry procurando a janela com `State::Active` (janela em primeiro
    /// plano). Limites: 50 apps × 20 janelas — custo de dezenas de roundtrips,
    /// só no caminho de fallback.
    async fn find_active_window(connection: &A11yBus) -> Option<atspi::ObjectRefOwned> {
        const MAX_APPS: usize = 50;
        const MAX_WINDOWS: usize = 20;

        let root = connection.root_accessible_on_registry().await.ok()?;
        let apps = root.get_children().await.ok()?;
        let mut windows_checked = 0usize;
        for app_ref in apps.iter().take(MAX_APPS) {
            let Ok(app) = connection.object_as_accessible(app_ref).await else {
                continue;
            };
            let Ok(windows) = app.get_children().await else {
                continue;
            };
            for window_ref in windows.iter().take(MAX_WINDOWS) {
                windows_checked += 1;
                let Ok(window) = connection.object_as_accessible(window_ref).await else {
                    continue;
                };
                let active = window
                    .get_state()
                    .await
                    .map(|states| states.contains(State::Active))
                    .unwrap_or(false);
                if active {
                    tracing::debug!(
                        apps = apps.len(),
                        windows_checked,
                        "atspi: janela ativa encontrada no seed"
                    );
                    return Some(window_ref.clone());
                }
            }
        }
        tracing::debug!(
            apps = apps.len(),
            windows_checked,
            "atspi: seed não achou janela com State::Active"
        );
        None
    }

    /// Sobe os pais do elemento focado até achar a janela (título) e o app.
    async fn ancestors<'a>(
        connection: &'a A11yBus,
        focused: &AccessibleProxy<'_>,
    ) -> (String, Option<AccessibleProxy<'a>>) {
        let mut window_title = String::new();
        let mut current = focused.parent().await.ok();
        let mut app_ref = focused.get_application().await.ok();

        for _ in 0..MAX_DEPTH {
            let Some(object_ref) = current else { break };
            if app_ref.is_none() {
                app_ref = Some(object_ref.clone());
            }
            let Ok(proxy) = connection.object_as_accessible(&object_ref).await else {
                break;
            };
            match proxy.get_role().await.unwrap_or(atspi::Role::Unknown) {
                atspi::Role::Frame | atspi::Role::Window | atspi::Role::Dialog => {
                    window_title = proxy.name().await.unwrap_or_default();
                    // A janela foi encontrada; o app vem do get_application.
                    app_ref = proxy.get_application().await.ok().or(app_ref);
                    break;
                }
                _ => current = proxy.parent().await.ok(),
            }
        }

        let app_proxy = match app_ref {
            Some(object_ref) => connection.object_as_accessible(&object_ref).await.ok(),
            None => None,
        };
        (truncate_title(&window_title), app_proxy)
    }

    /// Snapshot completo com uma conexão/robô async dedicado.
    fn snapshot_inner(&self) -> Result<Option<ScreenContextSnapshot>> {
        let started = Instant::now();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| GhostWhisperError::Other(format!("runtime atspi: {error}")))?;

        let focused = self.focused.lock().expect("lock").clone();
        // Slot compartilhado: o resultado do seed vira foco observado (ISS-098).
        let focused_slot = Arc::clone(&self.focused);
        let from_seed = focused.is_none();

        match runtime.block_on(async move {
            tokio::time::timeout(
                ADAPTER_BUDGET,
                Self::snapshot_connected(focused, focused_slot, from_seed, started),
            )
            .await
        }) {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    budget_ms = ADAPTER_BUDGET.as_millis() as u64,
                    "atspi: snapshot excedeu o orçamento"
                );
                Err(GhostWhisperError::Other(
                    "contexto de tela: timeout no adaptador".into(),
                ))
            }
        }
    }

    /// Corpo do snapshot (conexão aberta + orçamento aplicado por quem chama).
    async fn snapshot_connected(
        focused: Option<atspi::ObjectRefOwned>,
        focused_slot: Arc<Mutex<Option<atspi::ObjectRefOwned>>>,
        from_seed: bool,
        started: Instant,
    ) -> Result<Option<ScreenContextSnapshot>> {
        let connection = A11yBus::connect().await.map_err(|error| {
            tracing::warn!("atspi: sem bus de acessibilidade: {error}");
            GhostWhisperError::Other("atspi indisponível".into())
        })?;
        tracing::debug!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "atspi: conectado ao bus"
        );

        // Sem evento de foco observado (Chrome já focado **antes** do
        // boot não gera evento): resolve a janela ativa agora — ISS-093.
        let focused_ref = match focused {
            Some(focused_ref) => Some(focused_ref),
            None => {
                let seed_started = Instant::now();
                let found = Self::find_active_window(&connection).await;
                tracing::debug!(
                    found = found.is_some(),
                    elapsed_ms = seed_started.elapsed().as_millis() as u64,
                    "atspi: busca da janela ativa (seed)"
                );
                found
            }
        };
        let Some(focused_ref) = focused_ref else {
            return Ok(None);
        };
        let Ok(focused_proxy) = connection.object_as_accessible(&focused_ref).await else {
            // O nó focado expirou (app fechou) — sem contexto.
            return Ok(None);
        };

        let (window_title, app_proxy) = Self::ancestors(&connection, &focused_proxy).await;
        tracing::debug!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "atspi: ancestrais resolvidos"
        );
        let Some(app_proxy) = app_proxy else {
            tracing::debug!("atspi: ancestors sem nó de aplicação → None");
            return Ok(None);
        };
        let app = app_proxy.name().await.unwrap_or_default();
        if app.trim().is_empty() {
            tracing::debug!("atspi: app sem nome → None");
            return Ok(None);
        }
        // Defesa em profundidade (ISS-093): nunca servir a própria janela
        // nem uma shell de desktop, mesmo que o foco tenha passado sem o
        // filtro do listener (ex.: daemon D-Bus indisponível lá).
        if is_desktop_shell(&app) {
            tracing::debug!(app = %app, "atspi: shell de desktop → None");
            return Ok(None);
        }
        let own_bus = app_proxy.inner().destination().clone().into_owned();
        let is_own = {
            let dbus = zbus::fdo::DBusProxy::new(connection.connection())
                .await
                .ok();
            match dbus {
                Some(dbus) => dbus
                    .get_connection_unix_process_id(own_bus)
                    .await
                    .map(|pid| pid == std::process::id())
                    .unwrap_or(false),
                None => false,
            }
        };
        if is_own {
            tracing::debug!(app = %app, "atspi: é o próprio processo → None");
            return Ok(None);
        }
        // Sem evento de foco (app focado antes do boot), o seed achou a
        // janela real: guarda como foco para as próximas leituras — a
        // barra rouba o foco e o seed passaria a encontrar a si mesma.
        if from_seed {
            if let Ok(mut slot) = focused_slot.lock() {
                *slot = Some(focused_ref.clone());
            }
            tracing::debug!(app = %app, "atspi: seed promovido a foco observado");
        }
        tracing::debug!(app = %app, "atspi: app resolvido, caminhando");

        let mut visited = 0usize;
        let mut text_chars = 0usize;
        let root = Self::walk(&connection, &app_proxy, &mut visited, &mut text_chars, 0).await;
        let text = extract_text(&root, CONTEXT_MAX_CHARS);
        let elapsed_ms = started.elapsed().as_millis() as u64;

        // Log só com métricas — nunca app/título/conteúdo (RNF-03).
        tracing::debug!(
            nodes = visited,
            chars = text.chars().count(),
            elapsed_ms,
            "contexto de tela lido"
        );

        if text.trim().is_empty() && window_title.trim().is_empty() {
            tracing::debug!(app = %app, "atspi: sem texto e sem título → None");
            return Ok(None);
        }

        Ok(Some(ScreenContextSnapshot {
            app,
            window_title,
            text,
            captured_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or_default(),
        }))
    }
}

impl Default for AtspiScreenContextSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenContextSource for AtspiScreenContextSource {
    fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>> {
        self.snapshot_inner()
    }

    fn name(&self) -> &'static str {
        ATSPI_SOURCE_NAME
    }
}

/// Thread própria que escuta `object:state-changed` e guarda o elemento
/// focado, notificando o callback (a pill atualiza sem reabrir a barra —
/// CA-02.5). Sem bus acessível, fica ociosa (o snapshot devolve `None`).
///
/// **Filtro de foco (ISS-093):** o próprio GhostWhisper (a AskBar rouba o
/// foco ao abrir) e shells de desktop **não sobrescrevem** o foco guardado —
/// a pill continua mostrando a última janela real que o usuário estava vendo.
fn spawn_focus_listener(
    focused: Arc<Mutex<Option<atspi::ObjectRefOwned>>>,
    on_focus: Arc<Mutex<Option<FocusCallback>>>,
) {
    std::thread::Builder::new()
        .name("atspi-focus".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!("atspi-focus: sem runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                let Ok(connection) = A11yBus::connect().await else {
                    // Sem D-Bus de acessibilidade: sem foco observado. Não é
                    // erro fatal — a pill mostra "Sem contexto"/"Indisponível".
                    tracing::info!("atspi-focus: bus de acessibilidade indisponível");
                    return;
                };
                if connection
                    .register_event::<StateChangedEvent>()
                    .await
                    .is_err()
                {
                    tracing::info!("atspi-focus: registro de eventos falhou");
                    return;
                }
                // Proxy do daemon D-Bus (mesmo bus) para resolver o pid do
                // dono de um node — identificação do próprio processo.
                let own_pid = std::process::id();
                let dbus = zbus::fdo::DBusProxy::new(connection.connection())
                    .await
                    .ok();
                let mut stream = std::pin::pin!(connection.event_stream());
                while let Some(event) = stream.next().await {
                    let Ok(event) = event else { continue };
                    let Ok(state_changed) = StateChangedEvent::try_from(event) else {
                        continue;
                    };
                    if state_changed.state != State::Focused || !state_changed.enabled {
                        continue;
                    }
                    // Filtro ISS-093: nosso processo / shells de desktop não
                    // sobrescrevem o foco real (a AskBar rouba o foco ao abrir).
                    if should_ignore_focus(&connection, dbus.as_ref(), &state_changed.item, own_pid)
                        .await
                    {
                        tracing::debug!("foco interno ignorado (ISS-093)");
                        continue;
                    }
                    if let Ok(mut slot) = focused.lock() {
                        *slot = Some(state_changed.item);
                    }
                    // Notificação fora do lock (o callback pode ler o estado).
                    let callback = on_focus.lock().ok().and_then(|slot| slot.clone());
                    if let Some(callback) = callback {
                        callback();
                    }
                }
            });
        })
        .expect("spawn atspi-focus");
}

/// Shells de desktop: nunca são "contexto de tela" útil (ISS-093).
const DESKTOP_SHELLS: &[&str] = &[
    "gnome-shell",
    "gnome shell",
    "org.gnome.shell",
    "plasmashell",
    "ksplash",
];

/// `true` quando o nome do app é uma shell de desktop (ISS-093).
fn is_desktop_shell(app_name: &str) -> bool {
    let name = app_name.trim().to_ascii_lowercase().replace(' ', "");
    DESKTOP_SHELLS
        .iter()
        .any(|shell| name == shell.replace(' ', ""))
}

/// `true` quando o pid resolve ao **nosso processo** (a AskBar rouba o foco
/// ao abrir — sem o filtro o snapshot viraria o próprio GhostWhisper).
fn is_own_process(pid: Option<u32>, own_pid: u32) -> bool {
    pid.is_some_and(|pid| pid == own_pid)
}

/// `true` quando o foco deve ser **descartado** — não sobrescreve o foco
/// guardado, então a pill mantém a última janela real do usuário:
///
/// 1. node pertence ao **nosso processo** (pid igual);
/// 2. node vem de uma **shell de desktop** (gnome-shell etc.).
async fn should_ignore_focus(
    connection: &A11yBus,
    dbus: Option<&zbus::fdo::DBusProxy<'_>>,
    item: &atspi::ObjectRefOwned,
    own_pid: u32,
) -> bool {
    // 1) Nosso processo: o bus name do node resolve no pid pelo daemon.
    if let (Some(dbus), Some(unique)) = (dbus, item.name()) {
        let bus_name = zbus::names::BusName::from(unique.clone());
        if let Ok(pid) = dbus.get_connection_unix_process_id(bus_name).await {
            if is_own_process(Some(pid), own_pid) {
                return true;
            }
        }
    }

    // 2) Shell de desktop: nome do nó de aplicação dono do node.
    let Ok(proxy) = connection.object_as_accessible(item).await else {
        return false;
    };
    let Ok(app_ref) = proxy.get_application().await else {
        return false;
    };
    let Ok(app) = connection.object_as_accessible(&app_ref).await else {
        return false;
    };
    app.name()
        .await
        .map(|name| is_desktop_shell(&name))
        .unwrap_or(false)
}

/// Converte um `Event` bruto no evento de estado (helper p/ testes).
#[cfg(test)]
fn is_focus_event(event: &atspi::Event) -> bool {
    StateChangedEvent::try_from(event.clone())
        .map(|state| state.state == State::Focused && state.enabled)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use atspi::Event;

    use crate::domain::screen_context::TITLE_MAX_CHARS;

    #[test]
    fn role_mapping_keeps_password_text_out_of_text() {
        // Senha nunca vira texto extraível (RNF-03).
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::PasswordText),
            NodeRole::Structure
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Application),
            NodeRole::Structure
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Heading),
            NodeRole::Text
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Paragraph),
            NodeRole::Text
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Entry),
            NodeRole::Field
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Text),
            NodeRole::Field
        );
        assert_eq!(
            AtspiScreenContextSource::map_role(atspi::Role::Button),
            NodeRole::Other
        );
    }

    #[test]
    fn title_truncation_uses_domain_budget() {
        let long = "t".repeat(TITLE_MAX_CHARS + 10);
        assert!(truncate_title(&long).chars().count() <= TITLE_MAX_CHARS);
    }

    #[test]
    fn source_reports_atspi_name() {
        // O construtor inicia a thread de foco; em ambiente sem D-Bus ela
        // simplesmente sai — o `name()` continua estável.
        let source = AtspiScreenContextSource::new();
        assert_eq!(source.name(), ATSPI_SOURCE_NAME);
    }

    #[test]
    fn desktop_shells_are_recognized() {
        // ISS-093: shell de desktop nunca é contexto.
        assert!(is_desktop_shell("gnome-shell"));
        assert!(is_desktop_shell("GNOME Shell"));
        assert!(is_desktop_shell("org.gnome.Shell"));
        assert!(is_desktop_shell("plasmashell"));
        assert!(is_desktop_shell("KSplash"));
        // Apps comuns continuam valendo.
        assert!(!is_desktop_shell("google-chrome"));
        assert!(!is_desktop_shell("Code"));
        assert!(!is_desktop_shell("gnome-terminal"));
        // "gnome-shell-something" não é a shell (comparação exata normalizada).
        assert!(!is_desktop_shell("gnome-shell-extension-app"));
    }

    #[test]
    fn own_process_focus_is_ignored() {
        // ISS-093: a AskBar rouba o foco ao abrir — o snapshot do próprio
        // GhostWhisper é descartado.
        assert!(is_own_process(Some(4242), 4242));
        assert!(!is_own_process(Some(1), 4242));
        assert!(
            !is_own_process(None, 4242),
            "pid indeterminado não descarta"
        );
    }

    #[test]
    fn focus_event_filter_accepts_only_focused_enabled() {
        let focused = StateChangedEvent {
            state: State::Focused,
            enabled: true,
            ..StateChangedEvent::default()
        };
        assert!(is_focus_event(&Event::from(focused.clone())));

        let disabled = StateChangedEvent {
            enabled: false,
            ..focused.clone()
        };
        assert!(!is_focus_event(&Event::from(disabled)));

        let other_state = StateChangedEvent {
            state: State::Active,
            ..focused
        };
        assert!(!is_focus_event(&Event::from(other_state)));
    }

    /// Requer sessão D-Bus de acessibilidade real (at-spi2 no ar).
    ///
    /// Roda com `cargo test --lib atspi_live -- --ignored --nocapture`.
    /// Não assertion de conteúdo (depende do ambiente): só o contrato —
    /// `Ok(None)` (sem foco/árvore) ou `Ok(Some(_))` com métricas no log.
    #[test]
    #[ignore = "requer sessão AT-SPI real (at-spi2 no ar)"]
    fn atspi_live_snapshot_respects_contract() {
        // Logs de fase do adaptador (ISS-093 diagnóstico): sem subscriber os
        // `tracing::debug` do adaptador não aparecem.
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .try_init();

        let source = AtspiScreenContextSource::new();
        // Dá tempo para a thread de foco observar algo.
        std::thread::sleep(Duration::from_millis(400));
        let started = Instant::now();
        match source.snapshot() {
            Ok(Some(snapshot)) => {
                assert!(!snapshot.app.trim().is_empty(), "app sempre presente");
                assert!(
                    snapshot.text.chars().count() <= CONTEXT_MAX_CHARS,
                    "orçamento do domain respeitado"
                );
                println!(
                    "atspi ok em {}ms: app={}, title={}, chars={}",
                    started.elapsed().as_millis(),
                    snapshot.app,
                    snapshot.window_title,
                    snapshot.text.chars().count()
                );
            }
            Ok(None) => println!(
                "atspi ok: sem contexto (sem foco/árvore) em {}ms",
                started.elapsed().as_millis()
            ),
            Err(error) => panic!("mecanismo inacessível não deveria ser Err aqui: {error}"),
        }
    }

    /// Sonda de diagnóstico (ISS-093): imprime a estrutura da árvore AT-SPI
    /// (só roles/profundidade — sem conteúdo de texto) dos apps registrados.
    ///
    /// Conecta, **espera 3 s** (ativação da árvore pelos apps) e refaz a
    /// leitura — discrimina "árvore vazia" de "ainda ativando".
    /// `cargo test --lib atspi_tree_probe -- --ignored --nocapture`
    #[test]
    #[ignore = "requer sessão AT-SPI real (at-spi2 no ar)"]
    fn atspi_tree_probe() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let connection = match A11yBus::connect().await {
                Ok(connection) => connection,
                Err(error) => panic!("sem bus: {error}"),
            };
            println!("--- leitura imediata ---");
            dump_apps(&connection).await;
            println!("\n--- após 3s de conexão (ativação dos apps) ---");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            dump_apps(&connection).await;
        });
    }

    async fn dump_apps(connection: &A11yBus) {
        let root = connection
            .root_accessible_on_registry()
            .await
            .expect("root");
        let apps = root.get_children().await.expect("apps");
        println!("apps registrados: {}", apps.len());
        for app_ref in apps.iter() {
            let Ok(app) = connection.object_as_accessible(app_ref).await else {
                continue;
            };
            let name = app.name().await.unwrap_or_default();
            if name != "code" && name != "Google Chrome" {
                continue;
            }
            let role = app.get_role().await.unwrap_or(atspi::Role::Unknown);
            println!("\n== app '{name}' role={:?}", role);
            dump_tree(connection, &app, 0, 3).await;
            // ObjectRef cru do primeiro frame (diagnóstico GetChildren).
            if let Ok(frames) = app.get_children().await {
                if let Some(frame) = frames.first() {
                    println!(
                        "frame ref: bus={:?} path={}",
                        frame.name().map(|n| n.as_str()),
                        frame.path_as_str()
                    );
                }
            }
        }
    }

    /// Imprime a estrutura (role + nº de filhos) até `max_depth`.
    async fn dump_tree(
        connection: &A11yBus,
        node: &AccessibleProxy<'_>,
        depth: usize,
        max_depth: usize,
    ) {
        let role = node.get_role().await.unwrap_or(atspi::Role::Unknown);
        let children = node.get_children().await.unwrap_or_default();
        let states = node.get_state().await.ok();
        let flags: Vec<&str> = match &states {
            Some(states) => {
                let mut flags = Vec::new();
                if states.contains(State::Active) {
                    flags.push("Active");
                }
                if states.contains(State::Focused) {
                    flags.push("Focused");
                }
                if states.contains(State::Showing) {
                    flags.push("Showing");
                }
                if states.contains(State::Visible) {
                    flags.push("Visible");
                }
                flags
            }
            None => Vec::new(),
        };
        println!(
            "{:indent$}- {:?} filhos={} {}",
            "",
            role,
            children.len(),
            flags.join(","),
            indent = depth * 2
        );
        if depth >= max_depth {
            return;
        }
        for child in children.iter().take(8) {
            match connection.object_as_accessible(child).await {
                Ok(proxy) => {
                    Box::pin(dump_tree(connection, &proxy, depth + 1, max_depth)).await;
                }
                Err(error) => {
                    println!(
                        "{:indent$}- FALHA object_as_accessible: {error} (ref={child:?})",
                        "",
                        indent = (depth + 1) * 2
                    );
                }
            }
        }
    }
}
