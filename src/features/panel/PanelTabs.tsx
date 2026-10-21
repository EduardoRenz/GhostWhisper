import type { KeyboardEvent, ReactElement } from "react";
import {
  JournalIcon,
  MicIcon,
  SummaryIcon,
  type IconProps,
} from "../../components/icons";
import type { PanelTab } from "../../core/domain/settings";
import type { TranscriptActivity } from "../../core/application/useTranscriptActivity";

interface TabDefinition {
  id: PanelTab;
  label: string;
  Icon: (props: IconProps) => ReactElement;
}

const TABS: TabDefinition[] = [
  { id: "transcription", label: "Transcrição", Icon: MicIcon },
  { id: "summary", label: "Resumo", Icon: SummaryIcon },
  { id: "session", label: "Sessão", Icon: JournalIcon },
];

interface PanelTabsProps {
  active: PanelTab;
  onChange: (tab: PanelTab) => void;
  /** Atividade de transcrição por canal (efeito na aba Transcrição). */
  transcriptionActivity?: TranscriptActivity;
}

const IDLE: TranscriptActivity = { microphone: false, system: false };

/** Texto do tooltip/status da aba Transcrição. */
function activityLabel(activity: TranscriptActivity): string {
  if (activity.microphone && activity.system) {
    return "Transcrição processando microfone e Outros";
  }
  if (activity.microphone) {
    return "Transcrição processando microfone";
  }
  if (activity.system) {
    return "Transcrição processando Outros";
  }
  return "Transcrição em silêncio";
}

/**
 * *Pill* de abas do painel da sessão (BR-033), no estilo do notch do Perssua:
 * um conjunto fechado com ícone + rótulo por aba. A aba ativa fica preenchida.
 *
 * Acessível: `role="tablist"`, `aria-selected` por aba e navegação pelas setas
 * (←/→/Home/End).
 */
export function PanelTabs({
  active,
  onChange,
  transcriptionActivity = IDLE,
}: PanelTabsProps) {
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = TABS.findIndex((tab) => tab.id === active);
    const selectAt = (target: number) => {
      const next = TABS[(target + TABS.length) % TABS.length];
      if (next) {
        event.preventDefault();
        onChange(next.id);
      }
    };
    if (event.key === "ArrowRight") {
      selectAt(index + 1);
    } else if (event.key === "ArrowLeft") {
      selectAt(index - 1);
    } else if (event.key === "Home") {
      selectAt(0);
    } else if (event.key === "End") {
      selectAt(TABS.length - 1);
    }
  };

  const activity = activityLabel(transcriptionActivity);

  return (
    <div
      className="panel-tabs"
      role="tablist"
      aria-label="Painel da sessão"
      data-interactive
      onKeyDown={move}
    >
      {TABS.map(({ id, label, Icon }) => {
        const selected = active === id;
        const isTranscription = id === "transcription";
        const streamingMic =
          isTranscription && transcriptionActivity.microphone;
        const streamingSystem =
          isTranscription && transcriptionActivity.system;
        return (
          <button
            key={id}
            type="button"
            role="tab"
            id={`panel-tab-${id}`}
            aria-selected={selected}
            aria-controls={`panel-view-${id}`}
            tabIndex={selected ? 0 : -1}
            title={isTranscription ? activity : undefined}
            className={[
              "panel-tabs__tab",
              selected ? "panel-tabs__tab--active" : "",
              streamingMic ? "panel-tabs__tab--streaming-mic" : "",
              streamingSystem ? "panel-tabs__tab--streaming-system" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            onClick={() => onChange(id)}
          >
            {isTranscription && (
              <span className="panel-tabs__status-dot" aria-hidden="true" />
            )}
            <Icon className="panel-tabs__icon" size={16} />
            <span>{label}</span>
          </button>
        );
      })}
    </div>
  );
}
