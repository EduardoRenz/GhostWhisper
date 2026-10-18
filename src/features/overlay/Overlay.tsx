import { useEffect, useMemo, useState } from "react";
import {
  openSettingsWindow,
  setStealth as setStealthIpc,
  setSurface as setSurfaceIpc,
  startDrag as startDragIpc,
  type Surface,
} from "../../adapters/ipc/overlay";
import { useAsrStatus } from "../../core/application/useAsrStatus";
import { useInsight } from "../../core/application/useInsight";
import { useInsightFeed } from "../../core/application/useInsightFeed";
import { useInteractiveRegions } from "../../core/application/useInteractiveRegions";
import { useLiveTranscription } from "../../core/application/useLiveTranscription";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useTranslation } from "../../core/application/useTranslation";
import { buildChatTimeline } from "../../core/domain/chat";
import { ALL_CHANNELS } from "../../core/domain/transcript";
import { ActionBar } from "./ActionBar";
import { AsrStatusBanner } from "./AsrStatusBanner";
import { LevelMeters } from "./LevelMeters";
import { ChatView } from "../chat/ChatView";
import { HistoryPanel } from "../history/HistoryPanel";
import { TranslationView } from "../translation/TranslationView";

/** Visão mostrada no painel expandido. */
type PanelView = "live" | "history";

/**
 * Casca do overlay: ActionBar sempre visível e painel expandido (conversa em
 * bolhas, tradução e histórico). Apenas elementos `[data-interactive]` capturam o
 * mouse; o restante é click-through (input-shape por região no X11).
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
  const { settings } = useSettings();
  // Destino "auto" ou igual à origem = não traduz.
  const translationApplies =
    settings.translation_enabled &&
    settings.target_language !== "auto" &&
    settings.target_language !== settings.source_language;
  const { translations } = useTranslation(segments, translationApplies);
  // As sugestões (inclusive as do atalho, que chegam por evento) entram na mesma
  // conversa das falas — BR-020.
  const {
    suggestions,
    loading: insightLoading,
    error: insightError,
    request: requestInsight,
  } = useInsight(state.session_id);
  const pushedInsight = useInsightFeed();
  const { bubbles, pending } = useMemo(
    () => buildChatTimeline(segments, suggestions),
    [segments, suggestions],
  );
  // O cabeçalho é memoizado: o `ChatView` é memoizado e um elemento novo a cada
  // render anularia a memoização (o overlay re-renderiza com frequência).
  const chatHeader = useMemo(
    () => (
      <button
        type="button"
        className="overlay__button"
        data-interactive
        disabled={insightLoading || !state.session_id}
        title="Pede uma sugestão da IA com base na conversa"
        onClick={() => void requestInsight()}
      >
        {insightLoading ? "Gerando…" : "Sugerir"}
      </button>
    ),
    [insightLoading, requestInsight, state.session_id],
  );
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

  // Insight por atalho: abre o painel da conversa ao vivo para exibi-lo.
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

  const shownError = error ?? settingsError ?? insightError;

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
        startedAtMs={state.started_at_ms}
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
              Conversa
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
                channels={state.channels}
                listening={state.listening && !state.paused}
              />
              <ChatView
                bubbles={bubbles}
                pending={pending}
                header={chatHeader}
              />
              {translationApplies && (
                <TranslationView
                  segments={segments}
                  translations={translations}
                  targetLanguage={settings.target_language}
                />
              )}
            </>
          )}
        </div>
      )}

      <AsrStatusBanner status={asrStatus} />

      {shownError && (
        <p className="overlay__error" data-interactive>
          {shownError}
        </p>
      )}
    </div>
  );
}
