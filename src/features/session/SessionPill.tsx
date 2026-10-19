import { useEffect, useState } from "react";
import { formatClockLong } from "../../core/domain/transcript";

interface SessionPillProps {
  /** Início da sessão (relógio de parede, ms); `null` = sem sessão. */
  startedAtMs: number | null;
  /** Confirmação de encerramento em exibição. */
  confirming: boolean;
  busy: boolean;
  /** Pede o encerramento (abre a confirmação). */
  onRequestStop: () => void;
  /** Desiste do encerramento. */
  onCancelStop: () => void;
  /** Confirma o encerramento. */
  onStop: () => void;
}

/**
 * Pílula de sessão (paridade com o Perssua): tempo decorrido em `mm:ss` e, ao
 * clicar, a confirmação inline **"Encerrar sessão? Sim / Não"** — encerrar nunca
 * acontece por um clique solto (BR-019, `chat-ui.md` §3).
 *
 * O tempo vem de `started_at_ms` (início da sessão, no relógio de parede): pausar
 * e retomar não zeram nem reiniciam a contagem.
 */
export function SessionPill({
  startedAtMs,
  confirming,
  busy,
  onRequestStop,
  onCancelStop,
  onStop,
}: SessionPillProps) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (startedAtMs === null) {
      return;
    }
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [startedAtMs]);

  if (startedAtMs === null) {
    return null;
  }

  if (confirming) {
    return (
      <span className="session-pill session-pill--confirming" data-interactive>
        <span className="session-pill__text">Encerrar sessão?</span>
        <button
          type="button"
          className="session-pill__button"
          disabled={busy}
          onClick={onStop}
        >
          Sim
        </button>
        <button
          type="button"
          className="session-pill__button"
          disabled={busy}
          onClick={onCancelStop}
        >
          Não
        </button>
      </span>
    );
  }

  return (
    <button
      type="button"
      className="session-pill"
      data-interactive
      title="Tempo de sessão — clique para encerrar"
      aria-label={`Sessão em andamento há ${formatClockLong(now - startedAtMs)}; clique para encerrar`}
      onClick={onRequestStop}
    >
      <span className="session-pill__dot" aria-hidden="true" />
      <span className="session-pill__timer">
        {formatClockLong(now - startedAtMs)}
      </span>
    </button>
  );
}
