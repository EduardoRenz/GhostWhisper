import type { KeyboardEvent, ReactElement } from "react";
import { MicIcon, SummaryIcon, type IconProps } from "../../components/icons";
import { useTranslation, type TranslationKey } from "../../core/l10n";

/** Abas do detalhe da sessão no histórico. */
export type DetailTabId = "summary" | "transcription";

interface TabDefinition {
  id: DetailTabId;
  label: TranslationKey;
  Icon: (props: IconProps) => ReactElement;
}

const TABS: TabDefinition[] = [
  { id: "summary", label: "panel_tab_summary", Icon: SummaryIcon },
  { id: "transcription", label: "panel_tab_transcription", Icon: MicIcon },
];

interface DetailTabsProps {
  active: DetailTabId;
  onChange: (tab: DetailTabId) => void;
}

/**
 * Pills segmentadas "Resumo"/"Transcrição" do detalhe da sessão, no mesmo
 * padrão visual do `PanelTabs` do painel da sessão (BR-033).
 *
 * Acessível: `role="tablist"`, `aria-selected` por aba e navegação pelas
 * setas (←/→/Home/End). Os ids são prefixados com `history-` para não
 * colidir com as abas do painel da sessão.
 */
export function DetailTabs({ active, onChange }: DetailTabsProps) {
  const { t } = useTranslation();

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

  return (
    <div
      className="panel-tabs history__tabs"
      role="tablist"
      aria-label={t("history_session_summary")}
      data-interactive
      onKeyDown={move}
    >
      {TABS.map(({ id, label, Icon }) => {
        const selected = active === id;
        return (
          <button
            key={id}
            type="button"
            role="tab"
            id={`history-tab-${id}`}
            aria-selected={selected}
            aria-controls={`history-view-${id}`}
            tabIndex={selected ? 0 : -1}
            className={`panel-tabs__tab${
              selected ? " panel-tabs__tab--active" : ""
            }`}
            onClick={() => onChange(id)}
          >
            <Icon className="panel-tabs__icon" size={16} />
            <span>{t(label)}</span>
          </button>
        );
      })}
    </div>
  );
}
