import { panelOpen } from "../../adapters/ipc/panels";

interface ChatHeaderActionsProps {
  /** Sessão atual; sem ela não há o que sugerir. */
  sessionId: string | null;
  /** Uma sugestão está sendo gerada. */
  loading: boolean;
  /** Pede uma sugestão da IA com base na conversa. */
  onRequest: () => void;
}

/**
 * Ações do cabeçalho da conversa: **Sugerir** (IA) e atalhos para as janelas de
 * Resumo e Sugestões. Fica na janela de Chat, que é onde a conversa vive agora
 * (task 07).
 */
export function ChatHeaderActions({
  sessionId,
  loading,
  onRequest,
}: ChatHeaderActionsProps) {
  return (
    <div className="chat__head-actions">
      <button
        type="button"
        className="overlay__button"
        data-interactive
        disabled={loading || !sessionId}
        title="Pede uma sugestão da IA com base na conversa"
        onClick={onRequest}
      >
        {loading ? "Gerando…" : "Sugerir"}
      </button>
      <button
        type="button"
        className="overlay__button"
        data-interactive
        title="Abre a janela de resumo da sessão"
        onClick={() => void panelOpen("summary")}
      >
        Resumo
      </button>
      <button
        type="button"
        className="overlay__button"
        data-interactive
        title="Abre a janela de sugestões por agente"
        onClick={() => void panelOpen("suggestions")}
      >
        Sugestões
      </button>
    </div>
  );
}
