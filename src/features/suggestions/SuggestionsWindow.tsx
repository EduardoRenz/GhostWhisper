import { useEffect, useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useSuggestions } from "../../core/application/useSuggestions";
import { useSuggestionCadence } from "../../core/application/useSuggestionCadence";
import type { Suggestion } from "../../core/domain/assistant";
import { saveSettings } from "../../adapters/ipc/settings";
import { openSettingsWindow } from "../../adapters/ipc/overlay";
import { SlidersIcon, SparkleIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import { StatusPill } from "../../components/StatusPill";
import { AgentSelector } from "./AgentSelector";
import { AutoSuggestControl } from "./AutoSuggestControl";
import { SuggestionCard } from "./SuggestionCard";
import { SuggestionHistory } from "./SuggestionHistory";
import { PanelFrame } from "../panels/PanelFrame";

/** Janela de sugestões com seleção de agente. */
export function SuggestionsWindow() {
  const opened = usePanelOpen("suggestions");
  const { state } = useSessionState();
  const { settings, reload } = useSettings();
  const { modes } = useAssistantModes(opened);
  const cadence = useSuggestionCadence();
  const [modeId, setModeId] = useState(settings.assistant_mode_id);
  const [extraContext, setExtraContext] = useState("");
  /** O contexto extra fica recolhido por padrão (abre pelo ícone do header). */
  const [extraOpen, setExtraOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const auto = settings.suggestion_auto_refresh;

  useEffect(() => {
    setModeId(settings.assistant_mode_id);
  }, [settings.assistant_mode_id]);

  const {
    current,
    quick,
    awaitingFull,
    history,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    pin,
  } = useSuggestions({
    sessionId: state.session_id,
    modeId,
    extraContext,
  });

  const selectMode = (id: string) => {
    setModeId(id);
    void saveSettings({ ...settings, assistant_mode_id: id }).then(() =>
      reload(),
    );
  };

  // O automático vive no core (reage a fala nova, mesmo com a janela fechada);
  // aqui só ligamos/desligamos.
  const toggleAuto = () => {
    void saveSettings({
      ...settings,
      suggestion_auto_refresh: !settings.suggestion_auto_refresh,
    }).then(() => reload());
  };

  /** Copia uma sugestão (a atual, ou a de um item do histórico). */
  const onCopy = async (target?: Suggestion | null) => {
    const suggestion = target ?? current;
    if (suggestion && (await copyText(suggestion.text))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    }
  };

  const status = !state.listening
    ? "Sem sessão"
    : state.paused
      ? "Pausado"
      : "Gravando";

  // Cabeçalho: faísca + título + badge de estado (BR-039).
  const heading = (
    <div className="suggestions-title">
      <span className="suggestions-title__spark" aria-hidden="true">
        <SparkleIcon size={14} />
      </span>
      <span className="panel__title">Sugestões</span>
      {state.listening ? (
        <Pill
          className="live-badge"
          tone={state.paused ? "neutral" : "accent"}
          mono
          title={status}
        >
          <span className="live-badge__dot" />
          {state.paused ? "PAUSADO" : "LIVE"}
        </Pill>
      ) : null}
    </div>
  );

  const actions = (
    <>
      <button
        type="button"
        className={`icon-button${extraOpen ? " icon-button--on" : ""}`}
        onClick={() => setExtraOpen((previous) => !previous)}
        aria-expanded={extraOpen}
        title="Contexto extra (opcional)"
        aria-label="Contexto extra (opcional)"
      >
        <SlidersIcon size={16} />
      </button>
      <AutoSuggestControl
        cadence={cadence}
        paused={!auto}
        onToggle={toggleAuto}
      />
      <button
        type="button"
        className="suggest-now"
        disabled={loading || !state.session_id}
        onClick={() => void suggestNow()}
        title="Gerar uma sugestão agora"
      >
        <SparkleIcon size={12} />
        {loading ? "Analisando…" : "Sugerir agora"}
      </button>
    </>
  );

  return (
    <PanelFrame label="suggestions" heading={heading} actions={actions}>
      <AgentSelector modes={modes} modeId={modeId} onSelect={selectMode} />
      {extraOpen && (
        <label className="panel__extra">
          <span className="panel__extra-label">Contexto extra (opcional)</span>
          <textarea
            className="panel__textarea"
            placeholder="Cole aqui o enunciado ou trecho de código para dar mais contexto ao agente."
            value={extraContext}
            onChange={(event) => setExtraContext(event.target.value)}
          />
        </label>
      )}
      <div className="panel__scroll">
        {error ? (
          <div className="panel__error">
            <p>{error}</p>
            <button
              type="button"
              className="panel__button"
              onClick={() => void openSettingsWindow()}
            >
              Abrir configurações
            </button>
          </div>
        ) : !state.session_id ? (
          <div className="suggestion-empty">
            <span className="suggestion-empty__icon">
              <SparkleIcon size={18} />
            </span>
            <p className="suggestion-empty__title">Nenhuma escuta ativa</p>
            <p className="suggestion-empty__hint">
              Inicie uma escuta no notch para receber sugestões.
            </p>
          </div>
        ) : current || quick ? (
          <>
            {loading && current && (
              <StatusPill
                label="Gerando uma nova sugestão…"
                title="A sugestão atual continua na tela até a nova ficar pronta; a anterior vai para o histórico"
              />
            )}
            <SuggestionCard
              current={current}
              quick={quick}
              pinned={Boolean(pinned)}
              copied={copied}
              awaitingFull={awaitingFull}
              onCopy={() => void onCopy()}
              onPin={() => void pin()}
            />
            {skipped && (
              <p className="panel__hint">Aguardando nova fala…</p>
            )}
          </>
        ) : loading ? (
          <>
            <StatusPill label="Gerando sugestão…" />
            <p className="panel__empty">Analisando a conversa…</p>
          </>
        ) : (
          <div className="suggestion-empty">
            <span className="suggestion-empty__icon">
              <SparkleIcon size={18} />
            </span>
            <p className="suggestion-empty__title">Ainda sem sugestão</p>
            <p className="suggestion-empty__hint">
              Fale na reunião: o agente sugere o que dizer a cada fala nova.
            </p>
          </div>
        )}

        <SuggestionHistory
          items={history.filter((item) => item.at_ms !== current?.at_ms)}
          pinnedId={pinned}
          onCopy={(suggestion) => void onCopy(suggestion)}
          onPin={(suggestion) => void pin(suggestion)}
        />
      </div>
    </PanelFrame>
  );
}
