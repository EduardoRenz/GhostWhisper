import { useEffect, useMemo } from "react";
import { useInsight } from "../../core/application/useInsight";
import { useLiveTranscription } from "../../core/application/useLiveTranscription";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useTranslation } from "../../core/application/useTranslation";
import { buildChatTimeline } from "../../core/domain/chat";
import { LevelMeters } from "../overlay/LevelMeters";
import { PanelFrame } from "../panels/PanelFrame";
import { ChatHeaderActions } from "./ChatHeaderActions";
import { ChatView } from "./ChatView";

/**
 * Janela da conversa ao vivo (task 07): as bolhas da linha temporal, a fala em
 * andamento, as sugestões e os medidores de nível — o que antes vivia no painel
 * expandido do overlay.
 *
 * Mantém a mesma assinatura de eventos de sessão/transcrição do overlay e segue
 * a linha temporal pura de `core/domain/chat`.
 */
export function ChatWindow() {
  // Assina os eventos do painel para reagir a abrir/fechar (auto-sugestões etc.).
  usePanelOpen("chat");
  const { state } = useSessionState();
  const { segments } = useLiveTranscription(state.session_id);
  const { settings } = useSettings();
  const translationEnabled =
    settings.translation_enabled && settings.target_language !== "auto";
  const {
    translations,
    pending: translationPending,
    translate: translateSegment,
    reset: resetTranslations,
  } = useTranslation();

  // Trocar de sessão invalida o cache de traduções.
  useEffect(() => {
    resetTranslations();
  }, [state.session_id, resetTranslations]);

  const {
    suggestions,
    loading: insightLoading,
    error: insightError,
    request: requestInsight,
  } = useInsight(state.session_id);

  const { bubbles, pending } = useMemo(
    () => buildChatTimeline(segments, suggestions),
    [segments, suggestions],
  );

  // Memoizado: o `ChatView` é memoizado e um elemento novo a cada render anularia
  // a memoização (o evento de nível e a prévia re-renderizam com frequência).
  const header = useMemo(
    () => (
      <ChatHeaderActions
        sessionId={state.session_id}
        loading={insightLoading}
        onRequest={() => void requestInsight()}
      />
    ),
    [state.session_id, insightLoading, requestInsight],
  );

  const status = !state.listening
    ? "Sem sessão"
    : state.paused
      ? "Pausado"
      : "Gravando";
  const subtitle = state.session_id
    ? `${state.session_id.slice(0, 8)} · ${status}`
    : status;

  return (
    <PanelFrame label="chat" title="Conversa" subtitle={subtitle}>
      <LevelMeters
        channels={state.channels}
        listening={state.listening && !state.paused}
      />
      {insightError && <p className="panel__error">{insightError}</p>}
      <ChatView
        bubbles={bubbles}
        pending={pending}
        header={header}
        emptyLabel={
          state.session_id
            ? "Nenhuma fala ainda…"
            : "Inicie uma escuta para ver a conversa aqui."
        }
        translationEnabled={translationEnabled}
        translations={translations}
        translationPending={translationPending}
        translationTarget={settings.target_language}
        onTranslate={translateSegment}
      />
    </PanelFrame>
  );
}
