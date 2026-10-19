import { useEffect, useMemo } from "react";
import { useLiveTranscription } from "../../core/application/useLiveTranscription";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useTranslation } from "../../core/application/useTranslation";
import { buildChatTimeline } from "../../core/domain/chat";
import { LevelMeters } from "../overlay/LevelMeters";
import { ChatHeaderActions } from "../chat/ChatHeaderActions";
import { ChatView } from "../chat/ChatView";
import { LanguageBar } from "./LanguageBar";

/**
 * Aba **Transcrição** do painel da sessão (BR-033): a conversa em bolhas, a fala
 * em andamento e os medidores de nível.
 *
 * É **só transcrição**: sugestões da IA não são geradas nem injetadas aqui (o
 * "Sugerir" e as bolhas de sugestão ao vivo saíram — a aba **Sugestões** será
 * aprimorada à parte). O que já estiver persistido continua no histórico/export.
 */
export function TranscriptionView() {
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

  const { bubbles, pending } = useMemo(
    () => buildChatTimeline(segments, []),
    [segments],
  );

  // Elemento estável: o `ChatView` é memoizado.
  const header = useMemo(() => <ChatHeaderActions />, []);

  return (
    <>
      <LevelMeters
        channels={state.channels}
        listening={state.listening && !state.paused}
      />
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
      <LanguageBar />
    </>
  );
}
