import { useCallback, useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

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
    let unlisten: (() => void) | undefined;
    void subscribe<RecordingProgress>("recording_progress", (payload) => {
      setProgress(payload);
      if (payload.phase !== "finalizing") {
        void reload();
      }
    }).then((stop) => {
      if (mounted.current) {
        unlisten = stop;
      } else {
        stop();
      }
    });
    return () => {
      mounted.current = false;
      unlisten?.();
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
        const path = data?.recording?.path;
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
