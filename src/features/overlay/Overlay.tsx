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
import { ActionBar } from "./ActionBar";
import { AsrStatusBanner } from "./AsrStatusBanner";
import { LevelMeters } from "./LevelMeters";
import { TranscriptView } from "../transcript/TranscriptView";
import { TranslationView } from "../translation/TranslationView";
import { InsightsPanel } from "../insights/InsightsPanel";

/**
 * Casca do overlay: ActionBar sempre visível e painel expandido (transcrição,
 * tradução e insights). Apenas elementos `[data-interactive]` capturam o mouse;
 * o restante é click-through (input-shape por região no X11).
 */
export function Overlay() {
  const [surface, setSurface] = useState<Surface>("actionbar");
  const [stealth, setStealth] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const { segments } = useLiveTranscription();
  const asrStatus = useAsrStatus();
  const levels = useAudioLevel();
  const { state, busy, error, start, stop } = useSessionState();
  const { settings } = useSettings();
  // Destino "auto" ou igual à origem = não traduz.
  const translationApplies =
    settings.translation_enabled &&
    settings.target_language !== "auto" &&
    settings.target_language !== settings.source_language;
  const { translations } = useTranslation(segments, translationApplies);
  const pushedInsight = useInsightFeed();
  useInteractiveRegions();

  // Sincroniza o stealth com as settings persistidas.
  useEffect(() => {
    setStealth(settings.stealth);
  }, [settings.stealth]);

  // Insight por atalho: abre o painel expandido para exibi-lo.
  useEffect(() => {
    if (pushedInsight) {
      setSurface("expanded");
      void setSurfaceIpc("expanded");
    }
  }, [pushedInsight]);

  const changeSurface = (next: Surface) => {
    setSurface(next);
    void setSurfaceIpc(next);
  };

  const toggleStealth = () => {
    const next = !stealth;
    setStealth(next);
    void setStealthIpc(next);
  };

  const onOpenSettings = async () => {
    setSettingsError(null);
    try {
      await openSettingsWindow();
    } catch (caught) {
      setSettingsError(String(caught));
    }
  };

  const onStartDrag = () => {
    void startDragIpc();
  };

  return (
    <div className="overlay" data-surface={surface}>
      <ActionBar
        surface={surface}
        onSurfaceChange={changeSurface}
        stealth={stealth}
        onToggleStealth={toggleStealth}
        listening={state.listening}
        busy={busy}
        onStart={(channels) => void start(channels)}
        onStop={() => void stop()}
        onOpenSettings={() => void onOpenSettings()}
        onStartDrag={onStartDrag}
      />

      {surface === "expanded" && (
        <div className="overlay__panel">
          <LevelMeters
            levels={levels}
            channels={state.channels}
            listening={state.listening}
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
            externalInsight={pushedInsight}
          />
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
