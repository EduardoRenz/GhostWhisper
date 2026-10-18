import { formatClock } from "../../core/domain/transcript";
import { SparkIcon } from "../../components/TransportIcons";

/** Lado/tonalidade da bolha. */
export type BubbleSide = "me" | "they" | "ai";

interface ChatBubbleProps {
  text: string;
  /** Rótulo do autor ("Você", "Interlocutor", "Sugestão da IA"). */
  label: string;
  side: BubbleSide;
  /** Tempo desde o início da sessão (ms). */
  atMs: number;
  /** Fala ainda em revisão (bolha "em andamento", com indicador `…`). */
  pending?: boolean;
  /** Copiar o texto (só a sugestão da IA usa). */
  onCopy?: () => void;
  copied?: boolean;
}

/**
 * Uma bolha da conversa, com os valores visuais do Perssua
 * (`prds/chat/architecture/chat-ui.md` §3): alinhamento e tonalidade por autor,
 * rótulo em maiúsculas, texto que quebra palavra longa e o tempo da sessão.
 */
export function ChatBubble({
  text,
  label,
  side,
  atMs,
  pending = false,
  onCopy,
  copied = false,
}: ChatBubbleProps) {
  return (
    <div
      className={`chat__row chat__row--${side}${pending ? " chat__row--pending" : ""}`}
      data-kind={side}
      data-pending={pending ? "true" : undefined}
      title={pending ? "transcrevendo — o texto ainda pode mudar" : undefined}
    >
      <div className="chat__bubble">
        <div className="chat__label">
          {side === "ai" && <SparkIcon className="chat__label-icon" />}
          <span>{label}</span>
          {pending && (
            <span className="chat__typing" aria-hidden="true">
              …
            </span>
          )}
        </div>
        <div className="chat__text">{text}</div>
        <div className="chat__foot">
          <span className="chat__time" title="tempo desde o início da sessão">
            {formatClock(atMs)}
          </span>
          {onCopy && (
            <button
              type="button"
              className="chat__copy"
              data-interactive
              aria-label={`Copiar ${label}`}
              onClick={onCopy}
            >
              {copied ? "Copiado" : "Copiar"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
