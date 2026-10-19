import { useState } from "react";
import { formatClock } from "../../core/domain/transcript";
import { SparkIcon, TranslateIcon } from "../../components/TransportIcons";
import { MarkdownView } from "../../components/MarkdownView";

/** Acima disto a bolha de sugestão começa colapsada (≈3 linhas). */
const CLAMP_THRESHOLD_CHARS = 160;

/** Compara ignorando caixa/espaços/pontuação final (guarda pt→pt). */
const normalize = (value: string): string =>
  value.trim().toLowerCase().replace(/[.!?…,;:]+$/u, "");

/** Lado/tonalidade da bolha. */
export type BubbleSide = "me" | "they" | "ai";

interface ChatBubbleProps {
  text: string;
  /** Rótulo do autor ("Você", "Interlocutor", "Sugestão da IA"). */
  label: string;
  side: BubbleSide;
  /** Tempo desde o início da sessão (ms); ausente em continuações. */
  atMs?: number;
  /** Fala ainda em revisão (bolha "em andamento", com indicador `…`). */
  pending?: boolean;
  /** Continuação de uma fala longa (parte seguinte, na mesma conversa). */
  continues?: boolean;
  /** Mostra o indicador `…` de digitação (padrão: `pending`). */
  typing?: boolean;
  /** Renderiza o texto como Markdown (sugestões da IA — BR-025). */
  markdown?: boolean;
  /** Copiar o texto (só a sugestão da IA usa). */
  onCopy?: () => void;
  copied?: boolean;
  /** Tradução já obtida para esta fala (mostrada abaixo, sob demanda). */
  translation?: string;
  /** Tradução em andamento (desabilita o botão e mostra `…`). */
  translating?: boolean;
  /** Pede a tradução ao clicar no ícone (bolha de fala). */
  onTranslate?: () => void;
  /** Idioma de destino (rótulo do botão e `lang` da tradução). */
  translationTarget?: string;
}

/**
 * Uma bolha da conversa, com os valores visuais do Perssua
 * (`prds/chat/architecture/chat-ui.md` §3): alinhamento e tonalidade por autor,
 * rótulo em maiúsculas, texto que quebra palavra longa e o tempo da sessão.
 *
 * Continuação de fala longa (`continues`) é compacta: sem rótulo e sem tempo
 * repetidos — a bolha de cima já diz quem falou e quando.
 */
export function ChatBubble({
  text,
  label,
  side,
  atMs,
  pending = false,
  continues = false,
  typing,
  markdown = false,
  onCopy,
  copied = false,
  translation,
  translating = false,
  onTranslate,
  translationTarget,
}: ChatBubbleProps) {
  const collapsible = markdown && text.trim().length > CLAMP_THRESHOLD_CHARS;
  const [expanded, setExpanded] = useState(false);
  const [showTranslation, setShowTranslation] = useState(false);
  const showTyping = typing ?? pending;
  const showLabelRow = !continues || onTranslate !== undefined || showTyping;
  const showFoot = atMs !== undefined || onCopy !== undefined;

  // Mesma língua não vira linha de tradução (ex.: pt → pt com detecção errada).
  const usableTranslation =
    translation && normalize(translation) !== normalize(text)
      ? translation.trim()
      : "";

  const toggleTranslation = () => {
    if (usableTranslation) {
      setShowTranslation((previous) => !previous);
      return;
    }
    // Primeiro clique: pede a tradução e já deixa o espaço pronto para ela.
    setShowTranslation(true);
    onTranslate?.();
  };

  return (
    <div
      className={`chat__row chat__row--${side}${pending ? " chat__row--pending" : ""}${
        continues ? " chat__row--continues" : ""
      }`}
      data-kind={side}
      data-pending={pending ? "true" : undefined}
      data-continues={continues ? "true" : undefined}
      title={pending ? "transcrevendo — o texto ainda pode mudar" : undefined}
    >
      <div className="chat__bubble">
        {showLabelRow && (
          <div className="chat__label">
            {!continues && (
              <>
                {side === "ai" && <SparkIcon className="chat__label-icon" />}
                <span>{label}</span>
              </>
            )}
            {showTyping && (
              <span className="chat__typing" aria-hidden="true">
                …
              </span>
            )}
            {onTranslate && (
              <button
                type="button"
                className={`chat__translate${showTranslation ? " chat__translate--on" : ""}`}
                data-interactive
                disabled={translating}
                aria-label={
                  showTranslation
                    ? "Ocultar tradução"
                    : `Traduzir${translationTarget ? ` para ${translationTarget}` : ""}`
                }
                title={
                  showTranslation
                    ? "Ocultar tradução"
                    : `Traduzir${translationTarget ? ` para ${translationTarget}` : ""}`
                }
                onClick={() => void toggleTranslation()}
              >
                {translating ? (
                  <span className="chat__translate-busy" aria-hidden="true">
                    …
                  </span>
                ) : (
                  <TranslateIcon />
                )}
              </button>
            )}
          </div>
        )}
        {markdown ? (
          <div
            className={`chat__clamp${expanded ? " chat__clamp--open" : ""}`}
          >
            <MarkdownView markdown={text} compact />
          </div>
        ) : (
          <div className="chat__text">{text}</div>
        )}
        {showTranslation && usableTranslation && (
          <p className="chat__translation" lang={translationTarget}>
            {usableTranslation}
          </p>
        )}
        {collapsible && (
          <button
            type="button"
            className="chat__expand"
            data-interactive
            aria-expanded={expanded}
            onClick={() => setExpanded((previous) => !previous)}
          >
            {expanded ? "mostrar menos" : "mostrar mais"}
          </button>
        )}
        {showFoot && (
          <div className="chat__foot">
            {atMs !== undefined && (
              <span className="chat__time" title="tempo desde o início da sessão">
                {formatClock(atMs)}
              </span>
            )}
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
        )}
      </div>
    </div>
  );
}
