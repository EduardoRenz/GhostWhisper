import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { copyText } from "../../core/application/clipboard";
import {
  buildChatRows,
  type ChatBubble,
} from "../../core/domain/chat";
import {
  formatMessageClock,
  type Speaker,
  type TranscriptSegment,
} from "../../core/domain/transcript";
import type { TranslationMap } from "../../core/application/useTranslation";
import { useTranslation } from "../../core/l10n";
import { speakerLabelKey } from "../../core/l10n/labels";
import { ChatBubble as Bubble } from "./ChatBubble";

/** Dados de uma fala entregues à ação de IA (BR-041). */
export interface AskAiInput {
  id: string;
  speaker: Speaker;
  /** Texto inteiro da fala (a bolha nunca é dividida por tamanho — BR-102). */
  text: string;
  atMs: number;
}

interface ChatViewProps {
  /** Linha temporal (falas finalizadas + sugestões). */
  bubbles: ChatBubble[];
  /** Falas ainda em revisão: posicionadas pelo tempo, como as finalizadas. */
  pending?: TranscriptSegment[];
  emptyLabel?: string;
  /** Início da sessão: a hora da mensagem usa a hora de parede (BR-040). */
  sessionStartedAtMs?: number | null;
  /** Ação de IA de uma fala finalizada (BR-041). */
  onAskAi?: (input: AskAiInput) => Promise<void> | void;
  /** Id da fala cuja ação de IA está em preparação. */
  askingAiId?: string | null;
  /** Tradução sob demanda nas bolhas de fala (botão no canto). */
  translationEnabled?: boolean;
  translations?: TranslationMap;
  translationPending?: Record<string, boolean>;
  translationTarget?: string;
  onTranslate?: (input: {
    id: string;
    text: string;
  }) => Promise<string | null> | void;
}

/** Distância do fim (px) em que a rolagem continua "grudada" no fim. */
const NEAR_BOTTOM_PX = 40;

/**
 * Conversa em cartões de largura total (BR-040).
 *
 * A fala em andamento é um cartão próprio na posição do seu `start_ms`; se ela
 * começou no meio de um bloco finalizado, o bloco é dividido para ela aparecer
 * no lugar certo (ISS-018/ISS-082). A rolagem segue o fim só enquanto o usuário
 * está perto dele.
 *
 * Memoizada: o painel re-renderiza por eventos de sessão/status e a conversa
 * pode ter centenas de bolhas.
 */
export const ChatView = memo(function ChatView({
  bubbles,
  pending = [],
  emptyLabel,
  sessionStartedAtMs,
  onAskAi,
  askingAiId,
  translationEnabled = false,
  translations,
  translationPending,
  translationTarget,
  onTranslate,
}: ChatViewProps) {
  const { t } = useTranslation();
  const scrollerRef = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);
  const [copied, setCopied] = useState<string | null>(null);

  const rows = useMemo(
    () => buildChatRows(bubbles, pending),
    [bubbles, pending],
  );
  const pendingText = pending.map((segment) => segment.text).join("\u0000");

  const onScroll = () => {
    const element = scrollerRef.current;
    if (!element) {
      return;
    }
    const distance = element.scrollHeight - element.scrollTop - element.clientHeight;
    stickToBottom.current = distance <= NEAR_BOTTOM_PX;
  };

  // Segue o fim enquanto o usuário não rolar para cima: falas novas e, também, o
  // texto da fala em andamento crescendo.
  useEffect(() => {
    if (!stickToBottom.current) {
      return;
    }
    const element = scrollerRef.current;
    element?.scrollTo?.({ top: element.scrollHeight, behavior: "smooth" });
  }, [rows.length, pending.length, pendingText]);

  const onCopy = useCallback(async (id: string, text: string) => {
    if (await copyText(text)) {
      setCopied(id);
      window.setTimeout(() => setCopied(null), 1_500);
    }
  }, []);

  const empty = rows.length === 0;

  return (
    <div className="chat" data-interactive>
      <div className="chat__list" ref={scrollerRef} onScroll={onScroll}>
        {empty && <p className="chat__empty">{emptyLabel ?? t("chat_no_speech_yet")}</p>}
        {rows.map((row) => {
          if (row.kind === "pending") {
            const segment = row.segment;
            // A fala em andamento é **uma** bolha por falante: o texto ainda
            // pode mudar, então nada é dividido antes do turno fechar (BR-102).
            return (
              <Bubble
                key={`pending-${segment.id}`}
                text={segment.text}
                label={t(speakerLabelKey(segment.speaker))}
                side={segment.speaker === "ME" ? "me" : "they"}
                pending
                streaming
                typing
              />
            );
          }
          const bubble = row.bubble;
          return bubble.kind === "speech" ? (
            <Bubble
              key={`speech-${bubble.id}`}
              text={bubble.text}
              label={t(speakerLabelKey(bubble.speaker))}
              side={bubble.speaker === "ME" ? "me" : "they"}
              timeLabel={formatMessageClock(bubble.at_ms, sessionStartedAtMs)}
              onAskAi={
                onAskAi
                  ? () =>
                      onAskAi({
                        id: bubble.id,
                        speaker: bubble.speaker,
                        text: bubble.text,
                        atMs: bubble.at_ms,
                      })
                  : undefined
              }
              askingAi={askingAiId === bubble.id}
              translation={
                translationEnabled ? translations?.[bubble.id] : undefined
              }
              translating={
                translationEnabled ? translationPending?.[bubble.id] : undefined
              }
              translationTarget={translationEnabled ? translationTarget : undefined}
              onTranslate={
                translationEnabled && onTranslate
                  ? () => onTranslate({ id: bubble.id, text: bubble.text })
                  : undefined
              }
              copied={copied === bubble.id}
              onCopy={() => void onCopy(bubble.id, bubble.text)}
            />
          ) : (
            <Bubble
              key={`suggestion-${bubble.id}`}
              text={bubble.text}
              label={
                bubble.mode_name
                  ? t("chat_suggestion_mode", { mode: bubble.mode_name })
                  : t("chat_ai_suggestion")
              }
              side="ai"
              timeLabel={formatMessageClock(bubble.at_ms, sessionStartedAtMs)}
              markdown
              copied={copied === bubble.id}
              onCopy={() => void onCopy(bubble.id, bubble.text)}
            />
          );
        })}
      </div>
    </div>
  );
});
