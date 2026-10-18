import { useState } from "react";
import type { SessionChannel } from "../../core/domain/transcript";
import {
  PauseIcon,
  PlayIcon,
  StopIcon,
} from "../../components/TransportIcons";

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
  /** Pede o encerramento — a confirmação ("Sim/Não") fica com o ActionBar. */
  onRequestStop: () => void;
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
 * Seletor de canais + transporte da sessão (paridade com o Perssua,
 * `prds/chat/architecture/chat-ui.md` §3).
 *
 * O transporte é uma **pílula de ícones clássicos**: play (retomar), pause e stop
 * — o inativo fica esmaecido. Parado, a pílula dá lugar a "Iniciar escuta"
 * (cobre o "Start Listening"/"Restart" do Perssua).
 *
 * O seletor de canais fica **sempre habilitado**: com uma sessão aberta ele troca
 * a captura na hora (mantendo a sessão e a linha temporal), sem precisar pausar
 * ou parar (BR-019).
 */
export function SessionControls({
  listening,
  paused,
  channels,
  busy,
  onStart,
  onPause,
  onResume,
  onRequestStop,
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

      {!listening ? (
        <button
          type="button"
          className="session-start"
          disabled={busy}
          title="Começa a captura com os canais escolhidos"
          onClick={() => onStart(channelsOf(choice))}
        >
          <PlayIcon className="session-start__icon" />
          Iniciar escuta
        </button>
      ) : (
        <>
          <div className="transport" role="group" aria-label="Controles da sessão">
            <button
              type="button"
              className="transport__button"
              aria-label="Retomar"
              title="Retoma a captura na mesma sessão"
              disabled={busy || !paused}
              onClick={onResume}
            >
              <PlayIcon className="transport__icon" />
            </button>
            <button
              type="button"
              className="transport__button"
              aria-label="Pausar"
              title="Suspende a captura; a sessão e as falas continuam"
              disabled={busy || paused}
              onClick={onPause}
            >
              <PauseIcon className="transport__icon" />
            </button>
            <button
              type="button"
              className="transport__button transport__button--stop"
              aria-label="Encerrar sessão"
              title="Encerra a sessão (pede confirmação)"
              disabled={busy}
              onClick={onRequestStop}
            >
              <StopIcon className="transport__icon" />
            </button>
          </div>
          <button
            type="button"
            className="overlay__button"
            title="Encerra esta sessão e começa outra"
            disabled={busy}
            onClick={() => onStart(channelsOf(effective))}
          >
            Nova sessão
          </button>
        </>
      )}
    </div>
  );
}
