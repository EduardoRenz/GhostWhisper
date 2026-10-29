import { useCallback, useEffect, useRef, useState } from "react";
import {
  suggestionGenerate,
  suggestionLast,
  suggestionPin,
} from "../../adapters/ipc/suggestions";
import { subscribe } from "../../adapters/ipc/events";
import type {
  ManualSuggestionContext,
  Suggestion,
  SuggestionTrigger,
} from "../domain/assistant";

interface UseSuggestionsOptions {
  sessionId: string | null;
  /** Agente selecionado. */
  modeId: string;
  /** Contexto colado pelo usuário (enviado como fonte suplementar). */
  extraContext: string;
  /** Mensagem selecionada na Transcrição (BR-041). */
  manualContext?: ManualSuggestionContext | null;
}

const HISTORY_LIMIT = 8;

/**
 * Sugestão viva da janela.
 *
 * A geração **automática** vive no core e reage a fala nova **com a janela
 * aberta** (BR-028 revisada); chega pelo evento `suggestion` em **duas fases**
 * (BR-038): a resposta curta (`quick`, do passe *fast-first*) e a completa
 * (`full`, Markdown com deltas do streaming).
 *
 * A janela **não gera ao abrir**: hidrata com a última sugestão em cache
 * (`suggestion_last`) e deixa o automático/“Sugerir agora” gerar (BR-042).
 *
 * Com **contexto manual ativo**, a automática vai para o histórico e não
 * substitui a manual; a manual é disparada pelo usuário (`generateManual`).
 */
