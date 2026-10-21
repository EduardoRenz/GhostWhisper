import { useCallback, useEffect, useState } from "react";

import { subscribe } from "../../adapters/ipc/events";
import {
  recordingCapabilities,
  recordingPause,
  recordingResume,
  recordingStart,
  recordingState,
} from "../../adapters/ipc/recording";
import {
  IDLE_RECORDING,
  type RecordingCapabilities,
  type RecordingState as RecordingStatus,
} from "../domain/recording";

/**
 * Estado da gravação + ações do botão de câmera (BR-065).
 *
 * O estado vem do backend pelo evento `recording_state`: fechar e reabrir o
 * overlay não perde o vermelho.
 */
export function useRecording(): {
  state: RecordingStatus;
  capabilities: RecordingCapabilities | null;
  busy: boolean;
  error: string | null;
  toggle: () => Promise<void>;
} {
  const [state, setState] = useState<RecordingStatus>(IDLE_RECORDING);
  const [capabilities, setCapabilities] =
    useState<RecordingCapabilities | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void subscribe<RecordingStatus>("recording_state", setState).then((stop) => {
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

  useEffect(() => {
    let cancelled = false;
    void recordingCapabilities().then((capabilities) => {
      if (!cancelled && capabilities) {
        setCapabilities(capabilities);
      }
    });
    void recordingState().then((current) => {
      if (!cancelled && current) {
        setState(current);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const toggle = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      if (state.status === "recording") {
        await recordingPause();
      } else if (state.status === "paused") {
        await recordingResume();
      } else {
        await recordingStart();
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, [state.status]);

  return { state, capabilities, busy, error, toggle };
}
