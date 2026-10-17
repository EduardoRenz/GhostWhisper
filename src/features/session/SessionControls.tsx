import { useState } from "react";
import type { SessionChannel } from "../../core/domain/transcript";

type ChannelChoice = "both" | SessionChannel;

interface SessionControlsProps {
  listening: boolean;
  busy: boolean;
  onStart: (channels: SessionChannel[]) => void;
  onStop: () => void;
}

/** Seleção de canal(is) + iniciar/parar sessão. */
export function SessionControls({
  listening,
  busy,
  onStart,
  onStop,
}: SessionControlsProps) {
  const [choice, setChoice] = useState<ChannelChoice>("both");

  const start = () => {
    if (choice === "both") {
      onStart(["microphone", "system"]);
    } else {
      onStart([choice]);
    }
  };

  return (
    <div className="session-controls" data-interactive>
      <select
        className="overlay__select"
        value={choice}
        disabled={listening}
        onChange={(event) => setChoice(event.target.value as ChannelChoice)}
      >
        <option value="both">Ambos (mic + sistema)</option>
        <option value="microphone">Microfone</option>
        <option value="system">Sistema</option>
      </select>
      {listening ? (
        <button
          type="button"
          className="overlay__button overlay__button--stop"
          disabled={busy}
          onClick={onStop}
        >
          Parar
        </button>
      ) : (
        <button
          type="button"
          className="overlay__button overlay__button--start"
          disabled={busy}
          onClick={start}
        >
          Iniciar
        </button>
      )}
    </div>
  );
}
