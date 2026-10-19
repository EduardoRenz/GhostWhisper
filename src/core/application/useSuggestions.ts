import { useCallback, useEffect, useRef, useState } from "react";
import {
  suggestionGenerate,
  suggestionPin,
} from "../../adapters/ipc/suggestions";
import { subscribe } from "../../adapters/ipc/events";
import type { Suggestion } from "../domain/assistant";

interface UseSuggestionsOptions {
  sessionId: string | null;
  opened: boolean;
  /** Agente selecionado. */
  modeId: string;
  /** Contexto colado pelo usuário (enviado como fonte suplementar). */
  extraContext: string;
}

const HISTORY_LIMIT = 8;

/**
 * Sugestão viva da janela.
 *
 * A geração **automática** vive no core (reage a fala nova, sem depender da
 * janela aberta) e chega pelo evento `suggestion` como texto em construção
 * (fast-first + deltas do streaming). A janela só pede uma sugestão ao abrir
 * (para não começar vazia), permite forçar, fixar e navegar no histórico.
 */
export function useSuggestions({
  sessionId,
  opened,
  modeId,
  extraContext,
}: UseSuggestionsOptions) {
  const [current, setCurrent] = useState<Suggestion | null>(null);
  const [history, setHistory] = useState<Suggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [skipped, setSkipped] = useState(false);
  const [pinned, setPinned] = useState<string | null>(null);
  /** Há streaming recente (deltas) — o pill some 1,5 s após o último. */
  const [streamActive, setStreamActive] = useState(false);

  const inFlight = useRef(false);
  const sessionRef = useRef<string | null>(sessionId);
  const modeRef = useRef(modeId);
  const extraRef = useRef(extraContext);
  const contextUntil = useRef<number | null>(null);
  const lastText = useRef<string | null>(null);
  const streamTimer = useRef<number | null>(null);

  /** Marca atividade de streaming (fast-first + deltas do evento `suggestion`). */
  const markStreaming = useCallback(() => {
    setStreamActive(true);
    if (streamTimer.current !== null) {
      window.clearTimeout(streamTimer.current);
    }
    streamTimer.current = window.setTimeout(() => {
      setStreamActive(false);
      streamTimer.current = null;
    }, 1_500);
  }, []);

  useEffect(
    () => () => {
      if (streamTimer.current !== null) {
        window.clearTimeout(streamTimer.current);
      }
    },
    [],
  );

  /** Aplica uma sugestão recebida (streaming) e atualiza o histórico por `at_ms`. */
  const applyIncoming = useCallback((incoming: Suggestion) => {
    setCurrent(incoming);
    lastText.current = incoming.text;
    setHistory((previous) => {
      const rest = previous.filter((item) => item.at_ms !== incoming.at_ms);
      return [incoming, ...rest].slice(0, HISTORY_LIMIT);
    });
  }, []);

  useEffect(() => {
    sessionRef.current = sessionId;
    contextUntil.current = null;
    lastText.current = null;
    setCurrent(null);
    setHistory([]);
    setError(null);
    setSkipped(false);
    setPinned(null);
    setStreamActive(false);
    if (streamTimer.current !== null) {
      window.clearTimeout(streamTimer.current);
      streamTimer.current = null;
    }
  }, [sessionId]);

  useEffect(() => {
    modeRef.current = modeId;
    contextUntil.current = null;
    lastText.current = null;
  }, [modeId]);

  useEffect(() => {
    extraRef.current = extraContext;
  }, [extraContext]);

  // Sugestões do core (fast-first + streaming), inclusive sem a janela aberta.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<Suggestion>("suggestion", (incoming) => {
      if (
        cancelled ||
        !incoming ||
        incoming.skipped ||
        !incoming.text.trim() ||
        incoming.session_id !== sessionRef.current
      ) {
        return;
      }
      setSkipped(false);
      applyIncoming(incoming);
      markStreaming();
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unlisten = stop;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [applyIncoming, markStreaming]);

  const generate = useCallback(
    async (force: boolean) => {
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
          fastFirst: true,
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
        applyIncoming(result);
      } catch (caught) {
        setError(String(caught));
      } finally {
        inFlight.current = false;
        setLoading(false);
      }
    },
    [applyIncoming],
  );

  // Gera ao abrir e ao trocar de sessão/agente, para a janela não começar vazia.
  useEffect(() => {
    if (opened && sessionId) {
      void generate(true);
    }
  }, [opened, sessionId, modeId, generate]);

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
    /** `true` enquanto a IA gera/transmite uma sugestão (manual ou automática). */
    generating: loading || streamActive,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    pin,
    select,
  };
}
