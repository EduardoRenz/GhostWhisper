import { VideoCameraIcon } from "../../components/icons";
import type {
  RecordingCapabilities,
  RecordingState,
} from "../../core/domain/recording";

interface RecordingButtonProps {
  state: RecordingState;
  capabilities: RecordingCapabilities | null;
  busy: boolean;
  onToggle: () => void;
  /** Abre o seletor de janelas (clique secundário). */
  onPick: () => void;
}

/**
 * Botão de câmera do notch (BR-065/US-01).
 *
 * Fica **depois de Sugestões** e só existe com sessão ativa. Vermelho quando
 * grava; pausado é vermelho vazado; desabilitado quando o ambiente não grava.
 */
export function RecordingButton({
  state,
  capabilities,
  busy,
  onToggle,
  onPick,
}: RecordingButtonProps) {
  const recording = state.status === "recording";
  const paused = state.status === "paused";
  const finalizing = state.status === "finalizing";
  const unavailable = capabilities ? !capabilities.available : false;
  const disabled = busy || unavailable || finalizing;

  const title = unavailable
    ? (capabilities?.reason ?? "Gravação indisponível")
    : finalizing
      ? "Finalizando a gravação…"
      : recording
        ? "Pausar gravação"
        : paused
          ? "Retomar gravação"
          : `Gravar esta sessão${state.target_label ? ` (${state.target_label})` : ""} · clique direito para escolher a janela`;

  const classes = [
    "notch-tabs__recording",
    recording ? "notch-tabs__recording--live" : "",
    paused ? "notch-tabs__recording--paused" : "",
    unavailable ? "notch-tabs__recording--unavailable" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={classes}
      data-interactive
      aria-label={title}
      aria-pressed={recording}
      title={title}
      disabled={disabled}
      onClick={onToggle}
      onContextMenu={(event) => {
        event.preventDefault();
        if (!disabled) {
          onPick();
        }
      }}
    >
      <VideoCameraIcon className="notch-tabs__icon" size={15} />
      {recording ? <span className="notch-tabs__rec-dot" aria-hidden="true" /> : null}
    </button>
  );
}
