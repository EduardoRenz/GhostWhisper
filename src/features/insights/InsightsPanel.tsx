import { useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { useInsight } from "../../core/application/useInsight";

interface InsightsPanelProps {
  sessionId: string | null;
  /** Insight empurrado pelo core (ex.: atalho global). */
  externalInsight?: string | null;
}

/** Painel de sugestões/insights contextuais. */
export function InsightsPanel({
  sessionId,
  externalInsight = null,
}: InsightsPanelProps) {
  const { insight, loading, error, request, clear } = useInsight(sessionId);
  const [copied, setCopied] = useState(false);
  const shown = insight ?? externalInsight;

  const onCopy = async () => {
    if (shown && (await copyText(shown))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    }
  };

  return (
    <div className="insights" data-interactive>
      <div className="insights__actions">
        <button
          type="button"
          className="overlay__button"
          data-interactive
          disabled={loading}
          onClick={() => void request()}
        >
          {loading ? "Gerando…" : "Sugerir"}
        </button>
        {shown && (
          <button
            type="button"
            className="overlay__button"
            data-interactive
            onClick={() => void onCopy()}
          >
            {copied ? "Copiado" : "Copiar"}
          </button>
        )}
        {insight && (
          <button
            type="button"
            className="overlay__button"
            data-interactive
            onClick={clear}
          >
            Limpar
          </button>
        )}
      </div>
      {error && <p className="insights__error">{error}</p>}
      {shown && <p className="insights__text">{shown}</p>}
    </div>
  );
}
