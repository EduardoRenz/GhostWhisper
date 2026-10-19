import { useEffect, useState } from "react";
import {
  openSettingsWindow,
  setStealth as setStealthIpc,
  setSurface as setSurfaceIpc,
  startDrag as startDragIpc,
} from "../../adapters/ipc/overlay";
import { panelHide, panelOpen } from "../../adapters/ipc/panels";
import { useAsrStatus } from "../../core/application/useAsrStatus";
import { useInteractiveRegions } from "../../core/application/useInteractiveRegions";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { ActionBar } from "./ActionBar";
import { AsrStatusBanner } from "./AsrStatusBanner";

/**
 * Casca do overlay: apenas a barra de ações (e o aviso de status do ASR). A
 * conversa e o histórico saíram para janelas próprias, abertas pelos toggles
 * **Chat** e **Histórico** (tasks 07/08, BR-026).
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
  const chatOpen = usePanelOpen("chat");
  const historyOpen = usePanelOpen("history");
  useInteractiveRegions();

  // Sincroniza o stealth com as settings persistidas.
  useEffect(() => {
    setStealth(settings.stealth);
  }, [settings.stealth]);

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

  const toggleChat = () => {
    void (chatOpen ? panelHide("chat") : panelOpen("chat"));
  };

  const toggleHistory = () => {
    void (historyOpen ? panelHide("history") : panelOpen("history"));
  };

  const hideOverlay = () => {
    void setSurfaceIpc("hidden");
  };

  const shownError = error ?? settingsError;

  return (
    <div className="overlay">
      <ActionBar
        stealth={stealth}
        onToggleStealth={toggleStealth}
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
        onOpenSettings={() => void openSettings()}
        onStartDrag={() => void startDragIpc()}
        chatOpen={chatOpen}
        onToggleChat={toggleChat}
        historyOpen={historyOpen}
        onToggleHistory={toggleHistory}
        onHide={hideOverlay}
      />

      <AsrStatusBanner status={asrStatus} />

      {shownError && (
        <p className="overlay__error" data-interactive>
          {shownError}
        </p>
      )}
    </div>
  );
}
