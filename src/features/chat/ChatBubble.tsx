import { useState } from "react";
import { SparkIcon, TranslateIcon } from "../../components/TransportIcons";
import { MarkdownView } from "../../components/MarkdownView";
import { useTranslation } from "../../core/l10n";
import { languageCodeKey } from "../../core/l10n/labels";

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
  /** Selo **Streaming** na fala em andamento. */
  streaming?: boolean;
  /** Mostra o indicador `…` de digitação (padrão: `pending`). */
  typing?: boolean;
  /** Renderiza o texto como Markdown (sugestões da IA — BR-025). */
  markdown?: boolean;
  /** Copiar o texto da bolha (sugestão e fala finalizada). */
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
 * A fala em andamento mostra "Falando agora…" e o selo **Streaming**. Cada
 * fala é uma bolha só: não há divisão por tamanho (BR-102).
 */
export function ChatBubble({
  text,
  label,
  side,
  timeLabel,
  pending = false,
  streaming = false,
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
  const { t } = useTranslation();
  const collapsible = markdown && text.trim().length > CLAMP_THRESHOLD_CHARS;
  const [expanded, setExpanded] = useState(false);
  const [showTranslation, setShowTranslation] = useState(false);
  const showTyping = typing ?? pending;
  // Copiar fica no rodapé (não precisa da linha de ações do topo). A linha do
  // topo só existe quando há tradução ou IA — evita um bloco vazio no histórico.
  const hasTopActions =
    onTranslate !== undefined || (onAskAi !== undefined && !pending);

  // Mesma língua não vira linha de tradução (ex.: pt → pt com detecção errada).
  const usableTranslation =
    translation && normalize(translation) !== normalize(text)
      ? translation.trim()
      : "";
  const targetKey = translationTarget
    ? languageCodeKey(translationTarget)
    : null;
  const translationTargetLabel = targetKey
    ? t(targetKey)
    : translationTarget;
  const translateTitle = showTranslation
    ? t("chat_hide_translation")
    : translationTargetLabel
      ? t("chat_translate_language", { language: translationTargetLabel })
      : t("chat_translate");

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
      aria-label={translateTitle}
      title={translateTitle}
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
      className={`chat__row chat__row--${side}${pending ? " chat__row--pending" : ""}`}
      data-kind={side}
      data-pending={pending ? "true" : undefined}
      title={
        pending
          ? t("chat_transcribing_text_may_still_change")
          : undefined
      }
    >
      <div className="chat__bubble">
        <div className="chat__top">
          <div className="chat__label">
            {side === "ai" && <SparkIcon className="chat__label-icon" />}
            <span className="chat__speaker">{label}</span>
            {pending && (
              <span className="chat__now" aria-hidden="true">
                {t("chat_speaking_now")}
              </span>
            )}
            {timeLabel && !pending && (
              <span className="chat__time" title={t("chat_message_time")}>
                {timeLabel}
              </span>
            )}
            {showTyping && !pending && (
              <span className="chat__typing" aria-hidden="true">
                …
              </span>
            )}
          </div>

          {pending ? (
            <div className="chat__actions">
              {translateButton}
              {streaming && (
                <span className="chat__streaming" aria-hidden="true">
                  <span className="chat__streaming-dot" />
                  {t("chat_streaming")}
                </span>
              )}
            </div>
          ) : (
            hasTopActions && (
              <div className="chat__actions">
                {translateButton}
                {onAskAi && (
                  <button
                    type="button"
                    className="chat__ai"
                    data-interactive
                    disabled={askingAi}
                    aria-label={t("chat_suggest_ai_message",
                    )}
                    title={t("chat_suggest_ai_message")}
                    onClick={onAskAi}
                  >
                    <SparkIcon size={14} />
                  </button>
                )}
              </div>
            )
          )}
        </div>

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
            {expanded ? t("chat_show_less") : t("chat_show_more")}
          </button>
        )}

        {onCopy && (
          <div className="chat__foot">
            <button
              type="button"
              className="chat__copy"
              data-interactive
              aria-label={t("chat_copy_label", { label })}
              onClick={onCopy}
            >
              {copied ? t("chat_copied") : t("chat_copy")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
