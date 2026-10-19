import { useCallback, useRef, useState } from "react";
import { llmTranslate } from "../../adapters/ipc/llm";

/** Traduções por id de segmento. */
export type TranslationMap = Record<string, string>;

/** Texto traduzível (o que a bolha conhece: id + texto). */
export interface TranslatableText {
  id: string;
  text: string;
}

/**
 * Tradução **sob demanda**: quem pede é a bolha (botão no canto), não um
 * processo automático.
 *
 * A fila é serial (uma chamada por vez) para não estourar rate limit do provider;
 * o resultado fica em cache por id e é reusado no toggle original/tradução.
 */
export function useTranslation(): {
  translations: TranslationMap;
  pending: Record<string, boolean>;
  error: string | null;
  translate: (input: TranslatableText) => Promise<string | null>;
  reset: () => void;
} {
  const [translations, setTranslations] = useState<TranslationMap>({});
  const [pending, setPending] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const cache = useRef<TranslationMap>({});
  const queue = useRef<TranslatableText[]>([]);
  const running = useRef(false);

  const drain = useCallback(async () => {
    if (running.current) {
      return;
    }
    running.current = true;
    try {
      while (queue.current.length > 0) {
        const item = queue.current.shift();
        if (!item) {
          continue;
        }
        try {
          const translated = await llmTranslate(item.text);
          if (translated !== undefined && translated.trim().length > 0) {
            cache.current = { ...cache.current, [item.id]: translated };
            setTranslations((prev) => ({ ...prev, [item.id]: translated }));
          }
        } catch (caught) {
          setError(String(caught));
        } finally {
          setPending((prev) => {
            const next = { ...prev };
            delete next[item.id];
            return next;
          });
        }
      }
    } finally {
      running.current = false;
    }
  }, []);

  const translate = useCallback(
    async (input: TranslatableText): Promise<string | null> => {
      const cached = cache.current[input.id];
      if (cached !== undefined) {
        return cached;
      }
      if (!queue.current.some((item) => item.id === input.id)) {
        queue.current.push(input);
        setPending((prev) => ({ ...prev, [input.id]: true }));
      }
      await drain();
      return cache.current[input.id] ?? null;
    },
    [drain],
  );

  const reset = useCallback(() => {
    cache.current = {};
    queue.current = [];
    setTranslations({});
    setPending({});
    setError(null);
  }, []);

  return { translations, pending, error, translate, reset };
}
