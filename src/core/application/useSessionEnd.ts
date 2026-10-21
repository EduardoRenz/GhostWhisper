import { useCallback, useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import { onWindowFocus } from "../../adapters/ipc/client";
import { subscribe } from "../../adapters/ipc/events";
import {
  pickMarkdownPath,
  saveSessionMarkdown,
} from "../../adapters/ipc/export";
import { recordingDiscard } from "../../adapters/ipc/recording";
import { sessionEndClose, sessionEndContext } from "../../adapters/ipc/sessionEnd";
import { sessionStart } from "../../adapters/ipc/session";
import { ALL_CHANNELS } from "../domain/transcript";
import type { RecordingProgress, SessionEnd } from "../domain/recording";

/** Evento emitido quando o modal é aberto com uma sessão nova. */
const SESSION_END_OPEN_EVENT = "session_end_open";

/** Nome sugerido do Markdown (`título — AAAA-MM-DD.md`). */
function fileNameFor(data: SessionEnd): string {
  const date = new Date(data.started_at_ms).toISOString().slice(0, 10);
  const title = (data.subject ?? "Sessão")
    .replace(/[\\/:*?"<>|]/g, "-")
    .trim()
    .slice(0, 80);
  return `${title || "Sessão"} — ${date}.md`;
}

/**
 * Contexto e ações do modal "Sessão Finalizada" (BR-068).
 *
 * O resumo é relido do backend (a finalização pode estar em andamento); o
 * progresso chega por `recording_progress`.
 */
export function useSessionEnd(): {
  data: SessionEnd | null;
  loading: boolean;
  busy: boolean;
  notice: string | null;
  error: string | null;
  progress: RecordingProgress | null;
  download: () => Promise<void>;
  openFolder: () => Promise<void>;
  discard: () => Promise<void>;
  newSession: () => Promise<void>;
  close: () => Promise<void>;
} {
  const [data, setData] = useState<SessionEnd | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<RecordingProgress | null>(null);
  const mounted = useRef(true);

  const reload = useCallback(async () => {
    const context = await sessionEndContext();
    if (!mounted.current) {
      return;
    }
    setData(context ?? null);
    setLoading(false);
  }, []);

  useEffect(() => {
    mounted.current = true;
    void reload();
    const unlisteners: (() => void)[] = [];
    const keep = (stop: () => void) => {
      if (mounted.current) {
        unlisteners.push(stop);
      } else {
        stop();
      }
    };

    void subscribe<RecordingProgress>("recording_progress", (payload) => {
      setProgress(payload);
      if (payload.phase !== "finalizing") {
        void reload();
      }
    }).then(keep);

    // A janela é criada no boot (oculta) e reaproveitada; ao abrir, o core
    // avisa e ela relê o contexto.
    void subscribe(SESSION_END_OPEN_EVENT, () => {
      void reload();
    }).then(keep);

    // Fallback: se o evento se perder, o foco da janela relê.
    void onWindowFocus((focused) => {
      if (focused) {
        void reload();
      }
    }).then(keep);

    return () => {
      mounted.current = false;
      for (const stop of unlisteners) {
        stop();
      }
    };
  }, [reload]);

  const run = useCallback(async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, []);

  const download = useCallback(
    () =>
      run(async () => {
        if (!data) {
          return;
        }
        const path = await pickMarkdownPath(fileNameFor(data));
        if (path === null) {
          return;
        }
        const saved = await saveSessionMarkdown(data.session_id, path);
        setNotice(
          saved ? `Exportado: ${saved}` : "Exportação indisponível fora do app.",
        );
      }),
    [data, run],
  );

  const openFolder = useCallback(
    () =>
      run(async () => {
        // Sem arquivo final (falha na finalização), abre a pasta dos segmentos.
        const path = data?.recording?.path ?? data?.recording?.segments_dir;
        if (path) {
          await revealItemInDir(path);
        }
      }),
    [data, run],
  );

  const discard = useCallback(
    () =>
      run(async () => {
        if (!data) {
          return;
        }
        await recordingDiscard(data.session_id);
        setNotice("Gravação descartada.");
        await reload();
      }),
    [data, reload, run],
  );

  const newSession = useCallback(
    () =>
      run(async () => {
        await sessionStart(ALL_CHANNELS);
        await sessionEndClose();
      }),
    [run],
  );

  const close = useCallback(
    () =>
      run(async () => {
        await sessionEndClose();
      }),
    [run],
  );

  return {
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
  };
}
