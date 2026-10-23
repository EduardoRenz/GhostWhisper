import { panelOpen } from "../../adapters/ipc/panels";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { ALL_CHANNELS } from "../../core/domain/transcript";
import { HistoryIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";
import { PanelFrame } from "../panels/PanelFrame";
import { HistoryPanel } from "./HistoryPanel";

/**
 * Janela do histórico de sessões (task 08): lista, detalhe em bolhas e export.
 *
 * Continua uma sessão encerrada e traz a janela de Chat para frente, já que a
 * conversa não vive mais no overlay.
 */
export function HistoryWindow() {
  const { t } = useTranslation();
  usePanelOpen("history");
  const { state, continueSession } = useSessionState();

  const onContinue = (id: string) => {
    void continueSession(id, ALL_CHANNELS);
    void panelOpen("chat");
  };

  return (
    <PanelFrame
      label="history"
      title={t("history_history")}
      subtitle={
        state.session_id ? t("history_session_progress") : t("history_previous_sessions")
      }
      icon={<HistoryIcon size={16} />}
    >
      <HistoryPanel activeSessionId={state.session_id} onContinue={onContinue} />
    </PanelFrame>
  );
}
