import { panelOpen } from "../../adapters/ipc/panels";

/**
 * Ações do cabeçalho da Transcrição.
 *
 * O **Resumo** saiu daqui: ele já é a aba ao lado (o cabeçalho do painel mostra a
 * aba ativa). O **Sugerir** também saiu — a geração de sugestões no chat deixou de
 * fazer sentido e será revista na aba **Sugestões** (PRD `notch`, revisão de
 * 2026-10-19). Resta o atalho para a janela de Sugestões.
 */
export function ChatHeaderActions() {
  return (
    <div className="chat__head-actions">
      <button
        type="button"
        className="overlay__button"
        data-interactive
        title="Abre a janela de sugestões por agente"
        onClick={() => void panelOpen("suggestions")}
      >
        Sugestões
      </button>
    </div>
  );
}
