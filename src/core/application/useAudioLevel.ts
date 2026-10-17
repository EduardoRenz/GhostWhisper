import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  ALL_CHANNELS,
  type AudioLevel,
  type AudioLevelEvent,
  type SessionChannel,
} from "../domain/transcript";

const SILENT: AudioLevel = { rms: 0, peak: 0 };

/** Assina `audio_level` e devolve o nível por canal. */
export function useAudioLevel(): Record<SessionChannel, AudioLevel> {
  const [levels, setLevels] = useState<Record<SessionChannel, AudioLevel>>({
    microphone: SILENT,
    system: SILENT,
  });

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<AudioLevelEvent>("audio_level", (payload) => {
      if (!ALL_CHANNELS.includes(payload.channel)) {
        return;
      }
      setLevels((prev) => ({
        ...prev,
        [payload.channel]: { rms: payload.rms, peak: payload.peak },
      }));
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
  }, []);

  return levels;
}
