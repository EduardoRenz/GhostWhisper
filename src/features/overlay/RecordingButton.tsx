import { VideoCameraIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";
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
  const { t } = useTranslation();
  const recording = state.status === "recording";
  const paused = state.status === "paused";
  const finalizing = state.status === "finalizing";
  const unavailable = capabilities ? !capabilities.available : false;
  const disabled = busy || unavailable || finalizing;

  const title = unavailable
    ? (capabilities?.reason ?? t("recording_button_recording_unavailable"))
    : finalizing
      ? t("recording_button_finalizing_recording")
      : recording
        ? t("recording_button_pause_recording")
        : paused
          ? t("recording_button_resume_recording")
          : state.target_label
            ? t("recording_button_record_session_target_right_click_choose",
                { target: state.target_label },
              )
            : `${t("recording_button_record_session")} · ${t("recording_button_right_click_choose_window",
              )}`;

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
