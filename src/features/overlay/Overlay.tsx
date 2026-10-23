import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  closeMenu as closeMenuIpc,
  openMenu as openMenuIpc,
  openSettingsWindow,
  setActionbarWidth as setActionbarWidthIpc,
  setStealth as setStealthIpc,
  setSurface as setSurfaceIpc,
  startDrag as startDragIpc,
} from "../../adapters/ipc/overlay";
import { subscribe } from "../../adapters/ipc/events";
import { panelHide, panelOpen, panelOpenTab } from "../../adapters/ipc/panels";
import { askOpen } from "../../adapters/ipc/ask";
import { recordingPickerOpen } from "../../adapters/ipc/recording";
import { sessionEndOpen } from "../../adapters/ipc/sessionEnd";
import { useAsrStatus } from "../../core/application/useAsrStatus";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import { useInteractiveRegions } from "../../core/application/useInteractiveRegions";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useRecording } from "../../core/application/useRecording";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { defaultBindings } from "../../core/domain/shortcuts";
import { DwellProvider } from "../../components/dwell/DwellProvider";
import { ActionBar } from "./ActionBar";
import { AgentSwitcher } from "./AgentSwitcher";
import { AsrStatusBanner } from "./AsrStatusBanner";

/**
 * Casca do overlay: apenas a barra de ações (e o aviso de status do ASR). O
 * painel (Transcrição/Resumo), Sugestões e Histórico vivem em janelas próprias,
 * abertas pelo switcher/ícones da barra (BR-032/BR-033).
 *
 * Apenas elementos `[data-interactive]` capturam o mouse; o restante é
 * click-through (input-shape por região no X11).
 */
