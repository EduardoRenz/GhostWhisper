import { useState } from "react";
import type { SessionChannel } from "../../core/domain/transcript";

type ChannelChoice = "both" | SessionChannel;

interface SessionControlsProps {
  listening: boolean;
  /** Captura suspensa, mas a sessão segue aberta (BR-019). */
  paused: boolean;
  /** Canais captados agora (ou escolhidos, quando pausada). */
  channels: SessionChannel[];
  busy: boolean;
  onStart: (channels: SessionChannel[]) => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  /** Troca os canais **sem** encerrar a sessão. */
  onChannelsChange: (channels: SessionChannel[]) => void;
}

const CHOICES: { value: ChannelChoice; label: string }[] = [
  { value: "both", label: "Ambos (mic + sistema)" },
  { value: "microphone", label: "Microfone" },
  { value: "system", label: "Sistema" },
];

/** Converte a lista de canais na opção do seletor. */
const choiceOf = (channels: SessionChannel[]): ChannelChoice => {
  const hasMic = channels.includes("microphone");
  const hasSystem = channels.includes("system");
  if (hasMic && hasSystem) return "both";
  return hasSystem ? "system" : "microphone";
};

const channelsOf = (choice: ChannelChoice): SessionChannel[] =>
  choice === "both" ? ["microphone", "system"] : [choice];

/**
 * Seleção de canal(is) + ciclo de sessão: iniciar/pausar/retomar/nova/parar.
 *
 * O seletor de canais fica **sempre habilitado**: com uma sessão aberta ele troca
 * a captura na hora (mantendo a sessão e a linha temporal), sem precisar pausar
 * ou parar.
 */
export function SessionControls({
  listening,
  paused,
  channels,
  busy,
  onStart,
  onPause,
  onResume,
  onStop,
  onChannelsChange,
}: SessionControlsProps) {
  const [choice, setChoice] = useState<ChannelChoice>("both");
  // Com sessão aberta o seletor reflete o que está sendo captado de fato.
  const effective = listening ? choiceOf(channels) : choice;

  const onChange = (next: ChannelChoice) => {
    setChoice(next);
    if (listening) {
      // Enquanto o usuário mexe, o seletor segue a escolha dele.
      onChannelsChange(channelsOf(next));
    }
  };

  return (
    <div className="session-controls" data-interactive>
      <select
        className="overlay__select"
        value={effective}
        aria-label="Canais de áudio"
        title="Canais captados; trocar aqui não encerra a sessão"
        onChange={(event) => onChange(event.target.value as ChannelChoice)}
      >
        {CHOICES.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>

      {!listening && (
        <button
          type="button"
          className="overlay__button overlay__button--start"
          disabled={busy}
          onClick={() => onStart(channelsOf(choice))}
        >
          Iniciar
        </button>
      )}

      {listening && (
        <>
          <button
            type="button"
            className="overlay__button"
            title={
              paused
                ? "Volta a capturar na mesma sessão"
                : "Suspende a captura; a sessão e as mensagens continuam"
            }
            disabled={busy}
            onClick={paused ? onResume : onPause}
          >
            {paused ? "Retomar" : "Pausar"}
          </button>
          <button
            type="button"
            className="overlay__button"
            title="Encerra esta sessão e começa outra"
            disabled={busy}
            onClick={() => onStart(channelsOf(effective))}
          >
            Nova sessão
          </button>
          <button
            type="button"
            className="overlay__button overlay__button--stop"
            title="Encerra a sessão"
            disabled={busy}
            onClick={onStop}
          >
            Parar
          </button>
        </>
      )}
    </div>
  );
}
