import { MarkdownView } from "../../components/MarkdownView";
import {
  CameraIcon,
  SearchIcon,
  SparkleIcon,
} from "../../components/icons";
import {
  JOURNAL_ORIGIN_LABELS,
  hasJournalQuestion,
  journalOrigin,
  type JournalEntry,
} from "../../core/domain/journal";

/** Miniatura do print anexado (data URL + dimensões, quando conhecidas). */
export interface JournalAttachment {
  url: string;
  width: number | null;
  height: number | null;
}

interface JournalEntryCardProps {
  entry: JournalEntry;
  /** Início da sessão (ms, relógio de parede) — usado quando a entrada não
   *  tem o instante absoluto. */
  sessionStartedAtMs: number | null;
  /** Reenvia a mesma pergunta (entrada com erro). */
  onRetry?: (entry: JournalEntry) => void;
  /** Miniatura do print anexado. */
  attachment?: JournalAttachment | null;
  /** Entrada mais recente da sessão (selo "Mais Recente"). */
  newest?: boolean;
}

/**
 * Uma entrada do diário (BR-052): **origem**, pergunta e resposta.
 *
 * O cabeçalho diz de onde veio (texto, print ou os dois) e há quanto tempo;
 * a pergunta só aparece quando existe (print puro não tem). A resposta é
 * **Markdown da IA** e vem num bloco próprio, depois de cada pergunta.
 */
export function JournalEntryCard({
  entry,
  sessionStartedAtMs,
  onRetry,
  attachment,
  newest = false,
}: JournalEntryCardProps) {
  const pending = entry.status === "pending";
  const failed = entry.status === "error" || entry.status === "cancelled";
  const origin = journalOrigin(entry);
  const withImage = origin === "screenshot" || origin === "both";
  const askedAtMs =
    entry.created_at_ms > 0
      ? entry.created_at_ms
      : sessionStartedAtMs !== null
        ? sessionStartedAtMs + entry.asked_at_ms
        : null;
  const badge = newest ? "Mais Recente" : entry.model;

  return (
    <article className="journal__entry" data-status={entry.status} data-origin={origin}>
      <header className="journal__head">
        <span className="journal__origin-icon" aria-hidden="true">
          {withImage ? <CameraIcon size={14} /> : <SearchIcon size={14} />}
        </span>
        <span className="journal__source">{JOURNAL_ORIGIN_LABELS[origin]}</span>
        <span className="journal__time">
          · {formatAgo(askedAtMs)}
          {askedAtMs !== null && (
            <span className="journal__clock">{formatClock(askedAtMs)}</span>
          )}
        </span>
        {badge && <span className="journal__badge">{badge}</span>}
      </header>

      {hasJournalQuestion(entry) && (
        <p className="journal__question">
          <span className="journal__question-label">
            {withImage ? "Contexto:" : "Q:"}
          </span>{" "}
          {entry.question}
        </p>
      )}

      {attachment && (
        <figure className="journal__shot">
          <img
            className="journal__shot-image"
            src={attachment.url}
            alt="Print anexado à pergunta"
            loading="lazy"
          />
          <figcaption className="journal__shot-caption">
            <CameraIcon size={12} />
            <span>Captura de tela</span>
            {attachment.width !== null && attachment.height !== null && (
              <span className="journal__shot-size">
                {attachment.width}×{attachment.height}
              </span>
            )}
          </figcaption>
        </figure>
      )}

      {pending && <p className="journal__pending">Pensando…</p>}

      {!pending && entry.answer && entry.answer.trim() !== "" && (
        <section className="journal__answer">
          <header className="journal__answer-head">
            <SparkleIcon className="journal__answer-icon" size={14} />
            <span className="journal__answer-brand">GhostWhisper IA</span>
            <span className="journal__answer-sep" aria-hidden="true">
              ·
            </span>
            <span className="journal__answer-mode">Assistente</span>
          </header>
          <div className="journal__answer-body">
            <MarkdownView markdown={entry.answer} />
          </div>
        </section>
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

/** "Há 1 min" — quanto tempo faz que a pergunta foi feita. */
function formatAgo(askedAtMs: number | null): string {
  if (askedAtMs === null) {
    return "agora";
  }
  const elapsed = Math.max(0, Date.now() - askedAtMs);
  const minutes = Math.floor(elapsed / 60_000);
  if (minutes < 1) {
    return "Há instantes";
  }
  if (minutes < 60) {
    return `Há ${minutes} min`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `Há ${hours} h`;
  }
  return `Há ${Math.floor(hours / 24)} d`;
}

/** Hora de parede `(10:44:12)` do instante da pergunta. */
function formatClock(askedAtMs: number): string {
  const wall = new Date(askedAtMs);
  const pad = (value: number) => String(value).padStart(2, "0");
  return ` (${pad(wall.getHours())}:${pad(wall.getMinutes())}:${pad(wall.getSeconds())})`;
}
