import type { ReactNode } from "react";
import { panelHide, panelStartDrag, type PanelLabel } from "../../adapters/ipc/panels";

interface PanelFrameProps {
  label: PanelLabel;
  title: string;
  subtitle?: string;
  /** Botões do cabeçalho (antes do fechar). */
  actions?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

/**
 * Casca das janelas de painel: sem moldura (arrasta pelo cabeçalho), sempre no
 * topo e com o conteúdo rolando por dentro. O fechar esconde a janela.
 */
export function PanelFrame({
  label,
  title,
  subtitle,
  actions,
  children,
  footer,
}: PanelFrameProps) {
  return (
    <div className="panel" data-route={label}>
      <header className="panel__header">
        <span
          className="panel__drag"
          title="Mover janela"
          aria-label="Mover janela"
          onPointerDown={(event) => {
            event.preventDefault();
            void panelStartDrag();
          }}
        >
          ⋮⋮
        </span>
        <div className="panel__heading">
          <span className="panel__title">{title}</span>
          {subtitle && <span className="panel__subtitle">{subtitle}</span>}
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
  );
}
