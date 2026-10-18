import { useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { useInsight } from "../../core/application/useInsight";
import {
  formatClock,
  type InsightSuggestion,
} from "../../core/domain/transcript";

interface InsightsPanelProps {
  sessionId: string | null;
  /** Sugestão empurrada pelo core (ex.: atalho global), já persistida. */
  externalSuggestion?: InsightSuggestion | null;
}

/**
 * Painel de sugestões/insights contextuais.
 *
 * Cada sugestão aparece com o **tempo desde o início da sessão** (a mesma linha
 * temporal da transcrição), e todas ficam gravadas na sessão — é o que entra no
 * export Markdown.
 */
export function InsightsPanel({
  sessionId,
  externalSuggestion = null,
}: InsightsPanelProps) {
  const { suggestions, loading, error, request } = useInsight(sessionId);
  const [copied, setCopied] = useState<string | null>(null);

  // A sugestão do atalho pode chegar antes do hook recarregar: mescla por id.
  const list: InsightSuggestion[] = externalSuggestion
    ? [
        ...suggestions.filter(
          (item) => item.message_id !== externalSuggestion.message_id,
        ),
        externalSuggestion,
      ].sort((a, b) => a.at_ms - b.at_ms)
    : suggestions;

  const onCopy = async (suggestion: InsightSuggestion) => {
    if (await copyText(suggestion.text)) {
      setCopied(suggestion.message_id);
      window.setTimeout(() => setCopied(null), 1500);
    }
  };

  return (
    <div className="insights" data-interactive>
      <div className="insights__actions">
        <button
          type="button"
          className="overlay__button"
          data-interactive
          disabled={loading || !sessionId}
          onClick={() => void request()}
        >
          {loading ? "Gerando…" : "Sugerir"}
        </button>
      </div>
      {error && <p className="insights__error">{error}</p>}

      {list.length === 0 ? (
        <p className="insights__empty">
          Nenhuma sugestão ainda — o atalho ou o botão “Sugerir” pede uma.
        </p>
      ) : (
        <div className="insights__list">
          {list.map((suggestion) => (
            <div key={suggestion.message_id} className="insights__item">
              <span className="transcript__time">
                🤖 {formatClock(suggestion.at_ms)}
              </span>
              <span className="insights__text">{suggestion.text}</span>
              <button
                type="button"
                className="overlay__button"
                data-interactive
                onClick={() => void onCopy(suggestion)}
              >
                {copied === suggestion.message_id ? "Copiado" : "Copiar"}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
