import { useCallback, useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import { useTranslation } from "../l10n";
import {
  sessionContinue,
  sessionCurrent,
  sessionPause,
  sessionResume,
  sessionSetChannels,
  sessionStart,
  sessionStop,
} from "../../adapters/ipc/session";
import type { SessionChannel, SessionState } from "../domain/transcript";
import type { SessionEnd } from "../domain/recording";

/** Aviso de canal que não pôde ser capturado (ISS-087). */
export interface SessionWarning {
  channel: SessionChannel;
}

const IDLE: SessionState = {
  listening: false,
  paused: false,
  session_id: null,
  channels: [],
  started_at_ms: null,
};

/** Assina `session_state` e expõe os controles do ciclo de sessão (BR-019). */
export function useSessionState(): {
  state: SessionState;
  busy: boolean;
  error: string | null;
  start: (channels: SessionChannel[]) => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  stop: () => Promise<SessionEnd | null>;
  continueSession: (id: string, channels: SessionChannel[]) => Promise<void>;
  setChannels: (channels: SessionChannel[]) => Promise<void>;
} {
  const [state, setState] = useState<SessionState>(IDLE);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { t } = useTranslation();

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

  // Avisos de canal: quando o microfone ou o sistema não abre, o serviço
  // continua com os que funcionam e publica `session_warning`. Sem isto a
  // troca para "Ambos" parecia não ter efeito (ISS-087).
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<SessionWarning>("session_warning", (payload) => {
      const channel = payload?.channel;
      if (channel !== "microphone" && channel !== "system") {
        return;
      }
      setError(
        t(
          channel === "system"
            ? "session_warning_system_unavailable"
            : "session_warning_microphone_unavailable",
        ),
      );
    }).then((stop) => {
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
  }, [t]);

  // Semeia o estado atual: as janelas de painel podem abrir no meio da sessão e
  // não viram o último evento `session_state`. Se o evento já chegou, não mexe.
  useEffect(() => {
    let cancelled = false;
    void sessionCurrent().then((id) => {
      if (cancelled || !id) {
        return;
      }
      setState((previous) =>
        previous.session_id
          ? previous
          : { ...previous, listening: true, session_id: id },
      );
    });
    return () => {
      cancelled = true;
    };
  }, []);

  /** Executa uma ação do ciclo de sessão cuidando de `busy`/`error`. */
  const run = useCallback(async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, []);

  const start = useCallback(
    (channels: SessionChannel[]) => run(() => sessionStart(channels)),
    [run],
  );
  const pause = useCallback(() => run(() => sessionPause()), [run]);
  const resume = useCallback(() => run(() => sessionResume()), [run]);
  const stop = useCallback(async (): Promise<SessionEnd | null> => {
    setBusy(true);
    setError(null);
    try {
      const ended = await sessionStop();
      return ended ?? null;
    } catch (caught) {
      setError(String(caught));
      return null;
    } finally {
      setBusy(false);
    }
  }, []);
  const continueSession = useCallback(
    (id: string, channels: SessionChannel[]) =>
      run(() => sessionContinue(id, channels)),
    [run],
  );
  const setChannels = useCallback(
    (channels: SessionChannel[]) => run(() => sessionSetChannels(channels)),
    [run],
  );

  return {
    state,
    busy,
    error,
    start,
    pause,
    resume,
    stop,
    continueSession,
    setChannels,
  };
}
