import { MarkdownView } from "../../components/MarkdownView";
import {
  JOURNAL_SOURCE_LABELS,
  type JournalEntry,
} from "../../core/domain/journal";

interface JournalEntryCardProps {
  entry: JournalEntry;
  /** Início da sessão (ms, relógio de parede) — para a hora de parede. */
  sessionStartedAtMs: number | null;
  /** Reenvia a mesma pergunta (entrada com erro). */
  onRetry?: (entry: JournalEntry) => void;
  /** Miniatura do print anexado (data URL). */
  attachment?: string | null;
}

/**
 * Uma entrada do diário: a pergunta e a resposta (BR-052).
 *
 * O instante é o **relativo à sessão** (`mm:ss`), como o resto da linha
 * temporal (BR-019); a hora de parede aparece quando o início da sessão é
 * conhecido (BR-040).
 */
export function JournalEntryCard({
  entry,
  sessionStartedAtMs,
  onRetry,
  attachment,
}: JournalEntryCardProps) {
  const pending = entry.status === "pending";
  const failed = entry.status === "error" || entry.status === "cancelled";

  return (
    <article className="journal__entry" data-status={entry.status}>
      <header className="journal__head">
        <span className="journal__time">{formatInstant(entry, sessionStartedAtMs)}</span>
        <span className="journal__source">
          {JOURNAL_SOURCE_LABELS[entry.request_source]}
        </span>
        {entry.model && <span className="journal__model">{entry.model}</span>}
      </header>

      <p className="journal__question">{entry.question}</p>

      {attachment && (
        <img src={attachment} alt="Print da pergunta" className="journal__attachment" />
      )}

      {pending && <p className="journal__pending">Pensando…</p>}

      {!pending && entry.answer && (
        <div className="journal__answer">
          <MarkdownView markdown={entry.answer} />
        </div>
      )}

      {failed && (
        <div className="journal__failed">
          <p className="journal__error">
            {entry.error ??
              (entry.status === "cancelled" ? "Pergunta cancelada." : "Falhou.")}
          </p>
          {onRetry && (
            <button
              type="button"
              className="journal__retry"
              onClick={() => onRetry(entry)}
            >
              Tentar de novo
            </button>
          )}
        </div>
      )}
    </article>
  );
}

/** Instante da entrada: `mm:ss` relativo à sessão (ou hora de parede). */
function formatInstant(
  entry: JournalEntry,
  sessionStartedAtMs: number | null,
): string {
  if (sessionStartedAtMs === null) {
    return formatRelative(entry.asked_at_ms);
  }
  const wall = new Date(sessionStartedAtMs + entry.asked_at_ms);
  return wall.toLocaleTimeString(undefined, { hour12: false });
}

/** `mm:ss` a partir de milissegundos relativos. */
function formatRelative(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
