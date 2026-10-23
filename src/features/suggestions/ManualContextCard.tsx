import type { ManualSuggestionContext } from "../../core/domain/assistant";
import {
  formatMessageClock,
} from "../../core/domain/transcript";
import { useTranslation } from "../../core/l10n";
import { speakerLabelKey } from "../../core/l10n/labels";
import { SparkleIcon } from "../../components/icons";

interface ManualContextCardProps {
  context: ManualSuggestionContext;
  /** Início da sessão (para a hora de parede — BR-040). */
  startedAtMs?: number | null;
  /** `true` enquanto uma geração está em curso. */
  generating: boolean;
  /** `true` quando a última geração veio deste contexto (feedback). */
  generated: boolean;
  onGenerate: () => void;
  onClear: () => void;
}

/**
 * Bloco de **contexto manual** (BR-041): a mensagem selecionada na Transcrição.
 *
 * O usuário decide quando gerar; a automática continua rodando (vai para o
 * histórico enquanto este contexto estiver ativo — BR-042).
 */
export function ManualContextCard({
  context,
  startedAtMs,
  generating,
  generated,
  onGenerate,
  onClear,
}: ManualContextCardProps) {
  const { t } = useTranslation();
  return (
    <section className="manual-context" data-interactive>
      <div className="manual-context__head">
        <span className="manual-context__tag">
          <SparkleIcon size={11} />
          {t("suggestions_manual_context")}
        </span>
        <button
          type="button"
          className="manual-context__clear"
          onClick={onClear}
          title={t("suggestions_remove_context")}
          aria-label={t("suggestions_remove_manual_context")}
        >
          ✕
        </button>
      </div>

      <p className="manual-context__meta">
        <span
          className={`manual-context__speaker manual-context__speaker--${context.speaker.toLowerCase()}`}
        >
          {t(speakerLabelKey(context.speaker))}
        </span>
        <span className="manual-context__time">
          {formatMessageClock(context.at_ms, startedAtMs)}
        </span>
      </p>

      <blockquote className="manual-context__quote">{context.text}</blockquote>

      <p className="manual-context__hint">
        {t("suggestions_manual_suggestion_priority_automatic_ones_go",
        )}
      </p>

      <button
        type="button"
        className="manual-context__generate"
        disabled={generating}
        onClick={onGenerate}
        title={t("suggestions_generate_suggestion_message")}
      >
        <SparkleIcon size={12} />
        {generating
          ? t("suggestions_waiting_current_suggestion")
          : t("suggestions_generate_manual_suggestion")}
      </button>

      {generated && (
        <p className="manual-context__done" role="status">
          {t("suggestions_manual_suggestion_generated")}
        </p>
      )}
    </section>
  );
}
