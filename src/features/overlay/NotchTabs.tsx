import { HistoryIcon, MicIcon, SummaryIcon } from "../../components/icons";
import { PauseIcon, PlayIcon } from "../../components/TransportIcons";
import { NotchWaveform } from "./NotchWaveform";

interface NotchTabsProps {
  /** Sessão captando (a aba viva). */
  listening: boolean;
  /** Captura pausada (o transporte vira "play" e o waveform congela). */
  paused: boolean;
  /** Uma operação de sessão está em curso. */
  busy: boolean;
  /** A janela de histórico está visível. */
  historyOpen: boolean;
  /** Abre/foca o painel na aba Transcrição. */
  onOpenTranscription: () => void;
  /** Abre/foca o painel na aba Resumo. */
  onOpenSummary: () => void;
  /** Mostra/esconde o histórico. */
  onToggleHistory: () => void;
  onPause: () => void;
  onResume: () => void;
}

/**
 * Switcher do notch ativo (BR-032/task 08): **Transcrição** (com waveform),
 * play/pause contextual, **Histórico** e **Resumo**.
 *
 * A Transcrição fica destacada enquanto a sessão está captando — é o "modo vivo".
 * Clicar nela ou em Resumo abre o painel na aba correspondente; Histórico alterna
 * a janela. O encerrar **não** vive aqui (é só o timer — BR-022).
 */
export function NotchTabs({
  listening,
  paused,
  busy,
  historyOpen,
  onOpenTranscription,
  onOpenSummary,
  onToggleHistory,
  onPause,
  onResume,
}: NotchTabsProps) {
  return (
    <nav className="notch-tabs" aria-label="Modos do HUD" data-interactive>
      <button
        type="button"
        className={`notch-tabs__item${listening ? " notch-tabs__item--active" : ""}`}
        aria-current={listening ? "page" : undefined}
        title="Transcrição ao vivo"
        onClick={onOpenTranscription}
      >
        <MicIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">Transcrição</span>
        <NotchWaveform active={listening} paused={paused} />
      </button>

      <button
        type="button"
        className="notch-tabs__transport"
        aria-label={paused ? "Retomar" : "Pausar"}
        title={paused ? "Retomar a captura" : "Pausar a captura"}
        disabled={busy}
        onClick={paused ? onResume : onPause}
      >
        {paused ? (
          <PlayIcon className="transport__icon" />
        ) : (
          <PauseIcon className="transport__icon" />
        )}
      </button>

      <button
        type="button"
        className={`notch-tabs__item${historyOpen ? " notch-tabs__item--active" : ""}`}
        aria-pressed={historyOpen}
        title="Histórico de sessões"
        onClick={onToggleHistory}
      >
        <HistoryIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">Histórico</span>
      </button>

      <button
        type="button"
        className="notch-tabs__item"
        title="Resumo da sessão"
        onClick={onOpenSummary}
      >
        <SummaryIcon className="notch-tabs__icon" size={15} />
        <span className="notch-tabs__label">Resumo</span>
      </button>
    </nav>
  );
}
