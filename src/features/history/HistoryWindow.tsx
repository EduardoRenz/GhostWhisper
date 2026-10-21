import { panelOpen } from "../../adapters/ipc/panels";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { ALL_CHANNELS } from "../../core/domain/transcript";
import { HistoryIcon } from "../../components/icons";
import { PanelFrame } from "../panels/PanelFrame";
import { HistoryPanel } from "./HistoryPanel";

/**
 * Janela do histórico de sessões (task 08): lista, detalhe em bolhas e export.
 *
 * Continua uma sessão encerrada e traz a janela de Chat para frente, já que a
 * conversa não vive mais no overlay.
 */
export function HistoryWindow() {
  usePanelOpen("history");
  const { state, continueSession } = useSessionState();

  const onContinue = (id: string) => {
    void continueSession(id, ALL_CHANNELS);
    void panelOpen("chat");
  };

  return (
    <PanelFrame
      label="history"
      title="Histórico"
      subtitle={state.session_id ? "Sessão em andamento" : "Sessões anteriores"}
      icon={<HistoryIcon size={16} />}
    >
      <HistoryPanel activeSessionId={state.session_id} onContinue={onContinue} />
    </PanelFrame>
  );
}
