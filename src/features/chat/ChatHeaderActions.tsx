import { panelOpen } from "../../adapters/ipc/panels";
import { useTranslation } from "../../core/l10n";

/**
 * Ações do cabeçalho da Transcrição.
 *
 * O **Resumo** saiu daqui: ele já é a aba ao lado (o cabeçalho do painel mostra a
 * aba ativa). O **Sugerir** também saiu — a geração de sugestões no chat deixou de
 * fazer sentido e será revista na aba **Sugestões** (PRD `notch`, revisão de
 * 2026-10-19). Resta o atalho para a janela de Sugestões.
 */
export function ChatHeaderActions() {
  const { t } = useTranslation();
  return (
    <div className="chat__head-actions">
      <button
        type="button"
        className="overlay__button"
        data-interactive
        title={t("chat_opens_per_agent_suggestions_window")}
        onClick={() => void panelOpen("suggestions")}
      >
        {t("chat_suggestions")}
      </button>
    </div>
  );
}
