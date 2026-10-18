import { useEffect, useState } from "react";
import {
  openSettingsWindow,
  setStealth as setStealthIpc,
  setSurface as setSurfaceIpc,
  startDrag as startDragIpc,
  type Surface,
} from "../../adapters/ipc/overlay";
import { useAsrStatus } from "../../core/application/useAsrStatus";
import { useAudioLevel } from "../../core/application/useAudioLevel";
import { useInsightFeed } from "../../core/application/useInsightFeed";
import { useInteractiveRegions } from "../../core/application/useInteractiveRegions";
import { useLiveTranscription } from "../../core/application/useLiveTranscription";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useTranslation } from "../../core/application/useTranslation";
import { ALL_CHANNELS } from "../../core/domain/transcript";
import { ActionBar } from "./ActionBar";
import { AsrStatusBanner } from "./AsrStatusBanner";
import { LevelMeters } from "./LevelMeters";
import { HistoryPanel } from "../history/HistoryPanel";
import { TranscriptView } from "../transcript/TranscriptView";
import { TranslationView } from "../translation/TranslationView";
import { InsightsPanel } from "../insights/InsightsPanel";

/** Visão mostrada no painel expandido. */
type PanelView = "live" | "history";

/**
 * Casca do overlay: ActionBar sempre visível e painel expandido (transcrição,
 * tradução, insights e histórico). Apenas elementos `[data-interactive]`
 * capturam o mouse; o restante é click-through (input-shape por região no X11).
 */
export function Overlay() {
  const [surface, setSurface] = useState<Surface>("actionbar");
  const [view, setView] = useState<PanelView>("live");
  const [stealth, setStealth] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const { state, busy, error, start, pause, resume, stop, continueSession, setChannels } =
    useSessionState();
  const { segments } = useLiveTranscription(state.session_id);
  const asrStatus = useAsrStatus();
  const levels = useAudioLevel();
  const { settings } = useSettings();
  // Destino "auto" ou igual à origem = não traduz.
  const translationApplies =
    settings.translation_enabled &&
    settings.target_language !== "auto" &&
    settings.target_language !== settings.source_language;
  const { translations } = useTranslation(segments, translationApplies);
  const pushedInsight = useInsightFeed();
  useInteractiveRegions();

  const expanded = surface === "expanded" || surface === "expanded-tall";

  /** Aplica a superfície, respeitando a visão escolhida no painel. */
  const changeSurface = (next: Surface) => {
    const resolved: Surface =
      next === "expanded" && view === "history" ? "expanded-tall" : next;
    setSurface(resolved);
    void setSurfaceIpc(resolved);
  };

  const showLive = () => {
    setView("live");
    changeSurface("expanded");
  };

  const showHistory = () => {
    setView("history");
    changeSurface("expanded-tall");
  };

  // Sincroniza o stealth com as settings persistidas.
  useEffect(() => {
    setStealth(settings.stealth);
  }, [settings.stealth]);

  // Insight por atalho: abre o painel de transcrição ao vivo para exibi-lo.
  useEffect(() => {
    if (pushedInsight) {
      setView("live");
      void setSurfaceIpc("expanded");
      setSurface("expanded");
    }
  }, [pushedInsight]);

  const toggleStealth = () => {
    const next = !stealth;
    setStealth(next);
    void setStealthIpc(next);
  };

  const openSettings = async () => {
    setSettingsError(null);
    try {
      await openSettingsWindow();
    } catch (caught) {
      setSettingsError(String(caught));
    }
  };

  const onContinueSession = (id: string) => {
    void continueSession(id, ALL_CHANNELS);
    showLive();
  };

  return (
    <div className="overlay" data-surface={surface}>
      <ActionBar
        surface={surface}
        onSurfaceChange={changeSurface}
        stealth={stealth}
        onToggleStealth={toggleStealth}
        listening={state.listening}
        paused={state.paused}
        channels={state.channels}
        busy={busy}
        onStart={(channels) => void start(channels)}
        onPause={() => void pause()}
        onResume={() => void resume()}
        onStop={() => void stop()}
        onChannelsChange={(channels) => void setChannels(channels)}
        onOpenSettings={() => void openSettings()}
        onStartDrag={() => void startDragIpc()}
      />

      {expanded && (
        <div className="overlay__panel">
          <div className="overlay__tabs" data-interactive>
            <button
              type="button"
              className={`overlay__tab${view === "live" ? " overlay__tab--active" : ""}`}
              onClick={showLive}
            >
              Ao vivo
            </button>
            <button
              type="button"
              className={`overlay__tab${view === "history" ? " overlay__tab--active" : ""}`}
              onClick={showHistory}
            >
              Histórico
            </button>
          </div>

          {view === "history" ? (
            <HistoryPanel
              activeSessionId={state.session_id}
              onContinue={onContinueSession}
            />
          ) : (
            <>
              <LevelMeters
                levels={levels}
                channels={state.channels}
                listening={state.listening && !state.paused}
              />
              <TranscriptView segments={segments} />
              {translationApplies && (
                <TranslationView
                  segments={segments}
                  translations={translations}
                  targetLanguage={settings.target_language}
                />
              )}
              <InsightsPanel
                sessionId={state.session_id}
                externalSuggestion={pushedInsight}
              />
            </>
          )}
        </div>
      )}

      <AsrStatusBanner status={asrStatus} />

      {(error || settingsError) && (
        <p className="overlay__error" data-interactive>
          {error ?? settingsError}
        </p>
      )}
    </div>
  );
}
