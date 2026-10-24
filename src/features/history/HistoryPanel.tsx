import { useMemo, useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
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
import { MarkdownView } from "../../components/MarkdownView";
import { useTranslation } from "../../core/l10n";
import { sessionTitleText } from "../../core/l10n/labels";
import { DetailTabs, type DetailTabId } from "./DetailTabs";

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
 * sessão em abas — Resumo (overview em markdown) e Transcrição (linha temporal
 * de falas + sugestões da IA com o tempo) — com export Markdown.
 */
export function HistoryPanel({ activeSessionId, onContinue }: HistoryPanelProps) {
  const { t } = useTranslation();
  const { sessions, loading, error, refresh, rename, remove } =
    useSessionHistory(activeSessionId);
  const [selected, setSelected] = useState<string | null>(null);
  // Aba ativa no detalhe (Resumo × Transcrição); começa no resumo.
  const [tab, setTab] = useState<DetailTabId>("summary");
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
      saved
        ? t("history_exported_path", { path: saved })
        : t("history_export_unavailable_outside_app"),
    );
  };

  const onCopy = async (session: SessionSummary) => {
    setNotice(null);
    const markdown = await exportSessionMarkdown(session.id);
    const copied = markdown ? await copyText(markdown) : false;
    setNotice(
      copied ? t("history_markdown_copied") : t("history_could_not_copy"),
    );
  };

  /** Abre o detalhe de uma sessão sempre pela aba Resumo. */
  const openDetail = (id: string) => {
    setSelected(id);
    setTab("summary");
    setEditing(false);
    setNotice(null);
  };

  /** Volta para a lista limpando edição e avisos. */
  const closeDetail = () => {
    setSelected(null);
    setEditing(false);
    setNotice(null);
  };

  if (selected && current) {
    const overview =
      current.overview && current.overview.trim() !== ""
        ? current.overview
        : null;

    return (
      <div className="history" data-interactive>
        <div className="history__detail-top">
          <button
            type="button"
            className="overlay__button"
            onClick={closeDetail}
          >
            ← {t("history_back")}
          </button>
          <div className="history__detail-top-actions">
            {editing ? (
              <button
                type="button"
                className="overlay__button"
                onClick={() => void saveRename()}
              >
                {t("history_save")}
              </button>
            ) : (
              <button
                type="button"
                className="overlay__button"
                onClick={() => startRename(current)}
              >
                {t("history_rename")}
              </button>
            )}
          </div>
        </div>

        {editing ? (
          <input
            className="history__title-input"
            value={title}
            aria-label={t("history_session_title")}
            onChange={(event) => setTitle(event.target.value)}
          />
        ) : (
          <h2 className="history__detail-title">
            {sessionTitleText(current, t)}
          </h2>
        )}

        <p className="history__meta">
          {formatDateTime(current.started_at_ms)} · {t("history_duration")}{" "}
          {sessionDuration(current)} ·{" "}
          {t("history_count_item_s", { count: entries.length })}
        </p>

        <div className="history__actions">
          {current.status === "ended" && (
            <button
              type="button"
              className="overlay__button overlay__button--start"
              onClick={() => onContinue(current.id)}
            >
              {t("history_continue_session")}
            </button>
          )}
          {current.recording_path ? (
            <button
              type="button"
              className="overlay__button"
              title={t("history_opens_recording_folder")}
              onClick={() => void revealItemInDir(current.recording_path!)}
            >
              {t("history_open_recording")}
            </button>
          ) : null}
          <button
            type="button"
            className="overlay__button"
            onClick={() => void onExport(current)}
          >
            {t("history_export_md")}
          </button>
          <button
            type="button"
            className="overlay__button"
            onClick={() => void onCopy(current)}
          >
            {t("history_copy_markdown")}
          </button>
          <button
            type="button"
            className="overlay__button overlay__button--stop history__actions-delete"
            onClick={() => void remove(current.id)}
          >
            {t("history_delete")}
          </button>
        </div>

        {notice && <p className="settings__hint">{notice}</p>}

        <DetailTabs active={tab} onChange={setTab} />

        {tab === "summary" ? (
          <section
            className="history__tab-body"
            role="tabpanel"
            id="history-view-summary"
            aria-labelledby="history-tab-summary"
          >
            <h3 className="history__section-title">
              {t("history_session_summary")}
            </h3>
            {overview ? (
              <div className="history__summary-card">
                <MarkdownView markdown={overview} />
              </div>
            ) : (
              <p className="chat__empty">{t("history_summary_none")}</p>
            )}
          </section>
        ) : (
          <section
            className="history__tab-body"
            role="tabpanel"
            id="history-view-transcription"
            aria-labelledby="history-tab-transcription"
          >
            {timelineError ? (
              <p className="overlay__error">{timelineError}</p>
            ) : loadingTimeline ? (
              <p className="chat__empty">{t("history_loading")}</p>
            ) : entries.length === 0 ? (
              <p className="chat__empty">{t("history_nothing_recorded_session")}</p>
            ) : (
              <ChatView
                bubbles={bubbles}
                sessionStartedAtMs={current.started_at_ms}
              />
            )}
          </section>
        )}
      </div>
    );
  }

  return (
    <div className="history" data-interactive>
      <div className="history__head history__head--toolbar">
        <button
          type="button"
          className="overlay__button"
          disabled={loading}
          onClick={() => void refresh()}
        >
          {loading ? t("history_loading") : t("history_refresh")}
        </button>
      </div>

      {error && <p className="overlay__error">{error}</p>}

      <div className="history__list">
        {!loading && sessions.length === 0 && (
          <p className="overlay__empty">
            {t("history_no_sessions_yet_start_listening_create")}
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
              title={t("history_view_timeline")}
              onClick={() => openDetail(session.id)}
            >
              <span className="history__row-title">
                {sessionTitleText(session, t)}
              </span>
              <span className="history__row-meta">
                {formatDateTime(session.started_at_ms)} ·{" "}
                {sessionDuration(session)}
                {session.recording_kind ? (
                  <>
                    {" · "}
                    <span className="history__recording-badge">
                      {session.recording_kind === "audio"
                        ? t("history_audio")
                        : t("history_video")}
                    </span>
                  </>
                ) : null}
                {session.id === activeSessionId ? ` · ${t("history_recording")}` : ""}
              </span>
            </button>
            <button
              type="button"
              className="overlay__button"
              title={t("history_ends_current_session_any_resumes_capturing",
              )}
              onClick={() => onContinue(session.id)}
            >
              {t("history_continue")}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
