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
 * janela aberta) e chega pelo evento `suggestion` em **duas fases** (BR-038):
 * a resposta curta (`quick`, do passe *fast-first*) e a completa (`full`,
 * Markdown com deltas do streaming). As duas ficam em blocos próprios — a
 * rápida **não** substitui a completa.
 *
 * A janela só pede uma sugestão ao abrir (para não começar vazia), permite
 * forçar, fixar e navegar no histórico.
 */
export function useSuggestions({
  sessionId,
  opened,
  modeId,
  extraContext,
}: UseSuggestionsOptions) {
  /** Sugestão completa em destaque (Markdown). */
  const [current, setCurrent] = useState<Suggestion | null>(null);
  /** Resposta curta da geração atual (fase `quick`). */
  const [quick, setQuick] = useState<Suggestion | null>(null);
  /** Histórico: sugestões completas, mais recente primeiro. */
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

  /** Aplica uma sugestão recebida, respeitando a fase (BR-038). */
  const applyIncoming = useCallback((incoming: Suggestion) => {
    if (incoming.phase === "quick") {
      // A resposta rápida é o **começo** da nova sugestão: a anterior sai do
      // topo (já está no histórico) e o cartão passa a mostrar a nova sendo
      // montada — sem confundir com a antiga.
      setCurrent((previous) =>
        previous && previous.at_ms !== incoming.at_ms ? null : previous,
      );
      setQuick(incoming);
      return;
    }
    // A completa só convive com a rápida da **mesma** geração.
    setQuick((previous) =>
      previous && previous.at_ms === incoming.at_ms ? previous : null,
    );
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
    setQuick(null);
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
    // A sugestão atual continua na tela até a nova ficar pronta.
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
          force,
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

  /**
   * Gera ao abrir a janela e ao trocar de sessão, para ela não começar vazia.
   *
   * Não forçamos: o core devolve a última sugestão quando não houve fala nova
   * (nenhuma chamada ao LLM — BR-039).
   *
   * **Trocar de agente não gera**: a próxima sugestão segue a regra normal
   * (fala nova + contagem regressiva) ou o botão “Sugerir agora”. O agente
   * escolhido vale para a próxima geração (as settings são salvas).
   */
  useEffect(() => {
    if (!opened || !sessionId) {
      return;
    }
    void generate(false);
  }, [opened, sessionId, generate]);

  const suggestNow = useCallback(() => generate(true), [generate]);

  /** Fixa uma sugestão (a atual, ou a de um item do histórico). */
  const pin = useCallback(
    async (target?: Suggestion | null) => {
      const suggestion = target ?? current;
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
    },
    [current],
  );

  return {
    /** Sugestão completa em destaque. */
    current,
    /** Resposta curta da geração atual (fase `quick`). */
    quick,
    /** Sugestões completas anteriores (mais recente primeiro). */
    history,
    /** `true` enquanto a IA gera/transmite uma sugestão (manual ou automática). */
    generating: loading || streamActive,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    pin,
  };
}
