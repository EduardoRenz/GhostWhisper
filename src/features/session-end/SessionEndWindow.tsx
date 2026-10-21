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
import {
  formatDuration,
  mediaLabel,
} from "../../core/domain/recording";

/**
 * Modal "Sessão Finalizada" (BR-068).
 *
 * Mostra duração, mídia e itens processados; oferece abrir o local, baixar o
 * Markdown, descartar a gravação e começar outra sessão. O progresso da
 * finalização aparece no cartão de mídia.
 */
export function SessionEndWindow() {
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
  const hasRecording = Boolean(data?.recording?.path);
  const failed = progress?.phase === "failed";

  return (
    <div className="session-end" data-interactive>
      <div className="session-end__card">
        <WindowHeader
          className="session-end__header"
          title="Sessão Finalizada"
          icon={<CheckIcon size={18} />}
          onStartDrag={startDrag}
          onClose={() => void close()}
          closeLabel="Fechar resumo"
          dragLabel="Mover resumo"
        />

        {loading ? (
          <p className="session-end__loading">Carregando resumo…</p>
        ) : !data ? (
          <p className="session-end__loading">Nenhuma sessão para mostrar.</p>
        ) : (
          <>
            <div className="session-end__stats">
              <article className="session-end__stat">
                <span className="session-end__stat-label">
                  <span className="session-end__dot" aria-hidden="true" />
                  Duração total
                </span>
                <strong className="session-end__stat-value">
                  {formatDuration(data.duration_ms)}
                </strong>
              </article>

              <article className="session-end__stat">
                <span className="session-end__stat-label">
                  <span className="session-end__dot" aria-hidden="true" />
                  Mídia
                </span>
                <strong className="session-end__stat-value">
                  {mediaLabel(data.recording)}
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
                      Finalizando gravação… {percent}%
                    </span>
                  </div>
                ) : null}
              </article>
            </div>

            <div className="session-end__items">
              <span className="session-end__items-label">
                <span className="session-end__dot" aria-hidden="true" />
                Itens processados
              </span>
              <span className="session-end__items-value">
                {data.transcriptions} falas · {data.suggestions} sugestões IA
              </span>
            </div>

            <div className="session-end__actions">
              <button
                type="button"
                className="session-end__action session-end__action--ghost"
                disabled={!hasRecording || busy}
                title={
                  hasRecording
                    ? "Abrir a pasta da gravação"
                    : "Esta sessão não tem gravação"
                }
                onClick={() => void openFolder()}
              >
                <FolderIcon size={16} />
                Abrir local
              </button>
              <button
                type="button"
                className="session-end__action session-end__action--primary"
                disabled={busy}
                onClick={() => void download()}
              >
                <DownloadIcon size={16} />
                Baixar Sessão
              </button>
            </div>

            {notice ? (
              <p className="session-end__notice">{notice}</p>
            ) : null}
            {error || failed ? (
              <p className="session-end__error">
                {error ?? progress?.error ?? "Falha na gravação."}
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
                {confirmDiscard ? "Confirmar descarte?" : "Descartar gravação"}
              </button>
              <button
                type="button"
                className="session-end__link"
                disabled={busy}
                onClick={() => void newSession()}
              >
                Nova Sessão
              </button>
            </footer>
          </>
        )}
      </div>
    </div>
  );
}
