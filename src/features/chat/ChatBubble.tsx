import { useState } from "react";
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
  /** Rótulo do autor ("Microfone", "Outros", "Sugestão da IA"). */
  label: string;
  side: BubbleSide;
  /** Tempo exibido ao lado do rótulo (BR-040). */
  timeLabel?: string;
  /** Fala ainda em revisão (bolha "em andamento", com indicador `…`). */
  pending?: boolean;
  /** Última parte da fala em andamento (mostra o selo **Streaming**). */
  streaming?: boolean;
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
  /** Ação de IA da mensagem (BR-041): só na fala finalizada. */
  onAskAi?: () => void;
  /** Ação de IA em preparação. */
  askingAi?: boolean;
}

/**
 * Uma bolha da conversa, com o layout do painel ao vivo (BR-040):
 * cartão de **largura total**, rótulo colorido + hora no topo e o grupo de
 * ações no canto superior direito (tradução + IA).
 *
 * A fala em andamento mostra "Falando agora…" e o selo **Streaming**; a
 * continuação de fala longa (`continues`) não repete rótulo/hora.
 */
export function ChatBubble({
  text,
  label,
  side,
  timeLabel,
  pending = false,
  streaming = false,
  continues = false,
  typing,
  markdown = false,
  onCopy,
  copied = false,
  translation,
  translating = false,
  onTranslate,
  translationTarget,
  onAskAi,
  askingAi = false,
}: ChatBubbleProps) {
  const collapsible = markdown && text.trim().length > CLAMP_THRESHOLD_CHARS;
  const [expanded, setExpanded] = useState(false);
  const [showTranslation, setShowTranslation] = useState(false);
  const showTyping = typing ?? pending;
  // A continuação não repete o rótulo, mas ainda pode mostrar o selo Streaming
  // na última parte da fala em andamento.
  const showHeader = !continues || (pending && streaming);
  const hasActions =
    (onTranslate !== undefined && !continues) ||
    (onAskAi !== undefined && !pending && !continues) ||
    (onCopy !== undefined && !continues);

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

  const translateButton = onTranslate ? (
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
  ) : null;

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
        {showHeader && (
          <div className="chat__top">
            {!continues && (
              <div className="chat__label">
                {side === "ai" && <SparkIcon className="chat__label-icon" />}
                <span className="chat__speaker">{label}</span>
                {pending && (
                  <span className="chat__now" aria-hidden="true">
                    Falando agora…
                  </span>
                )}
                {timeLabel && !pending && (
                  <span className="chat__time" title="hora da mensagem">
                    {timeLabel}
                  </span>
                )}
                {showTyping && !pending && (
                  <span className="chat__typing" aria-hidden="true">
                    …
                  </span>
                )}
              </div>
            )}

            {pending ? (
              <div className="chat__actions">
                {!continues && translateButton}
                {streaming && (
                  <span className="chat__streaming" aria-hidden="true">
                    <span className="chat__streaming-dot" />
                    Streaming
                  </span>
                )}
              </div>
            ) : (
              hasActions && (
                <div className="chat__actions">
                  {translateButton}
                  {onAskAi && (
                    <button
                      type="button"
                      className="chat__ai"
                      data-interactive
                      disabled={askingAi}
                      aria-label="Sugerir com IA a partir desta mensagem"
                      title="Sugerir com IA a partir desta mensagem"
                      onClick={onAskAi}
                    >
                      <SparkIcon size={14} />
                    </button>
                  )}
                </div>
              )
            )}
          </div>
        )}

        {markdown ? (
          <div className={`chat__clamp${expanded ? " chat__clamp--open" : ""}`}>
            <MarkdownView markdown={text} compact />
          </div>
        ) : (
          <div className="chat__text">
            {text}
            {pending && showTyping && (
              <span className="chat__caret" aria-hidden="true" />
            )}
          </div>
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

        {onCopy && !continues && (
          <div className="chat__foot">
            <button
              type="button"
              className="chat__copy"
              data-interactive
              aria-label={`Copiar ${label}`}
              onClick={onCopy}
            >
              {copied ? "Copiado" : "Copiar"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
