import type { ReactNode } from "react";
import { panelHide, panelStartDrag, type PanelLabel } from "../../adapters/ipc/panels";
import { DragHandle } from "../../components/DragHandle";
import { ResizeHandles } from "./ResizeHandles";

interface PanelFrameProps {
  label: PanelLabel;
  title?: string;
  subtitle?: string;
  /** Cabeçalho próprio (substitui título + subtítulo). */
  heading?: ReactNode;
  /** Botões do cabeçalho (antes do fechar). */
  actions?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

/**
 * Casca das janelas de painel: sem moldura (arrasta pelo cabeçalho), sempre no
 * topo e com o conteúdo rolando por dentro. O fechar esconde a janela.
 *
 * As alças de `ResizeHandles` ficam nas bordas da **janela**, fora do painel
 * (que tem uma margem), para o resize pegar no pixel mais externo.
 */
export function PanelFrame({
  label,
  title,
  subtitle,
  heading,
  actions,
  children,
  footer,
}: PanelFrameProps) {
  return (
    <div className="panel-shell">
      <ResizeHandles />
      <div className="panel" data-route={label}>
        <header className="panel__header">
          <DragHandle
            className="panel__drag"
            onStartDrag={() => void panelStartDrag()}
          />
          <div className="panel__heading">
            {heading ?? (
              <>
                <span className="panel__title">{title}</span>
                {subtitle && <span className="panel__subtitle">{subtitle}</span>}
              </>
            )}
          </div>
          <div className="panel__actions">{actions}</div>
          <button
            type="button"
            className="panel__close"
            aria-label="Fechar"
            title="Fechar"
            onClick={() => void panelHide(label)}
          >
            ✕
          </button>
        </header>
        <div className="panel__body">{children}</div>
        {footer && <footer className="panel__footer">{footer}</footer>}
      </div>
    </div>
  );
}
