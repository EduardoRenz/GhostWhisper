import { useCallback, useEffect, useRef, useState } from "react";
import {
  pickMarkdownPath,
  saveSessionMarkdown,
} from "../../adapters/ipc/export";
import {
  journalImageBase64,
  journalImageDataUrl,
  journalImages,
} from "../../adapters/ipc/journal";
import { capturedDataUrl, capturePending } from "../../adapters/ipc/screen";
import { useJournal } from "../../core/application/useJournal";
import { useSessionState } from "../../core/application/useSessionState";
import { usePendingCapture } from "../../core/application/usePendingCapture";
import { useSettings } from "../../core/application/useSettings";
import { DEFAULT_SAVED_QUESTIONS } from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import { SparkleIcon } from "../../components/icons";
import type { JournalEntry } from "../../core/domain/journal";
import { AskComposer } from "./AskComposer";
import { JournalEntryCard, type JournalAttachment } from "./JournalEntryCard";

/** Nome de arquivo sugerido no "Exportar MD". */
function exportFileName(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return `ghostwhisper-${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(
    now.getDate(),
  )}-${pad(now.getHours())}${pad(now.getMinutes())}`;
}

/**
 * Aba **Sessão**: o diário de perguntas da sessão (BR-052) — um registro
 * rolável com origem, pergunta, print e a resposta da IA em Markdown.
 *
 * A interação **mais recente fica no topo**. Perguntar pode ser pela lupa do
 * notch (`Ctrl K`) ou pelo **composer compacto** no topo da aba (campo +
 * perguntas prontas + print sem texto).
 */
export function JournalView() {
  const { t } = useTranslation();
  const { state } = useSessionState();
  const { entries, count, loading, error, busy, ask } = useJournal(state.session_id);
  const { settings } = useSettings();
  const {
    attachment: pending,
    capturing,
    error: captureError,
    capture,
    clear: clearCapture,
  } = usePendingCapture();
  const [shots, setShots] = useState<Record<string, JournalAttachment | null>>({});
  const [notice, setNotice] = useState<string | null>(null);
  const requested = useRef<Set<string>>(new Set());

  // Ao trocar/encerrar a sessão, as miniaturas carregadas não valem mais.
  useEffect(() => {
    requested.current.clear();
    setShots({});
  }, [state.session_id]);

  // Miniaturas dos prints — carregadas só para as entradas que têm imagem.
  useEffect(() => {
    const missing = entries.filter(
      (entry) =>
        entry.request_source === "screenshot" && !requested.current.has(entry.id),
    );
    if (missing.length === 0) {
      return;
    }
    missing.forEach((entry) => requested.current.add(entry.id));
    void Promise.all(
      missing.map(async (entry) => {
        const images = await journalImages(entry.id);
        const first = images?.[0];
        const attachment: JournalAttachment | null = first
          ? {
              url: journalImageDataUrl(first),
              width: first.width,
              height: first.height,
            }
          : null;
        return [entry.id, attachment] as const;
      }),
    ).then((pairs) => {
      setShots((previous) => {
        const next = { ...previous };
        for (const [id, attachment] of pairs) {
          next[id] = attachment;
        }
        return next;
      });
    });
  }, [entries]);

  const presets = (settings.saved_questions ?? DEFAULT_SAVED_QUESTIONS)
    .map((saved) => saved.text.trim())
    .filter((text) => text !== "");

  /** Envia a pergunta levando o print pendente (se houver). */
  const send = useCallback(
    async (text: string, source: "search" | "preset") => {
      const shot = await capturePending();
      const images = shot ? [{ mime: shot.mime, data: shot.data }] : [];
      const outcome = await ask(text, images, source);
      if (outcome && images.length > 0) {
        await clearCapture();
      }
    },
    [ask, clearCapture],
  );

  const retry = useCallback(
    async (entry: JournalEntry) => {
      const source =
        entry.request_source === "preset"
          ? "preset"
          : entry.request_source === "screenshot"
            ? "screenshot"
            : "search";
      const images =
        entry.request_source === "screenshot"
          ? ((await journalImages(entry.id)) ?? []).map((image) => ({
              mime: image.mime,
              data: journalImageBase64(image),
            }))
          : [];
      await ask(entry.question, images, source);
    },
    [ask],
  );

  const exportMarkdown = useCallback(async () => {
    const id = state.session_id;
    if (!id) {
      return;
    }
    setNotice(null);
    const path = await pickMarkdownPath(exportFileName());
    if (path === null) {
      return;
    }
    const saved = await saveSessionMarkdown(id, path);
    setNotice(
      saved
        ? t("journal_exported_path", { path: saved })
        : t("journal_export_unavailable_outside_app"),
    );
  }, [state.session_id, t]);

  const hasEntries = entries.length > 0;

  return (
    <div className="journal" data-interactive>
      <AskComposer
        presets={presets}
        attachment={pending ? capturedDataUrl(pending) : null}
        busy={busy || capturing}
        error={captureError}
        onSend={(text, source) => void send(text, source)}
        onCapture={() => void capture()}
        onRemoveAttachment={() => void clearCapture()}
      />

      <div className="journal__list" aria-live="polite">
        {loading && !hasEntries && (
          <p className="chat__empty">{t("journal_loading")}</p>
        )}

        {!loading && !hasEntries && (
          <div className="journal__empty">
            <span className="journal__empty-icon" aria-hidden="true">
              <SparkleIcon size={22} />
            </span>
            <p className="journal__empty-title">
              {t("journal_no_interactions_session_yet")}
            </p>
            <p className="journal__empty-hint">
              {t("journal_ask_field_above_notch_magnifier_ctrl",
              )}
            </p>
          </div>
        )}

        {entries.map((entry, index) => (
          <JournalEntryCard
            key={entry.id}
            entry={entry}
            sessionStartedAtMs={state.started_at_ms}
            onRetry={(failed) => void retry(failed)}
            attachment={shots[entry.id] ?? null}
            newest={index === 0}
          />
        ))}
      </div>

      {error && <p className="overlay__error">{error}</p>}
      {notice && <p className="journal__notice">{notice}</p>}

      <footer className="journal__footer">
        <span className="journal__count">
          <span className="journal__count-dot" aria-hidden="true" />
          {count === 1
            ? t("journal_1_interaction_recorded_session")
            : t("journal_count_interactions_recorded_session", { count })}
        </span>
        <span className="journal__sync">{t("journal_local_sync_sqlite")}</span>
        <button
          type="button"
          className="journal__export"
          title={t("journal_export_session_markdown")}
          disabled={!state.session_id}
          onClick={() => void exportMarkdown()}
        >
          {t("journal_export_md")}
        </button>
      </footer>
    </div>
  );
}
