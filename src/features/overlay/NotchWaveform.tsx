import { memo, useEffect, useRef } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  ALL_CHANNELS,
  type AudioLevelEvent,
} from "../../core/domain/transcript";
import {
  WAVEFORM_FACTORS,
  waveformLevel,
} from "../../core/domain/waveform";

interface NotchWaveformProps {
  /** Sessão captando (o notch ativo). */
  active: boolean;
  /** Captura pausada: barras estáticas e esmaecidas. */
  paused: boolean;
}

/**
 * Waveform do notch (4 barras cyan) ao lado de "Transcrição".
 *
 * Dirigido pelo `audio_level`: cada evento atualiza **CSS custom properties** nos
 * `refs` das barras — **sem `setState`**, então a `ActionBar` não re-renderiza a
 * cada amostra (protege a ISS-020). Pausado ou sem evento, as barras ficam baixas.
 */
export const NotchWaveform = memo(function NotchWaveform({
  active,
  paused,
}: NotchWaveformProps) {
  const barsRef = useRef<Array<HTMLSpanElement | null>>([]);

  useEffect(() => {
    const bars = barsRef.current;
    const reset = () => {
      bars.forEach((bar) => bar?.style.setProperty("--wave", "0"));
    };

    if (!active || paused) {
      reset();
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<AudioLevelEvent>("audio_level", (payload) => {
      if (!ALL_CHANNELS.includes(payload.channel)) {
        return;
      }
      const level = waveformLevel(payload.rms);
      bars.forEach((bar, index) => {
        const factor = WAVEFORM_FACTORS[index] ?? 1;
        bar?.style.setProperty("--wave", String(level * factor));
      });
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
  }, [active, paused]);

  if (!active) {
    return null;
  }

  return (
    <span
      className={`notch-wave${paused ? " notch-wave--paused" : ""}`}
      aria-hidden="true"
      data-testid="notch-waveform"
    >
      {WAVEFORM_FACTORS.map((factor, index) => (
        <span
          key={`${factor}-${index}`}
          ref={(element) => {
            barsRef.current[index] = element;
          }}
          className="notch-wave__bar"
        />
      ))}
    </span>
  );
});
