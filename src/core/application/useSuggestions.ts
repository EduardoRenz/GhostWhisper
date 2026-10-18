import { useCallback, useEffect, useRef, useState } from "react";
import {
  suggestionGenerate,
  suggestionPin,
} from "../../adapters/ipc/suggestions";
import type { Suggestion } from "../domain/assistant";

interface UseSuggestionsOptions {
  sessionId: string | null;
  opened: boolean;
  /** Agente selecionado. */
  modeId: string;
  auto: boolean;
  intervalSecs: number;
  active: boolean;
  /** Contexto colado pelo usuário (enviado como fonte suplementar). */
  extraContext: string;
}

const HISTORY_LIMIT = 8;

/**
 * Sugestão viva da janela: gera ao abrir/trocar de agente, atualiza no
 * intervalo (só com fala nova), permite forçar, fixar e navegar no histórico.
 */
export function useSuggestions({
  sessionId,
  opened,
  modeId,
  auto,
  intervalSecs,
  active,
  extraContext,
}: UseSuggestionsOptions) {
  const [current, setCurrent] = useState<Suggestion | null>(null);
  const [history, setHistory] = useState<Suggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [skipped, setSkipped] = useState(false);
  const [pinned, setPinned] = useState<string | null>(null);

  const inFlight = useRef(false);
  const sessionRef = useRef<string | null>(sessionId);
  const modeRef = useRef(modeId);
  const extraRef = useRef(extraContext);
  const contextUntil = useRef<number | null>(null);
  const lastText = useRef<string | null>(null);

  useEffect(() => {
    sessionRef.current = sessionId;
    contextUntil.current = null;
    lastText.current = null;
    setCurrent(null);
    setHistory([]);
    setError(null);
    setSkipped(false);
    setPinned(null);
  }, [sessionId]);

  useEffect(() => {
    modeRef.current = modeId;
    contextUntil.current = null;
    lastText.current = null;
  }, [modeId]);

  useEffect(() => {
    extraRef.current = extraContext;
  }, [extraContext]);

  const generate = useCallback(async (force: boolean) => {
    if (inFlight.current || !sessionRef.current) {
      return;
    }
    inFlight.current = true;
    setLoading(true);
    setError(null);
    try {
      const result = await suggestionGenerate({
        sessionId: sessionRef.current,
        modeId: modeRef.current,
        sinceMs: force ? null : contextUntil.current,
        lastSuggestion: lastText.current,
        extraContext: extraRef.current.trim() ? extraRef.current : null,
      });
      if (!result || result.session_id !== sessionRef.current) {
        return;
      }
      contextUntil.current = result.context_until_ms;
      if (result.skipped) {
        setSkipped(true);
        return;
      }
      setSkipped(false);
      setCurrent(result);
      lastText.current = result.text;
      setHistory((previous) => [result, ...previous].slice(0, HISTORY_LIMIT));
    } catch (caught) {
      setError(String(caught));
    } finally {
      inFlight.current = false;
      setLoading(false);
    }
  }, []);

  // Gera ao abrir, ao trocar de sessão e ao trocar de agente.
  useEffect(() => {
    if (opened && sessionId) {
      void generate(true);
    }
  }, [opened, sessionId, modeId, generate]);

  // Atualização periódica (só com a janela aberta e sessão gravando).
  useEffect(() => {
    if (!opened || !auto || !active) {
      return;
    }
    const ms = Math.max(10, intervalSecs) * 1_000;
    const id = window.setInterval(() => void generate(false), ms);
    return () => window.clearInterval(id);
  }, [opened, auto, active, intervalSecs, generate]);

  const suggestNow = useCallback(() => generate(true), [generate]);

  const pin = useCallback(async () => {
    const suggestion = current;
    if (!suggestion || !sessionRef.current) {
      return;
    }
    setError(null);
    try {
      const pinnedSuggestion = await suggestionPin({
        sessionId: sessionRef.current,
        modeId: suggestion.mode_id,
        text: suggestion.text,
        atMs: suggestion.at_ms,
      });
      if (pinnedSuggestion) {
        setPinned(pinnedSuggestion.message_id);
        window.setTimeout(() => setPinned(null), 2_000);
      }
    } catch (caught) {
      setError(String(caught));
    }
  }, [current]);

  const select = useCallback((suggestion: Suggestion) => {
    setCurrent(suggestion);
    lastText.current = suggestion.text;
    setSkipped(false);
  }, []);

  return {
    current,
    history,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    pin,
    select,
  };
}
