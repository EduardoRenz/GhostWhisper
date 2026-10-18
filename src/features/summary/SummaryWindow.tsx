import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { openSettingsWindow } from "../../adapters/ipc/overlay";
import { copyText } from "../../core/application/clipboard";
import { usePanelOpen } from "../../core/application/usePanelOpen";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useSummary } from "../../core/application/useSummary";
import { summaryTopic } from "../../core/domain/assistant";
import { formatClock } from "../../core/domain/transcript";
import { MarkdownView } from "../../components/MarkdownView";
import { PanelFrame } from "../panels/PanelFrame";

/** Formata o relógio de parede de uma versão do resumo. */
function formatUpdatedAt(ms: number): string {
  try {
    return new Date(ms).toLocaleTimeString("pt-BR");
  } catch {
    return "";
  }
}

/** Janela do resumo vivo da sessão, com histórico de versões em memória. */
export function SummaryWindow() {
  const opened = usePanelOpen("summary");
  const { state } = useSessionState();
  const { settings } = useSettings();
  const [auto, setAuto] = useState(true);
  const [copied, setCopied] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollTop = useRef(0);

  useEffect(() => {
    setAuto(settings.summary_auto_refresh);
  }, [settings.summary_auto_refresh]);

  const {
    summary,
    history,
    viewing,
    loading,
    error,
    hasMore,
    regenerate,
    select,
    showLatest,
  } = useSummary({
    sessionId: state.session_id,
    opened,
    auto,
    intervalSecs: settings.summary_interval_secs,
    active: state.listening && !state.paused,
  });

  // A versão exibida é a selecionada no histórico (congelada) ou a atual.
  const displayed = viewing ?? summary;

  // Preserva a rolagem quando o resumo ao vivo cresce; ao trocar de versão,
  // volta ao topo.
  useLayoutEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollTop.current;
    }
  }, [displayed?.markdown]);

  useLayoutEffect(() => {
    scrollTop.current = 0;
    if (scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
  }, [viewing?.updated_at_ms]);

  const onCopy = async () => {
    if (displayed && (await copyText(displayed.markdown))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    }
  };

  const status = !state.listening
    ? "Sem sessão"
    : state.paused
      ? "Pausado"
      : "Gravando";
  const subtitle = displayed
    ? `${summaryTopic(displayed)} · ${status}`
    : state.session_id
      ? `${state.session_id.slice(0, 8)} · ${status}`
      : status;

  const actions = (
    <>
      <button
        type="button"
        className="panel__button"
        disabled={loading || !state.session_id}
        onClick={() => void regenerate()}
        title="Regera o resumo do zero"
      >
        {loading ? "Gerando…" : "Atualizar"}
      </button>
      <button
        type="button"
        className={`panel__toggle${auto ? " panel__toggle--on" : ""}`}
        onClick={() => setAuto((previous) => !previous)}
        title="Resumo automático enquanto a janela estiver aberta"
      >
        {auto ? `Auto ${settings.summary_interval_secs}s` : "Auto off"}
      </button>
      <button
        type="button"
        className="panel__button"
        disabled={!displayed}
        onClick={() => void onCopy()}
      >
        {copied ? "Copiado" : "Copiar"}
      </button>
    </>
  );

  const footer = viewing ? (
    <>
      <span className="panel__meta">
        Vendo versão de {formatUpdatedAt(viewing.updated_at_ms)}
      </span>
      <button
        type="button"
        className="panel__button panel__button--small"
        onClick={showLatest}
      >
        Voltar ao atual
      </button>
    </>
  ) : (
    summary && (
      <span className="panel__meta">
        Atualizado às {formatUpdatedAt(summary.updated_at_ms)}
        {summary.summarized_from_ms > 0
          ? " · resumo a partir do meio da sessão"
          : ""}
        {hasMore ? " · processando o restante…" : ""}
      </span>
    )
  );

  return (
    <PanelFrame
      label="summary"
      title="Resumo da sessão"
      subtitle={subtitle}
      actions={actions}
      footer={footer}
    >
      <div
        className="panel__scroll"
        ref={scrollRef}
        onScroll={() => {
          if (scrollRef.current) {
            scrollTop.current = scrollRef.current.scrollTop;
          }
        }}
      >
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
            Inicie uma escuta para gerar o resumo da conversa.
          </p>
        ) : displayed ? (
          <>
            <MarkdownView markdown={displayed.markdown} />
            {history.length > 1 && (
              <details className="panel__history">
                <summary>Resumos anteriores ({history.length - 1})</summary>
                <ul>
                  {history.slice(1).map((version) => (
                    <li key={version.updated_at_ms}>
                      <button
                        type="button"
                        className={`panel__history-item${
                          viewing?.updated_at_ms === version.updated_at_ms
                            ? " panel__history-item--active"
                            : ""
                        }`}
                        onClick={() => select(version)}
                      >
                        <span className="panel__history-time">
                          {formatUpdatedAt(version.updated_at_ms)}
                        </span>
                        <span className="panel__history-text">
                          {summaryTopic(version)}
                        </span>
                        <span className="panel__history-meta">
                          cobre até {formatClock(version.summarized_until_ms)}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </>
        ) : loading ? (
          <p className="panel__empty">Gerando o resumo…</p>
        ) : (
          <p className="panel__empty">
            Ainda não há fala suficiente para resumir.
          </p>
        )}
      </div>
    </PanelFrame>
  );
}
