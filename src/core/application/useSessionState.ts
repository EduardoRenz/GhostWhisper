import { useCallback, useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import { sessionStart, sessionStop } from "../../adapters/ipc/session";
import type { SessionChannel, SessionState } from "../domain/transcript";

const IDLE: SessionState = {
  listening: false,
  session_id: null,
  channels: [],
};

/** Assina `session_state` e expõe start/stop de sessão. */
export function useSessionState(): {
  state: SessionState;
  busy: boolean;
  error: string | null;
  start: (channels: SessionChannel[]) => Promise<void>;
  stop: () => Promise<void>;
} {
  const [state, setState] = useState<SessionState>(IDLE);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<SessionState>("session_state", setState).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unlisten = stop;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const start = useCallback(async (channels: SessionChannel[]) => {
    setBusy(true);
    setError(null);
    try {
      await sessionStart(channels);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, []);

  const stop = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await sessionStop();
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, []);

  return { state, busy, error, start, stop };
}
