import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { copyText } from "../../core/application/clipboard";
import {
  MAX_BUBBLES,
  splitSpeechText,
  type ChatBubble,
} from "../../core/domain/chat";
import {
  speakerLabel,
  type TranscriptSegment,
} from "../../core/domain/transcript";
import type { TranslationMap } from "../../core/application/useTranslation";
import { ChatBubble as Bubble } from "./ChatBubble";

interface ChatViewProps {
  /** Linha temporal (falas finalizadas + sugestões). */
  bubbles: ChatBubble[];
  /** Falas ainda em revisão: exibidas **depois** da linha temporal. */
  pending?: TranscriptSegment[];
  /** Cabeçalho do chat (ex.: botão "Sugerir"). */
  header?: ReactNode;
  emptyLabel?: string;
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
 * Conversa em bolhas: a linha temporal do Perssua (`chat-ui.md` §3).
 *
 * A fala em andamento aparece **no fim**, como bolha própria com `…` — nunca no
 * meio da linha temporal (é o que corrige a leitura errada da ISS-018). A rolagem
 * segue o fim só enquanto o usuário está perto dele.
 *
 * Memoizada: o overlay re-renderiza por eventos de sessão/status e a conversa pode
 * ter centenas de bolhas.
 */
export const ChatView = memo(function ChatView({
  bubbles,
  pending = [],
  header,
  emptyLabel = "Nenhuma fala ainda…",
  translationEnabled = false,
  translations,
  translationPending,
  translationTarget,
  onTranslate,
}: ChatViewProps) {
  const scrollerRef = useRef<HTMLDivElement>(null);
  const stickToBottom = useRef(true);
  const [copied, setCopied] = useState<string | null>(null);

  const visible = useMemo(() => bubbles.slice(-MAX_BUBBLES), [bubbles]);
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
  }, [visible.length, pending.length, pendingText]);

  const onCopy = useCallback(async (id: string, text: string) => {
    if (await copyText(text)) {
      setCopied(id);
      window.setTimeout(() => setCopied(null), 1_500);
    }
  }, []);

  const empty = visible.length === 0 && pending.length === 0;

  return (
    <div className="chat" data-interactive>
      {header && <div className="chat__head">{header}</div>}
      <div className="chat__list" ref={scrollerRef} onScroll={onScroll}>
        {empty && <p className="chat__empty">{emptyLabel}</p>}
        {visible.map((bubble) =>
          bubble.kind === "speech" ? (
            <Bubble
              key={`speech-${bubble.id}`}
              text={bubble.text}
              label={speakerLabel(bubble.speaker)}
              side={bubble.speaker === "ME" ? "me" : "they"}
              atMs={bubble.continues ? undefined : bubble.at_ms}
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
              continues={bubble.continues}
            />
          ) : (
            <Bubble
              key={`suggestion-${bubble.id}`}
              text={bubble.text}
              label={bubble.mode_name ? `Sugestão · ${bubble.mode_name}` : "Sugestão da IA"}
              side="ai"
              atMs={bubble.at_ms}
              markdown
              copied={copied === bubble.id}
              onCopy={() => void onCopy(bubble.id, bubble.text)}
            />
          ),
        )}
        {pending.flatMap((segment) => {
          // A prévia cresce enquanto o Interlocutor fala: mesma regra de teto, com
          // o `…` só na última parte.
          const parts = splitSpeechText(segment.text);
          return parts.map((part, index) => (
            <Bubble
              key={`pending-${segment.id}#${index}`}
              text={part}
              label={speakerLabel(segment.speaker)}
              side={segment.speaker === "ME" ? "me" : "they"}
              atMs={index === 0 ? segment.start_ms : undefined}
              pending
              typing={index === parts.length - 1}
              continues={index > 0}
            />
          ));
        })}
      </div>
    </div>
  );
});
