import {
  MicIcon,
  SparkleIcon,
  SummaryIcon,
} from "../../components/icons";
import { DwellButton } from "../../components/dwell/DwellButton";
import { PauseIcon, PlayIcon } from "../../components/TransportIcons";
import { useTranslation } from "../../core/l10n";
import type {
  RecordingCapabilities,
  RecordingState,
} from "../../core/domain/recording";
import { NotchWaveform } from "./NotchWaveform";
import { RecordingButton } from "./RecordingButton";

interface NotchTabsProps {
  /** Sessão captando (a aba viva). */
  listening: boolean;
  /** Captura pausada (o transporte vira "play" e o waveform congela). */
  paused: boolean;
  /** Uma operação de sessão está em curso. */
  busy: boolean;
  /** A janela de sugestões está visível. */
  suggestionsOpen: boolean;
  /** Abre/foca o painel na aba Transcrição. */
  onOpenTranscription: () => void;
  /** Abre/foca o painel na aba Resumo. */
  onOpenSummary: () => void;
  /** Mostra/esconde a janela de sugestões. */
  onToggleSuggestions: () => void;
  onPause: () => void;
  onResume: () => void;
  /** Gravação da sessão (BR-065). */
  recording: RecordingState;
  recordingBusy: boolean;
  recordingCapabilities: RecordingCapabilities | null;
  onToggleRecording: () => void;
  onPickRecordingTarget: () => void;
}

/**
 * Switcher do notch ativo (BR-032/task 08): **Transcrição** (com waveform),
 * play/pause contextual, **Resumo** e **Sugestões**. A lupa (barra de
 * pesquisa) vive na barra, ao lado da marca (BR-052).
 *
 * A Transcrição fica destacada enquanto a sessão está captando — é o "modo vivo".
 * Clicar nela ou em Resumo abre o painel na aba correspondente; Sugestões alterna
 * a janela. O **Histórico** vive no grupo trailing (ícone, antes de Configurações).
 * **Resumo** e **Sugestões** também têm drill-hover (BR-036). O encerrar **não**
 * vive aqui (é só o timer — BR-022).
 */
export function NotchTabs({
  listening,
  paused,
  busy,
  suggestionsOpen,
  onOpenTranscription,
  onOpenSummary,
  onToggleSuggestions,
  onPause,
  onResume,
  recording,
  recordingBusy,
  recordingCapabilities,
  onToggleRecording,
  onPickRecordingTarget,
}: NotchTabsProps) {
  const { t } = useTranslation();
  return (
    <nav className="notch-tabs" aria-label={t("notch_tabs_hud_modes")} data-interactive>
      <button
        type="button"
        className={`notch-tabs__item${listening ? " notch-tabs__item--active" : ""}`}
        aria-current={listening ? "page" : undefined}
        title={t("notch_tabs_live_transcription")}
        onClick={onOpenTranscription}
      >
        <MicIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">{t("notch_tabs_transcription")}</span>
        <NotchWaveform active={listening} paused={paused} />
      </button>

      <button
        type="button"
        className="notch-tabs__transport"
        aria-label={paused ? t("notch_tabs_resume") : t("notch_tabs_pause")}
        title={paused ? t("notch_tabs_resume_capture") : t("notch_tabs_pause_capture")}
        disabled={busy}
        onClick={paused ? onResume : onPause}
      >
        {paused ? (
          <PlayIcon className="transport__icon" />
        ) : (
          <PauseIcon className="transport__icon" />
        )}
      </button>

      <DwellButton
        className="notch-tabs__item"
        title={t("notch_tabs_session_summary")}
        onActivate={onOpenSummary}
      >
        <SummaryIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">{t("notch_tabs_summary")}</span>
      </DwellButton>

      <DwellButton
        className={`notch-tabs__item${suggestionsOpen ? " notch-tabs__item--active" : ""}`}
        pressed={suggestionsOpen}
        title={t("notch_tabs_ai_suggestions")}
        onActivate={onToggleSuggestions}
      >
        <SparkleIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">{t("notch_tabs_suggestions")}</span>
      </DwellButton>

      <span className="notch-tabs__divider" aria-hidden="true" />

      <RecordingButton
        state={recording}
        capabilities={recordingCapabilities}
        busy={recordingBusy}
        onToggle={onToggleRecording}
        onPick={onPickRecordingTarget}
      />
    </nav>
  );
}
