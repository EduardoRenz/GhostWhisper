import { useCallback, useEffect, useRef, useState } from "react";
import { summaryGenerate, summaryGet } from "../../adapters/ipc/summary";
import type { MeetingSummary } from "../domain/assistant";

interface UseSummaryOptions {
  /** Sessão alvo (`null` = nenhuma; a janela mostra estado vazio). */
  sessionId: string | null;
  /** `true` enquanto a janela está visível (para o automático). */
  opened: boolean;
  /** Automático ligado (configuração + pausa local). */
  auto: boolean;
  intervalSecs: number;
  /** Sessão gravando (não pausada). */
  active: boolean;
}

/**
 * Resumo da sessão: carrega o gravado, gera ao abrir, atualiza no intervalo e
 * retoma rápido enquanto houver trechos pendentes (`has_more`).
 */
export function useSummary({
  sessionId,
  opened,
  auto,
  intervalSecs,
  active,
}: UseSummaryOptions): {
  summary: MeetingSummary | null;
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  refresh: () => Promise<void>;
  regenerate: () => Promise<void>;
} {
  const [summary, setSummary] = useState<MeetingSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const inFlight = useRef(false);
  const sessionRef = useRef<string | null>(sessionId);

  useEffect(() => {
    sessionRef.current = sessionId;
    setSummary(null);
    setError(null);
    setHasMore(false);
    if (!sessionId) {
      return;
    }
    let cancelled = false;
    void summaryGet(sessionId).then((stored) => {
      if (cancelled || !stored || stored.session_id !== sessionRef.current) {
        return;
      }
      setSummary((previous) =>
        previous && previous.updated_at_ms >= stored.updated_at_ms
          ? previous
          : stored,
      );
    });
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const run = useCallback(async (mode: "auto" | "manual") => {
    if (inFlight.current || !sessionRef.current) {
      return;
    }
    inFlight.current = true;
    setLoading(true);
    setError(null);
    try {
      const result = await summaryGenerate(sessionRef.current, mode);
      if (result && result.summary.session_id === sessionRef.current) {
        setSummary((previous) =>
          previous && previous.updated_at_ms > result.summary.updated_at_ms
            ? previous
            : result.summary,
        );
        setHasMore(result.has_more);
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      inFlight.current = false;
      setLoading(false);
    }
  }, []);

  // Gera ao abrir e ao trocar de sessão com a janela aberta.
  useEffect(() => {
    if (opened && sessionId) {
      void run("auto");
    }
  }, [opened, sessionId, run]);

  // Atualização periódica (só com a janela aberta e sessão gravando).
  useEffect(() => {
    if (!opened || !auto || !active) {
      return;
    }
    const ms = Math.max(10, intervalSecs) * 1_000;
    const id = window.setInterval(() => void run("auto"), ms);
    return () => window.clearInterval(id);
  }, [opened, auto, active, intervalSecs, run]);

  // Sessões longas: volta logo enquanto houver trechos pendentes.
  useEffect(() => {
    if (!opened || !hasMore || loading) {
      return;
    }
    const id = window.setTimeout(() => void run("auto"), 1_200);
    return () => window.clearTimeout(id);
  }, [opened, hasMore, loading, run]);

  const refresh = useCallback(() => run("auto"), [run]);
  const regenerate = useCallback(() => run("manual"), [run]);

  return { summary, loading, error, hasMore, refresh, regenerate };
}
