import { useMemo, useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { exportSessionMarkdown, pickMarkdownPath, saveSessionMarkdown } from "../../adapters/ipc/export";
import {
  useSessionHistory,
  useSessionTimeline,
} from "../../core/application/useSessionHistory";
import { bubblesFromTimeline } from "../../core/domain/chat";
import {
  formatDateTime,
  sessionDuration,
  sessionTitle,
  type SessionSummary,
} from "../../core/domain/transcript";
import { ChatView } from "../chat/ChatView";

interface HistoryPanelProps {
  /** Sessão em andamento (para marcar a lista). */
  activeSessionId: string | null;
  /** Continua uma sessão encerrada (volta a capturar nela). */
  onContinue: (id: string) => void;
}

/** Nome de arquivo sugerido no export: título + data. */
function fileNameFor(session: SessionSummary): string {
  const date = new Date(session.started_at_ms);
  const pad = (value: number) => String(value).padStart(2, "0");
  const stamp = `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}-${pad(
    date.getHours(),
  )}${pad(date.getMinutes())}`;
  const slug = sessionTitle(session)
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 40);
  return `ghostwhisper-${stamp}${slug ? `-${slug}` : ""}`;
}

/**
 * Histórico de sessões: lista (com continuar/renomear/excluir) e o detalhe da
 * linha temporal (falas + sugestões da IA com o tempo), com export Markdown.
 */
export function HistoryPanel({ activeSessionId, onContinue }: HistoryPanelProps) {
  const { sessions, loading, error, refresh, rename, remove } =
    useSessionHistory(activeSessionId);
  const [selected, setSelected] = useState<string | null>(null);
  const { entries, loading: loadingTimeline, error: timelineError } =
    useSessionTimeline(selected);
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState("");
  const [notice, setNotice] = useState<string | null>(null);

  const current = sessions.find((session) => session.id === selected) ?? null;
  // Converter a linha temporal em bolhas pode custar caro numa sessão longa e não
  // pode rodar a cada render (ISS-020).
  const bubbles = useMemo(() => bubblesFromTimeline(entries), [entries]);

  const startRename = (session: SessionSummary) => {
    setEditing(true);
    setTitle(sessionTitle(session));
  };

  const saveRename = async () => {
    if (!current) {
      return;
    }
    await rename(current.id, title);
    setEditing(false);
  };

  const onExport = async (session: SessionSummary) => {
    setNotice(null);
    const path = await pickMarkdownPath(fileNameFor(session));
    if (path === null) {
      return;
    }
    const saved = await saveSessionMarkdown(session.id, path);
    setNotice(
      saved ? `Exportado: ${saved}` : "Exportação indisponível fora do app.",
    );
  };

  const onCopy = async (session: SessionSummary) => {
    setNotice(null);
    const markdown = await exportSessionMarkdown(session.id);
    const copied = markdown ? await copyText(markdown) : false;
    setNotice(copied ? "Markdown copiado." : "Não foi possível copiar.");
  };

  if (selected && current) {
    return (
      <div className="history" data-interactive>
        <div className="history__detail-head">
          <button
            type="button"
            className="overlay__button"
            onClick={() => {
              setSelected(null);
              setEditing(false);
              setNotice(null);
            }}
          >
            ← Voltar
          </button>
          {editing ? (
            <>
              <input
                className="history__title-input"
                value={title}
                aria-label="Título da sessão"
                onChange={(event) => setTitle(event.target.value)}
              />
              <button
                type="button"
                className="overlay__button"
                onClick={() => void saveRename()}
              >
                Salvar
              </button>
            </>
          ) : (
            <>
              <span className="history__detail-title">{sessionTitle(current)}</span>
              <button
                type="button"
                className="overlay__button"
                onClick={() => startRename(current)}
              >
                Renomear
              </button>
            </>
          )}
        </div>

        <p className="settings__hint">
          {formatDateTime(current.started_at_ms)} · duração{" "}
          {sessionDuration(current)} · {entries.length} item(ns)
        </p>

        <div className="history__actions">
          {current.status === "ended" && (
            <button
              type="button"
              className="overlay__button overlay__button--start"
              onClick={() => onContinue(current.id)}
            >
              Continuar sessão
            </button>
          )}
          <button
            type="button"
            className="overlay__button"
            onClick={() => void onExport(current)}
          >
            Exportar .md
          </button>
          <button
            type="button"
            className="overlay__button"
            onClick={() => void onCopy(current)}
          >
            Copiar Markdown
          </button>
          <button
            type="button"
            className="overlay__button overlay__button--stop"
            onClick={() => void remove(current.id)}
          >
            Excluir
          </button>
        </div>

        {notice && <p className="settings__hint">{notice}</p>}
        {timelineError && <p className="overlay__error">{timelineError}</p>}

        {loadingTimeline ? (
          <p className="chat__empty">Carregando…</p>
        ) : entries.length === 0 ? (
          <p className="chat__empty">Nada registrado nesta sessão.</p>
        ) : (
          <ChatView bubbles={bubbles} />
        )}
      </div>
    );
  }

  return (
    <div className="history" data-interactive>
      <div className="history__head">
        <span className="history__title">Histórico</span>
        <button
          type="button"
          className="overlay__button"
          disabled={loading}
          onClick={() => void refresh()}
        >
          {loading ? "Carregando…" : "Atualizar"}
        </button>
      </div>

      {error && <p className="overlay__error">{error}</p>}

      <div className="history__list">
        {!loading && sessions.length === 0 && (
          <p className="overlay__empty">
            Nenhuma sessão ainda. Inicie a escuta para criar a primeira.
          </p>
        )}
        {sessions.map((session) => (
          <div
            key={session.id}
            className={`history__row${
              session.id === activeSessionId ? " history__row--active" : ""
            }`}
          >
            <button
              type="button"
              className="history__open"
              title="Ver a linha temporal"
              onClick={() => setSelected(session.id)}
            >
              <span className="history__row-title">{sessionTitle(session)}</span>
              <span className="history__row-meta">
                {formatDateTime(session.started_at_ms)} ·{" "}
                {sessionDuration(session)}
                {session.id === activeSessionId ? " · gravando" : ""}
              </span>
            </button>
            <button
              type="button"
              className="overlay__button"
              title="Encerra a atual (se houver) e volta a capturar nesta sessão"
              onClick={() => onContinue(session.id)}
            >
              Continuar
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
