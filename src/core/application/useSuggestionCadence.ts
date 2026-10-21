import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";

/**
 * Cadência do auto-sugerir (BR-039).
 *
 * Publicada pelo **core** (o agendador é a fonte da verdade): é o timer que já
 * existe, exposto como feedback visual. A UI **não** recalcula a cadência.
 */
export interface SuggestionCadence {
  /** O automático está ligado (configuração do usuário). */
  enabled: boolean;
  /** O automático está rodando (ligado e com sessão aberta). */
  running: boolean;
  /** Milissegundos até a próxima janela automática. */
  remaining_ms: number;
  /** A janela venceu, mas não há fala nova para sugerir. */
  awaiting_speech: boolean;
  /** Duração total do ciclo (desenha o anel de progresso). */
  window_ms: number;
  /** A geração automática está em andamento (feedback imediato). */
  generating: boolean;
}

/** Estado parado (nenhum evento recebido ainda). */
const IDLE: SuggestionCadence = {
  enabled: false,
  running: false,
  remaining_ms: 0,
  awaiting_speech: false,
  window_ms: 1,
  generating: false,
};

/**
 * Assina a cadência publicada pelo core.
 *
 * A assinatura é **incondicional**: o hook só existe na janela de Sugestões e
 * o core publica o timer sempre. Depender de “a janela está aberta” já deixou
 * o controle parado em 0 s quando o estado de visibilidade não chegou.
 */
export function useSuggestionCadence(): SuggestionCadence {
  const [cadence, setCadence] = useState<SuggestionCadence>(IDLE);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void subscribe<SuggestionCadence>("suggestion_cadence", (incoming) => {
      if (cancelled || !incoming) {
        return;
      }
      setCadence(incoming);
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
  }, []);

  return cadence;
}