export function useSuggestions({
  sessionId,
  modeId,
  extraContext,
  manualContext,
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
  /** Origem do stream em andamento (rótulo do pill). */
  const [streamingTrigger, setStreamingTrigger] =
    useState<SuggestionTrigger | null>(null);
  /**
   * `generation_id` cuja **resposta completa** está pendente.
   *
   * A rápida chega primeiro; a completa pode demorar (principalmente no
   * automático). Este estado garante o aviso “Gerando resposta completa…”
   * durante a espera, sem depender do `streamActive` (que expira em 1,5 s).
   */
  const [pendingFull, setPendingFull] = useState<string | null>(null);
  const pendingTimer = useRef<number | null>(null);

  const inFlight = useRef(false);
  const eventSeen = useRef(false);
  const sessionRef = useRef<string | null>(sessionId);
  const modeRef = useRef(modeId);
  const extraRef = useRef(extraContext);
  const manualRef = useRef<ManualSuggestionContext | null>(manualContext ?? null);
  const contextUntil = useRef<number | null>(null);
  const lastText = useRef<string | null>(null);
  const streamTimer = useRef<number | null>(null);
  /**
   * Gerações cuja resposta **completa** já chegou.
   *
   * Serve para reconhecer uma `quick` atrasada da **mesma** geração (que ainda
   * deve preencher o bloco "resposta rápida") e descartar uma `quick` de uma
   * geração que já foi substituída por outra completa.
   */
  const fullGenerations = useRef<Set<string>>(new Set());
  /** Espelho síncrono de `current` para os guardas do evento. */
  const currentRef = useRef<Suggestion | null>(null);

  // A ref fica atualizada **antes** dos efeitos de sessão/subscribe, para o
  // filtro da automática enxergar o contexto manual já no primeiro evento.
  useEffect(() => {
    manualRef.current = manualContext ?? null;
  }, [manualContext]);

  /** Marca atividade de streaming (fast-first + deltas do evento `suggestion`). */
  const markStreaming = useCallback((trigger: SuggestionTrigger) => {
    setStreamActive(true);
    setStreamingTrigger(trigger);
    if (streamTimer.current !== null) {
      window.clearTimeout(streamTimer.current);
    }
    streamTimer.current = window.setTimeout(() => {
      setStreamActive(false);
      setStreamingTrigger(null);
      streamTimer.current = null;
    }, 1_500);
  }, []);

  useEffect(
    () => () => {
      if (streamTimer.current !== null) {
        window.clearTimeout(streamTimer.current);
      }
      if (pendingTimer.current !== null) {
        window.clearTimeout(pendingTimer.current);
      }
    },
    [],
  );

  /** Marca a completa como pendente (com um teto de segurança). */
  const markFullPending = useCallback((generationId: string) => {
    setPendingFull(generationId);
    if (pendingTimer.current !== null) {
      window.clearTimeout(pendingTimer.current);
    }
    // Se a completa falhar (sem evento), o aviso não fica preso para sempre.
    pendingTimer.current = window.setTimeout(() => {
      setPendingFull(null);
      pendingTimer.current = null;
    }, 45_000);
  }, []);

  /** A completa começou (ou falhou): o aviso some. */
  const clearFullPending = useCallback(() => {
    setPendingFull(null);
    if (pendingTimer.current !== null) {
      window.clearTimeout(pendingTimer.current);
      pendingTimer.current = null;
    }
  }, []);

  /** Adiciona uma sugestão completa ao histórico (dedupe por geração). */
  const pushHistory = useCallback((incoming: Suggestion) => {
    setHistory((previous) => {
      const rest = previous.filter(
        (item) => item.generation_id !== incoming.generation_id,
      );
      // Conteúdo idêntico à sugestão anterior não vira um segundo item no
      // histórico (proteção contra duplicatas): fica só a mais recente.
      if (rest[0] && rest[0].text.trim() === incoming.text.trim()) {
        return [incoming, ...rest.slice(1)].slice(0, HISTORY_LIMIT);
      }
      return [incoming, ...rest].slice(0, HISTORY_LIMIT);
    });
  }, []);

  /** Aplica uma sugestão recebida, respeitando a fase e a prioridade (BR-042). */
  const applyIncoming = useCallback(
    (incoming: Suggestion) => {
      if (incoming.phase === "full") {
        fullGenerations.current.add(incoming.generation_id);
      } else {
        const current = currentRef.current;
        const sameGeneration = current?.generation_id === incoming.generation_id;
        // Rápida de uma geração que já foi substituída por outra completa.
        if (fullGenerations.current.has(incoming.generation_id) && !sameGeneration) {
          return;
        }
        // Rápida mais antiga que a sugestão atual (modelos rápidos podem
        // terminar a completa antes do rascunho). `at_ms` é monotônico.
        if (current && !sameGeneration && incoming.at_ms < current.at_ms) {
          return;
        }
      }

      // Com contexto manual ativo, a automática não assume o cartão: a
      // completa vai para o histórico e a rápida é ignorada (efêmera).
      if (incoming.trigger === "auto" && manualRef.current) {
        if (incoming.phase === "full") {
          pushHistory(incoming);
        }
        return;
      }

      if (incoming.phase === "quick") {
        const sameGeneration =
          currentRef.current?.generation_id === incoming.generation_id;
        // A resposta rápida é o **começo** da nova sugestão: a anterior sai do
        // topo (já está no histórico) e o cartão passa a mostrar a nova sendo
        // montada. Se a completa da mesma geração já chegou, ela permanece.
        if (!sameGeneration) {
          currentRef.current = null;
          setCurrent(null);
        }
        setQuick(incoming);
        if (!fullGenerations.current.has(incoming.generation_id)) {
          markFullPending(incoming.generation_id);
        }
        return;
      }

      // A completa chegou: o aviso de “gerando” some.
      clearFullPending();
      // A completa só convive com a rápida da **mesma** geração.
      setQuick((previous) =>
        previous && previous.generation_id === incoming.generation_id
          ? previous
          : null,
      );
      currentRef.current = incoming;
      setCurrent(incoming);
      lastText.current = incoming.text;
      pushHistory(incoming);
    },
    [clearFullPending, markFullPending, pushHistory],
  );

  useEffect(() => {
    sessionRef.current = sessionId;
    eventSeen.current = false;
    contextUntil.current = null;
    lastText.current = null;
    currentRef.current = null;
    setCurrent(null);
    setQuick(null);
    setHistory([]);
    setError(null);
    setSkipped(false);
    setPinned(null);
    setStreamActive(false);
    setStreamingTrigger(null);
    setPendingFull(null);
    fullGenerations.current.clear();
    if (streamTimer.current !== null) {
      window.clearTimeout(streamTimer.current);
      streamTimer.current = null;
    }
    if (pendingTimer.current !== null) {
      window.clearTimeout(pendingTimer.current);
      pendingTimer.current = null;
    }

    if (!sessionId) {
      return;
    }

    // Hidrata com a última sugestão lembrada (cache do core, sem LLM).
    let cancelled = false;
    void suggestionLast(sessionId).then((last) => {
      if (
        cancelled ||
        !last ||
        sessionRef.current !== sessionId ||
        eventSeen.current ||
        inFlight.current
      ) {
        return;
      }
      applyIncoming(last);
    });
    return () => {
      cancelled = true;
    };
  }, [sessionId, applyIncoming]);

  useEffect(() => {
    modeRef.current = modeId;
    contextUntil.current = null;
    lastText.current = null;
    // A sugestão atual continua na tela até a nova ficar pronta.
  }, [modeId]);

  useEffect(() => {
    extraRef.current = extraContext;
  }, [extraContext]);

  // Sugestões do core (fast-first + streaming).
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
      eventSeen.current = true;
      setSkipped(false);
      applyIncoming(incoming);
      markStreaming(incoming.trigger);
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
    async (force: boolean, manual?: ManualSuggestionContext | null) => {
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
          manualContext: manual ?? null,
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
        clearFullPending();
        setError(String(caught));
      } finally {
        inFlight.current = false;
        setLoading(false);
      }
    },
    [applyIncoming, clearFullPending],
  );

  const suggestNow = useCallback(() => generate(true, null), [generate]);

  /** Gera a sugestão manual a partir da mensagem selecionada (BR-041). */
  const generateManual = useCallback(() => {
    const selected = manualRef.current;
    if (!selected || selected.session_id !== sessionRef.current) {
      return;
    }
    return generate(true, selected);
  }, [generate]);

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
    /** `true` enquanto a completa da geração atual ainda não chegou. */
    awaitingFull:
      pendingFull !== null && quick?.generation_id === pendingFull,
    /** Sugestões completas anteriores (mais recente primeiro). */
    history,
    /** `true` enquanto a IA gera/transmite uma sugestão. */
    generating: loading || streamActive,
    /** Origem do stream atual (`auto`/`now`/`manual`), quando houver. */
    streamingTrigger,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    generateManual,
    pin,
  };
}
