import { useCallback, useEffect, useRef, useState } from "react";
import { llmTranslate } from "../../adapters/ipc/llm";
import type { TranscriptSegment } from "../domain/transcript";

/** Traduções por id de segmento. */
export type TranslationMap = Record<string, string>;

/**
 * Traduz os segmentos finalizados em fila (uma por vez), quando habilitado.
 *
 * A fila serial evita estourar rate limit do provider; o resultado é indexado
 * pelo id do segmento para exibição lado a lado.
 */
export function useTranslation(
  segments: TranscriptSegment[],
  enabled: boolean,
): {
  translations: TranslationMap;
  error: string | null;
  reset: () => void;
} {
  const [translations, setTranslations] = useState<TranslationMap>({});
  const [error, setError] = useState<string | null>(null);
  const pending = useRef<Set<string>>(new Set());
  const queue = useRef<TranscriptSegment[]>([]);
  const running = useRef(false);

  const drain = useCallback(async () => {
    if (running.current) {
      return;
    }
    running.current = true;
    try {
      while (queue.current.length > 0) {
        const segment = queue.current.shift();
        if (!segment) {
          continue;
        }
        try {
          const translated = await llmTranslate(segment.text);
          if (translated !== undefined) {
            setTranslations((prev) => ({ ...prev, [segment.id]: translated }));
          }
        } catch (caught) {
          setError(String(caught));
        } finally {
          pending.current.delete(segment.id);
        }
      }
    } finally {
      running.current = false;
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      return;
    }
    for (const segment of segments) {
      if (!segment.final || translations[segment.id] || pending.current.has(segment.id)) {
        continue;
      }
      // Final de texto vazio é retratação de prévia (ISS-021), não fala: não gasta
      // uma chamada de tradução.
      if (segment.text.trim().length === 0) {
        continue;
      }
      pending.current.add(segment.id);
      queue.current.push(segment);
    }
    void drain();
  }, [segments, enabled, translations, drain]);

  const reset = useCallback(() => {
    setTranslations({});
    setError(null);
    pending.current.clear();
    queue.current = [];
  }, []);

  return { translations, error, reset };
}
