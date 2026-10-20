import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { openSettingsWindow } from "../../adapters/ipc/overlay";
import { copyText } from "../../core/application/clipboard";
import { useLlmSource } from "../../core/application/useLlmSource";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useSummary } from "../../core/application/useSummary";
import { summaryTopic } from "../../core/domain/assistant";
import { formatClock } from "../../core/domain/transcript";
import { MarkdownView } from "../../components/MarkdownView";
import { StatusPill } from "../../components/StatusPill";

/** Formata o relógio de parede de uma versão do resumo. */
function formatUpdatedAt(ms: number): string {
  try {
    return new Date(ms).toLocaleTimeString("pt-BR");
  } catch {
    return "";
  }
}

interface SummaryViewProps {
  /** A janela do painel está visível. */
  opened: boolean;
  /** A aba Resumo está ativa (o automático só roda com ela visível). */
  active: boolean;
}

/**
 * Aba **Resumo** do painel da sessão (BR-033): o resumo vivo da sessão com
 * histórico de versões em memória — o que antes vivia na janela `summary`.
 *
 * O automático só chama o LLM com a **janela aberta e a aba ativa**
 * (CA-05.2); "Atualizar agora" funciona em qualquer aba.
 */
export function SummaryView({ opened, active }: SummaryViewProps) {
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
    opened: opened && active,
    auto,
    intervalSecs: settings.summary_interval_secs,
    active: state.listening && !state.paused,
  });

  // Modelo que respondeu o **resumo** (BR-051): o escopo filtra o evento para
  // não mostrar o modelo da sugestão.
  const llmSource = useLlmSource("capability:summary");

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

  return (
    <div className="summary-view" data-interactive>
      <div className="summary-view__toolbar">
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
          title="Resumo automático enquanto a aba estiver visível"
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
      </div>

      {(loading || hasMore) && (
        <StatusPill
          label={
            loading
              ? "Gerando resumo completo…"
              : "Processando o restante do resumo…"
          }
        />
      )}

      {viewing && (
        <div className="summary-view__banner">
          <span>Vendo versão de {formatUpdatedAt(viewing.updated_at_ms)}</span>
          <button
            type="button"
            className="panel__button panel__button--small"
            onClick={showLatest}
          >
            Voltar ao atual
          </button>
        </div>
      )}

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
            {displayed.subject && displayed.subject.trim() && (
              <h2 className="summary-view__subject">
                {displayed.subject.trim()}
              </h2>
            )}
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
          <p className="panel__empty">
            O texto completo está sendo gerado e aparece aqui assim que ficar
            pronto.
          </p>
        ) : (
          <p className="panel__empty">
            Ainda não há fala suficiente para resumir.
          </p>
        )}
      </div>

      {!viewing && summary && (
        <div className="summary-view__meta">
          {hasMore ? "Resumo parcial" : "Resumo completo"} · atualizado às{" "}
          {formatUpdatedAt(summary.updated_at_ms)}
          {summary.summarized_from_ms > 0
            ? " · resumo a partir do meio da sessão"
            : ""}
          {llmSource && (
            <>
              {" · "}
              {llmSource.label} · {llmSource.model}
              {llmSource.is_fallback ? " (fallback)" : ""}
            </>
          )}
        </div>
      )}
    </div>
  );
}
