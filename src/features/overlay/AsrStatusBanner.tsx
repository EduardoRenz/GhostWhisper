import type { AsrStatusEvent } from "../../core/domain/asr";

interface AsrStatusProps {
  status: AsrStatusEvent | null;
}

/**
 * Aviso de preparação do motor de transcrição: só aparece enquanto o modelo
 * está sendo baixado/carregado ou quando falhou (some quando fica pronto).
 */
export function AsrStatusBanner({ status }: AsrStatusProps) {
  if (!status || status.phase === "ready") {
    return null;
  }
  const failed = status.phase === "failed";
  return (
    <p
      className={`asr-status${failed ? " asr-status--failed" : ""}`}
      role="status"
      data-interactive
    >
      <span className="asr-status__spinner" aria-hidden="true" />
      <span className="asr-status__text">{status.message}</span>
    </p>
  );
}