export function Overlay() {
  const [stealth, setStealth] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ kind: "agent" | "channel" } | null>(null);
  const stackRef = useRef<HTMLDivElement>(null);
  const { state, busy, error, start, pause, resume, stop } = useSessionState();
  const {
    state: recording,
    capabilities: recordingCapabilities,
    busy: recordingBusy,
    toggle: toggleRecording,
  } = useRecording();
  const asrStatus = useAsrStatus();
  const { settings } = useSettings();
  // O catálogo do seletor só recarrega quando os **agentes** mudam (o overlay
  // também recebe `settings_changed` da opacidade; recarregar a lista com os
  // prompts a cada autosave seria desperdício de IPC).
  const modesKey = settings.custom_assistant_modes
    .map((mode) => `${mode.id}:${mode.name}:${mode.icon}:${mode.color}`)
    .join("|");
  const { modes } = useAssistantModes(modesKey);
  const suggestionsOpen = usePanelOpen("suggestions");
  const historyOpen = usePanelOpen("history");
  useInteractiveRegions();

  // A janela do notch nunca rola (ISS-061): sem barras de rolagem, nada
  // "esconde" a barra quando o conteúdo cresce. A janela acompanha a pilha.
  useEffect(() => {
    document.documentElement.classList.add("overlay-window");
    return () => {
      document.documentElement.classList.remove("overlay-window");
    };
  }, []);

  // Notch dinâmico: mede a pilha (seletor + barra) e pede ao core a largura
  // exata. A janela acompanha os itens (cresce/contrai) sem sobra fixa; o core
  // reancora em topo-centro/direita e preserva a posição em modo livre.
  useLayoutEffect(() => {
    const node = stackRef.current;
    if (!node || typeof ResizeObserver === "undefined") {
      return;
    }
    let frame = 0;
    let lastWidth = 0;
    const publish = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const width = Math.ceil(node.getBoundingClientRect().width);
        if (width > 0 && Math.abs(width - lastWidth) >= 1) {
          lastWidth = width;
          void setActionbarWidthIpc(width);
        }
      });
    };
    const observer = new ResizeObserver(publish);
    observer.observe(node);
    publish();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);

  /** Abre o menu flutuante (janela própria) ancorado no gatilho. */
  const openMenu = useCallback(
    (kind: "agent" | "channel", rect: DOMRect) => {
      // Clicar de novo no mesmo gatilho fecha (dropdown normal).
      if (menu?.kind === kind) {
        setMenu(null);
        void closeMenuIpc();
        return;
      }
      const notch = stackRef.current?.getBoundingClientRect();
      if (!notch) {
        return;
      }
      setMenu({ kind });
      void openMenuIpc(kind, {
        trigger: {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        },
        notch: {
          x: Math.round(notch.x),
          y: Math.round(notch.y),
          width: Math.round(notch.width),
          height: Math.round(notch.height),
        },
      });
    },
    [menu],
  );

  const openAgentMenu = useCallback(
    (rect: DOMRect) => openMenu("agent", rect),
    [openMenu],
  );

  const openChannelMenu = useCallback(
    (rect: DOMRect) => openMenu("channel", rect),
    [openMenu],
  );

  // Menu aberto: clique em qualquer outro ponto do notch (ou Escape) fecha o
  // popup. O clique no próprio gatilho é ignorado aqui — o `onClick` alterna.
  useEffect(() => {
    if (!menu) {
      return;
    }
    const close = () => {
      setMenu(null);
      void closeMenuIpc();
    };
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Element | null;
      const trigger =
        menu.kind === "agent" ? ".agent-switcher" : ".channel-menu";
      if (target?.closest(trigger)) {
        return;
      }
      close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [menu]);

  // Fechado pelo menu (item escolhido, Escape, perda de foco): sincroniza o
  // estado local para o próximo clique não reenviar um close desnecessário.
  useEffect(() => {
    let stop: (() => void) | undefined;
    void subscribe<null>("menu_closed", () => setMenu(null)).then(
      (unlisten) => {
        stop = unlisten;
      },
    );
    return () => stop?.();
  }, []);

  // Sincroniza o stealth com as settings persistidas.
  useEffect(() => {
    setStealth(settings.stealth);
  }, [settings.stealth]);

  // Atalho exibido ao lado de "Começar a ouvir" (badges). Deriva do binding
  // persistido; cai no padrão quando as settings ainda não carregaram.
  const startAccelerator =
    settings.shortcuts.find((binding) => binding.action === "toggle_session")
      ?.accelerator ??
    defaultBindings().find((binding) => binding.action === "toggle_session")!
      .accelerator;
  const askAccelerator =
    settings.shortcuts.find((binding) => binding.action === "focus_ask")
      ?.accelerator ??
    defaultBindings().find((binding) => binding.action === "focus_ask")!
      .accelerator;

  const toggleStealth = () => {
    const next = !stealth;
    setStealth(next);
    void setStealthIpc(next);
  };

  const openSettings = async () => {
    setSettingsError(null);
    try {
      await openSettingsWindow();
    } catch (caught) {
      setSettingsError(String(caught));
    }
  };

  const openTranscription = () => {
    void panelOpenTab("chat", "transcription");
  };

  const openSummary = () => {
    void panelOpenTab("chat", "summary");
  };

  const openAsk = () => {
    void askOpen();
  };

  const toggleSuggestions = () => {
    void (suggestionsOpen ? panelHide("suggestions") : panelOpen("suggestions"));
  };

  const toggleHistory = () => {
    void (historyOpen ? panelHide("history") : panelOpen("history"));
  };

  const hideOverlay = () => {
    void setSurfaceIpc("hidden");
  };

  /** Para a sessão e abre o modal de resumo (BR-068). */
  const stopSession = async () => {
    const ended = await stop();
    if (ended) {
      await sessionEndOpen(ended.session_id);
    }
  };

  const shownError = error ?? settingsError;

  // HUD (BR-062): densidade via `data-density`. A opacidade global agora vive
  // no root do documento (`useHudOpacity`) e vale para todas as janelas.
  return (
    <div className="overlay" data-density={settings.hud_density}>
      <div className="overlay__stack" ref={stackRef}>
        <AgentSwitcher
          modes={modes}
          modeId={settings.assistant_mode_id}
          onOpenMenu={openAgentMenu}
        />

        <div className="overlay__bar-wrap">
          <DwellProvider
            enabled={settings.dwell_enabled}
            durationMs={settings.dwell_ms}
            active={state.listening}
          >
            <ActionBar
              listening={state.listening}
              paused={state.paused}
              channels={state.channels}
              startedAtMs={state.started_at_ms}
              busy={busy}
              onStart={(channels) => void start(channels)}
              onPause={() => void pause()}
              onResume={() => void resume()}
              onStop={() => void stopSession()}
              onOpenChannelMenu={openChannelMenu}
              onStartDrag={() => void startDragIpc()}
              startAccelerator={startAccelerator}
              askAccelerator={askAccelerator}
              stealth={stealth}
              onToggleStealth={toggleStealth}
              onOpenSettings={() => void openSettings()}
              onOpenTranscription={openTranscription}
              onOpenSummary={openSummary}
              onOpenAsk={openAsk}
              suggestionsOpen={suggestionsOpen}
              onToggleSuggestions={toggleSuggestions}
              historyOpen={historyOpen}
              onToggleHistory={toggleHistory}
              onHide={hideOverlay}
              recording={recording}
              recordingBusy={recordingBusy}
              recordingCapabilities={recordingCapabilities}
              onToggleRecording={() => void toggleRecording()}
              onPickRecordingTarget={() => void recordingPickerOpen()}
            />
          </DwellProvider>
        </div>
      </div>

      <AsrStatusBanner status={asrStatus} />

      {shownError && (
        <p className="overlay__error" data-interactive>
          {shownError}
        </p>
      )}
    </div>
  );
}
