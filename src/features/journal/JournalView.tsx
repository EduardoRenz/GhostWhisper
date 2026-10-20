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
import { useJournal } from "../../core/application/useJournal";
import { useSessionState } from "../../core/application/useSessionState";
import { SparkleIcon } from "../../components/icons";
import type { JournalEntry } from "../../core/domain/journal";
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
 * A interação **mais recente fica no topo**. Perguntar acontece pela lupa do
 * notch (`Ctrl K`), não por um campo dentro da aba.
 */
export function JournalView() {
  const { state } = useSessionState();
  const { entries, count, loading, error, ask } = useJournal(state.session_id);
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
      saved ? `Exportado: ${saved}` : "Exportação indisponível fora do app.",
    );
  }, [state.session_id]);

  const hasEntries = entries.length > 0;

  return (
    <div className="journal" data-interactive>
      <div className="journal__list" aria-live="polite">
        {loading && !hasEntries && (
          <p className="chat__empty">Carregando…</p>
        )}

        {!loading && !hasEntries && (
          <div className="journal__empty">
            <span className="journal__empty-icon" aria-hidden="true">
              <SparkleIcon size={22} />
            </span>
            <p className="journal__empty-title">
              Nenhuma interação nesta sessão ainda
            </p>
            <p className="journal__empty-hint">
              Pergunte pela lupa do notch (<strong>Ctrl K</strong>) ou capture a
              tela (<strong>Alt C</strong>).
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
            ? "1 interação registrada nesta sessão"
            : `${count} interações registradas nesta sessão`}
        </span>
        <span className="journal__sync">Sync local · SQLite</span>
        <button
          type="button"
          className="journal__export"
          title="Exportar a sessão em Markdown"
          disabled={!state.session_id}
          onClick={() => void exportMarkdown()}
        >
          ⬇ Exportar MD
        </button>
      </footer>
    </div>
  );
}
