import { useEffect, useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useSuggestions } from "../../core/application/useSuggestions";
import { saveSettings } from "../../adapters/ipc/settings";
import { openSettingsWindow } from "../../adapters/ipc/overlay";
import { MarkdownView } from "../../components/MarkdownView";
import { PanelFrame } from "../panels/PanelFrame";

/** Janela de sugestões com seleção de agente. */
export function SuggestionsWindow() {
  const opened = usePanelOpen("suggestions");
  const { state } = useSessionState();
  const { settings, reload } = useSettings();
  const { modes } = useAssistantModes(opened);
  const [modeId, setModeId] = useState(settings.assistant_mode_id);
  const [extraContext, setExtraContext] = useState("");
  const [copied, setCopied] = useState(false);
  const auto = settings.suggestion_auto_refresh;

  useEffect(() => {
    setModeId(settings.assistant_mode_id);
  }, [settings.assistant_mode_id]);

  const {
    current,
    history,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    pin,
    select,
  } = useSuggestions({
    sessionId: state.session_id,
    opened,
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

  const onCopy = async () => {
    if (current && (await copyText(current.text))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    }
  };

  const status = !state.listening
    ? "Sem sessão"
    : state.paused
      ? "Pausado"
      : "Gravando";

  const actions = (
    <>
      <select
        className="panel__select"
        aria-label="Agente"
        value={modeId}
        onChange={(event) => selectMode(event.target.value)}
      >
        {modes.length === 0 && <option value={modeId}>{modeId}</option>}
        {modes.map((mode) => (
          <option key={mode.id} value={mode.id}>
            {mode.name}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={`panel__toggle${auto ? " panel__toggle--on" : ""}`}
        onClick={toggleAuto}
        title="Sugestões automáticas a cada fala nova (funcionam mesmo com esta janela fechada)"
      >
        {auto ? "Auto" : "Auto off"}
      </button>
      <button
        type="button"
        className="panel__button"
        disabled={loading || !state.session_id}
        onClick={() => void suggestNow()}
      >
        {loading ? "Analisando…" : "Sugerir agora"}
      </button>
      <button
        type="button"
        className="panel__button"
        disabled={!current}
        onClick={() => void pin()}
      >
        {pinned ? "Fixada" : "Fixar"}
      </button>
      <button
        type="button"
        className="panel__button"
        disabled={!current}
        onClick={() => void onCopy()}
      >
        {copied ? "Copiado" : "Copiar"}
      </button>
    </>
  );

  const footer = current && (
    <span className="panel__meta">
      {current.mode_name} · contexto até {Math.round(current.context_until_ms / 1000)}s
    </span>
  );

  return (
    <PanelFrame
      label="suggestions"
      title="Sugestões"
      subtitle={status}
      actions={actions}
      footer={footer}
    >
      <details className="panel__extra">
        <summary>Contexto extra (opcional)</summary>
        <textarea
          className="panel__textarea"
          placeholder="Cole aqui o enunciado ou trecho de código para dar mais contexto ao agente."
          value={extraContext}
          onChange={(event) => setExtraContext(event.target.value)}
        />
      </details>
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
          <p className="panel__empty">
            Inicie uma escuta para receber sugestões.
          </p>
        ) : current ? (
          <>
            <MarkdownView markdown={current.text} />
            {skipped && (
              <p className="panel__hint">Aguardando nova fala…</p>
            )}
          </>
        ) : loading ? (
          <p className="panel__empty">Analisando a conversa…</p>
        ) : (
          <p className="panel__empty">
            Ainda não há fala suficiente para sugerir.
          </p>
        )}

        {history.length > 1 && (
          <details className="panel__history">
            <summary>Sugestões anteriores ({history.length - 1})</summary>
            <ul>
              {history.slice(1).map((suggestion) => (
                <li key={`${suggestion.at_ms}-${suggestion.message_id ?? suggestion.text.slice(0, 12)}`}>
                  <button
                    type="button"
                    className="panel__history-item"
                    onClick={() => select(suggestion)}
                  >
                    <span className="panel__history-time">
                      {Math.round(suggestion.at_ms / 1000)}s
                    </span>
                    <span className="panel__history-text">
                      {suggestion.text.replace(/[#>*`]/g, "").slice(0, 90)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </details>
        )}
      </div>
    </PanelFrame>
  );
}
