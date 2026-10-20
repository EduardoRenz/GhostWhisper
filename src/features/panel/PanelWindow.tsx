import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import { saveSettings } from "../../adapters/ipc/settings";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSettings } from "../../core/application/useSettings";
import type { PanelTab } from "../../core/domain/settings";
import { WaveformIcon } from "../../components/icons";
import { PanelFrame } from "../panels/PanelFrame";
import { PanelTabs } from "./PanelTabs";
import { JournalView } from "../journal/JournalView";
import { SummaryView } from "./SummaryView";
import { TranscriptionView } from "./TranscriptionView";

/** Evento que pede a troca de aba (tray "Abrir resumo"). */
const PANEL_TAB_EVENT = "panel_tab";

/**
 * Painel da sessão (janela `chat`): **Transcrição | Resumo** em abas (BR-033).
 *
 * Substitui as antigas janelas `chat` e `summary` por uma só. A aba ativa é
 * lembrada em `Settings.panel_tab`; o tray pode pedir a aba Resumo pelo evento
 * `panel_tab`. As duas visões ficam **montadas** (só a inativa fica `hidden`),
 * para trocar de aba sem reiniciar transcrição/rolagem.
 */
export function PanelWindow() {
  const opened = usePanelOpen("chat");
  const { settings, reload } = useSettings();
  const [tab, setTab] = useState<PanelTab>(settings.panel_tab);

  // As settings carregam de forma assíncrona: acompanha a aba persistida.
  useEffect(() => {
    setTab(settings.panel_tab);
  }, [settings.panel_tab]);

  // Pedido externo de aba (tray "Abrir resumo").
  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void subscribe<string>(PANEL_TAB_EVENT, (requested) => {
      if (
        requested === "transcription" ||
        requested === "summary" ||
        requested === "session"
      ) {
        setTab(requested);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  const changeTab = (next: PanelTab) => {
    if (next === tab) {
      return;
    }
    setTab(next);
    void saveSettings({ ...settings, panel_tab: next }).then(() => reload());
  };

  const heading = (
    <div className="panel__heading-row">
      {tab === "transcription" && (
        <span className="panel__title-badge" aria-hidden="true">
          <WaveformIcon size={16} />
        </span>
      )}
      <span className="panel__title">
        {tab === "summary"
          ? "Resumo"
          : tab === "session"
            ? "Sessão"
            : "Transcrição ao Vivo"}
      </span>
    </div>
  );

  return (
    <PanelFrame label="chat" heading={heading}>
      <PanelTabs active={tab} onChange={changeTab} />

      <div className="panel-tab-body">
        <div
          className="panel-view"
          id="panel-view-transcription"
          role="tabpanel"
          aria-labelledby="panel-tab-transcription"
          hidden={tab !== "transcription"}
        >
          <TranscriptionView />
        </div>
        <div
          className="panel-view"
          id="panel-view-summary"
          role="tabpanel"
          aria-labelledby="panel-tab-summary"
          hidden={tab !== "summary"}
        >
          <SummaryView opened={opened} active={tab === "summary"} />
        </div>

        <div
          className="panel-view"
          id="panel-view-session"
          role="tabpanel"
          aria-labelledby="panel-tab-session"
          hidden={tab !== "session"}
        >
          <JournalView />
        </div>
      </div>
    </PanelFrame>
  );
}
