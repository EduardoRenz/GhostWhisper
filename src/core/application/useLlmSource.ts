import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";

/**
 * Uso da fonte de IA na última chamada (BR-051).
 *
 * Vem do evento `llm_source_used`, emitido pelo roteador a cada chamada de
 * LLM. O `scope` diz **qual tarefa** pediu a chamada (`agent:meeting`,
 * `capability:Summary`, `default`) — é ele que permite mostrar o modelo certo
 * em cada tela.
 */
export interface LlmSourceUsage {
  /** Id da fonte que respondeu. */
  source_id: string;
  /** Nome de exibição da fonte ("DeepSeek"). */
  label: string;
  /** Provedor da fonte ("deepseek"). */
  provider: string;
  /** Modelo que respondeu ("deepseek-reasoner"). */
  model: string;
  /** Tarefa que pediu a chamada. */
  scope: string;
  /** A fonte respondeu em fallback? */
  is_fallback: boolean;
  /** Latência da chamada (ms). */
  latency_ms: number;
}

/**
 * Último uso de uma fonte de IA, opcionalmente filtrado por **escopo**.
 *
 * Sem escopo, devolve o último uso de qualquer tarefa. Com escopo, só o
 * primeiro evento do tipo `agent:<id>` / `capability:<nome>` / `default`
 * conta — assim o card da sugestão mostra o modelo da **sugestão**, e não o
 * do resumo.
 */
export function useLlmSource(scope?: string): LlmSourceUsage | null {
  const [usage, setUsage] = useState<LlmSourceUsage | null>(null);

  useEffect(() => {
    let stop: (() => void) | undefined;
    void subscribe<LlmSourceUsage>("llm_source_used", (event) => {
      if (scope && !matchesScope(event.scope, scope)) return;
      setUsage(event);
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, [scope]);

  return usage;
}

/** `true` quando o escopo do evento casa com o pedido. */
function matchesScope(eventScope: string, wanted: string): boolean {
  if (eventScope === wanted) return true;
  // `capability:Summary` (Rust) × `capability:summary` (frontend).
  return eventScope.toLowerCase() === wanted.toLowerCase();
}
