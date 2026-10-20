import {
  useEffect,
  useState,
  type ComponentProps,
  type CSSProperties,
} from "react";
import {
  openSettingsWindow,
  setStealth as setStealthIpc,
  setSurface as setSurfaceIpc,
  startDrag as startDragIpc,
} from "../../adapters/ipc/overlay";
import { panelHide, panelOpen, panelOpenTab } from "../../adapters/ipc/panels";
import { askOpen } from "../../adapters/ipc/ask";
import { useAsrStatus } from "../../core/application/useAsrStatus";
import { useInteractiveRegions } from "../../core/application/useInteractiveRegions";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { defaultBindings } from "../../core/domain/shortcuts";
import { DwellProvider, useDwell } from "../../components/dwell/DwellProvider";
import { ActionBar } from "./ActionBar";
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
  const { state, busy, error, start, pause, resume, stop, setChannels } =
    useSessionState();
  const asrStatus = useAsrStatus();
  const { settings } = useSettings();
  const suggestionsOpen = usePanelOpen("suggestions");
  const historyOpen = usePanelOpen("history");
  useInteractiveRegions();

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

  const shownError = error ?? settingsError;

  // HUD (BR-062): densidade via `data-density` e opacidade via variável CSS.
  const hudStyle = {
    "--hud-alpha": settings.overlay_opacity,
  } as CSSProperties;

  return (
    <div
      className="overlay"
      data-density={settings.hud_density}
      style={hudStyle}
    >
      <DwellProvider
        enabled={settings.dwell_enabled}
        durationMs={settings.dwell_ms}
        active={state.listening}
      >
        <DwellActionBar
          listening={state.listening}
          paused={state.paused}
          channels={state.channels}
          startedAtMs={state.started_at_ms}
          busy={busy}
          onStart={(channels) => void start(channels)}
          onPause={() => void pause()}
          onResume={() => void resume()}
          onStop={() => void stop()}
          onChannelsChange={(channels) => void setChannels(channels)}
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
        />
      </DwellProvider>

      <AsrStatusBanner status={asrStatus} />

      {shownError && (
        <p className="overlay__error" data-interactive>
          {shownError}
        </p>
      )}
    </div>
  );
}

/** Barra com o drill-hover: esconder o overlay também fecha a confirmação. */
function DwellActionBar(props: ComponentProps<typeof ActionBar>) {
  const dwell = useDwell();
  return (
    <ActionBar
      {...props}
      onHide={() => {
        dwell.cancel();
        props.onHide();
      }}
    />
  );
}
