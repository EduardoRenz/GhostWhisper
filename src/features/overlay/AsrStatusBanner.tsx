import type { AsrStatusEvent } from "../../core/domain/asr";
import { useTranslation } from "../../core/l10n";

interface AsrStatusProps {
  status: AsrStatusEvent | null;
}

/**
 * Aviso de preparação do motor de transcrição: só aparece enquanto o modelo
 * está sendo baixado/carregado ou quando falhou (some quando fica pronto).
 */
export function AsrStatusBanner({ status }: AsrStatusProps) {
  const { t } = useTranslation();
  if (!status || status.phase === "ready") {
    return null;
  }
  const failed = status.phase === "failed";
  const hint = /\(pode baixar (.+?)\)/.exec(status.message)?.[1];
  const text = failed
    ? `${t("asr_failed")} ${status.message}`
    : hint
      ? t("asr_preparing_model", { model: status.model, hint })
      : t("asr_preparing_model_no_hint", { model: status.model });
  return (
    <p
      className={`asr-status${failed ? " asr-status--failed" : ""}`}
      role="status"
      data-interactive
    >
      <span className="asr-status__spinner" aria-hidden="true" />
      <span className="asr-status__text">{text}</span>
    </p>
  );
}