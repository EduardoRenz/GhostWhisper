import type { ManualSuggestionContext } from "../../core/domain/assistant";
import {
  formatMessageClock,
  speakerLabel,
} from "../../core/domain/transcript";
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
  return (
    <section className="manual-context" data-interactive>
      <div className="manual-context__head">
        <span className="manual-context__tag">
          <SparkleIcon size={11} />
          Contexto manual
        </span>
        <button
          type="button"
          className="manual-context__clear"
          onClick={onClear}
          title="Remover contexto"
          aria-label="Remover contexto manual"
        >
          ✕
        </button>
      </div>

      <p className="manual-context__meta">
        <span
          className={`manual-context__speaker manual-context__speaker--${context.speaker.toLowerCase()}`}
        >
          {speakerLabel(context.speaker)}
        </span>
        <span className="manual-context__time">
          {formatMessageClock(context.at_ms, startedAtMs)}
        </span>
      </p>

      <blockquote className="manual-context__quote">{context.text}</blockquote>

      <p className="manual-context__hint">
        A sugestão manual tem prioridade; as automáticas vão para o histórico
        enquanto este contexto estiver ativo.
      </p>

      <button
        type="button"
        className="manual-context__generate"
        disabled={generating}
        onClick={onGenerate}
        title="Gerar uma sugestão a partir desta mensagem"
      >
        <SparkleIcon size={12} />
        {generating ? "Aguardando a sugestão em andamento…" : "Gerar sugestão manual"}
      </button>

      {generated && (
        <p className="manual-context__done" role="status">
          Sugestão manual gerada.
        </p>
      )}
    </section>
  );
}
