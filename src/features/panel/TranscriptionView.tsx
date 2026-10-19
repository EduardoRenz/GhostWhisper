import { useCallback, useEffect, useMemo, useState } from "react";
import { panelOpen } from "../../adapters/ipc/panels";
import { prepareManualSuggestion } from "../../adapters/ipc/suggestions";
import { useLiveTranscription } from "../../core/application/useLiveTranscription";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useTranslation } from "../../core/application/useTranslation";
import { buildChatTimeline } from "../../core/domain/chat";
import { ChatView, type AskAiInput } from "../chat/ChatView";
import { LanguageBar } from "./LanguageBar";

/**
 * Aba **Transcrição** do painel da sessão (BR-033): a conversa em cartões de
 * largura total, a fala em andamento e a ação de IA por mensagem (BR-040/041).
 *
 * Os medidores de nível e o atalho de texto "Sugestões" saíram da aba: o
 * waveform e o atalho vivem no notch (BR-032/BR-033).
 */
export function TranscriptionView() {
  const { state } = useSessionState();
  const { segments } = useLiveTranscription(state.session_id);
  const { settings } = useSettings();
  const translationEnabled =
    settings.translation_enabled && settings.target_language !== "auto";
  const [askingAiId, setAskingAiId] = useState<string | null>(null);
  const [manualError, setManualError] = useState<string | null>(null);
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

  /** Leva a mensagem para o Sugestões como contexto manual (não chama o LLM). */
  const onAskAi = useCallback(
    async (input: AskAiInput) => {
      const sessionId = state.session_id;
      if (!sessionId || !state.listening) {
        return;
      }
      setAskingAiId(input.id);
      setManualError(null);
      try {
        await prepareManualSuggestion({
          sessionId,
          speaker: input.speaker,
          text: input.text,
          atMs: input.atMs,
        });
        await panelOpen("suggestions");
      } catch (caught) {
        setManualError(String(caught));
      } finally {
        setAskingAiId(null);
      }
    },
    [state.listening, state.session_id],
  );

  return (
    <>
      {manualError && <p className="chat__error">{manualError}</p>}
      <ChatView
        bubbles={bubbles}
        pending={pending}
        sessionStartedAtMs={state.started_at_ms}
        onAskAi={state.listening && state.session_id ? onAskAi : undefined}
        askingAiId={askingAiId}
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
