import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauriRuntime } from "../../adapters/ipc/client";
import { WindowHeader } from "../../components/WindowHeader";
import {
  CheckIcon,
  DownloadIcon,
  FolderIcon,
  TrashIcon,
} from "../../components/icons";
import { useSessionEnd } from "../../core/application/useSessionEnd";
import { useTranslation } from "../../core/l10n";
import {
  formatDurationText,
  mediaLabelText,
} from "../../core/l10n/labels";

/**
 * Modal "Sessão Finalizada" (BR-068).
 *
 * Mostra duração, mídia e itens processados; oferece abrir o local, baixar o
 * Markdown, descartar a gravação e começar outra sessão. O progresso da
 * finalização aparece no cartão de mídia.
 */
export function SessionEndWindow() {
  const { t } = useTranslation();
  const {
    data,
    loading,
    busy,
    notice,
    error,
    progress,
    download,
    openFolder,
    discard,
    newSession,
    close,
  } = useSessionEnd();
  const [confirmDiscard, setConfirmDiscard] = useState(false);

  const startDrag = () => {
    if (isTauriRuntime()) {
      void getCurrentWindow().startDragging();
    }
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void close();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [close]);

  const finalizing =
    progress?.phase === "finalizing" ||
    data?.recording?.status === "finalizing";
  const percent = progress?.phase === "finalizing" ? progress.percent : 0;
  const hasRecording = Boolean(
    data?.recording?.path ?? data?.recording?.segments_dir,
  );
  const failed = progress?.phase === "failed";

  return (
    <div className="session-end" data-interactive>
      <div className="session-end__card">
        <WindowHeader
          className="session-end__header"
          title={t("session_end_session_ended")}
          icon={<CheckIcon size={18} />}
          onStartDrag={startDrag}
          onClose={() => void close()}
          closeLabel={t("session_end_close_summary")}
          dragLabel={t("session_end_move_summary")}
        />

        {loading ? (
          <p className="session-end__loading">{t("session_end_loading_session")}</p>
        ) : !data ? (
          <p className="session-end__loading">{t("session_end_loading_session")}</p>
        ) : (
          <>
            <div className="session-end__stats">
              <article className="session-end__stat">
                <span className="session-end__stat-label">
                  <span className="session-end__dot" aria-hidden="true" />
                  {t("session_end_total_duration")}
                </span>
                <strong className="session-end__stat-value">
                  {formatDurationText(data.duration_ms, t)}
                </strong>
              </article>

              <article className="session-end__stat">
                <span className="session-end__stat-label">
                  <span className="session-end__dot" aria-hidden="true" />
                  {t("session_end_media")}
                </span>
                <strong className="session-end__stat-value">
                  {mediaLabelText(data.recording, t)}
                </strong>
                {finalizing ? (
                  <div className="session-end__progress" role="status">
                    <div
                      className="session-end__progress-track"
                      aria-hidden="true"
                    >
                      <span
                        className="session-end__progress-fill"
                        style={{ width: `${percent}%` }}
                      />
                    </div>
                    <span className="session-end__progress-label">
                      {t("session_end_finalizing_recording_percent", { percent })}
                    </span>
                  </div>
                ) : null}
              </article>
            </div>

            <div className="session-end__items">
              <span className="session-end__items-label">
                <span className="session-end__dot" aria-hidden="true" />
                {t("session_end_processed_items")}
              </span>
              <span className="session-end__items-value">
                {t("session_end_transcriptions_utterances_suggestions_ai_suggestions", {
                  transcriptions: data.transcriptions,
                  suggestions: data.suggestions,
                })}
              </span>
            </div>

            <div className="session-end__actions">
              <button
                type="button"
                className="session-end__action session-end__action--ghost"
                disabled={!hasRecording || busy}
                title={
                  hasRecording
                    ? t("session_end_open_recording_folder")
                    : t("session_end_session_no_recording")
                }
                onClick={() => void openFolder()}
              >
                <FolderIcon size={16} />
                {t("session_end_open_location")}
              </button>
              <button
                type="button"
                className="session-end__action session-end__action--primary"
                disabled={busy}
                onClick={() => void download()}
              >
                <DownloadIcon size={16} />
                {t("session_end_download_session")}
              </button>
            </div>

            {notice ? (
              <p className="session-end__notice">{notice}</p>
            ) : null}
            {error || failed ? (
              <p className="session-end__error">
                {error ?? progress?.error ?? t("session_end_recording_failed")}
              </p>
            ) : null}

            <footer className="session-end__footer">
              <button
                type="button"
                className="session-end__link session-end__link--danger"
                disabled={!data.recording || busy || finalizing}
                onClick={() => {
                  if (!confirmDiscard) {
                    setConfirmDiscard(true);
                    return;
                  }
                  setConfirmDiscard(false);
                  void discard();
                }}
              >
                <TrashIcon size={14} />
                {confirmDiscard
                  ? t("session_end_confirm_discard")
                  : t("session_end_discard_recording")}
              </button>
              <button
                type="button"
                className="session-end__link"
                disabled={busy}
                onClick={() => void newSession()}
              >
                {t("session_end_new_session")}
              </button>
            </footer>
          </>
        )}
      </div>
    </div>
  );
}
