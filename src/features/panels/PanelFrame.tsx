import type { ReactNode } from "react";
import { panelHide, panelStartDrag, type PanelLabel } from "../../adapters/ipc/panels";
import { WindowHeader } from "../../components/WindowHeader";
import { ResizeHandles } from "./ResizeHandles";

interface PanelFrameProps {
  label: PanelLabel;
  title: ReactNode;
  subtitle?: ReactNode;
  /** Selo/ícone do cabeçalho. */
  icon?: ReactNode;
  /** Conteúdo extra ao lado do título (ex.: pill LIVE). */
  titleExtra?: ReactNode;
  /** Botões do cabeçalho (antes do fechar). */
  actions?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

/**
 * Casca das janelas de painel: sem moldura, sempre no topo e com o conteúdo
 * rolando por dentro. O fechar esconde a janela.
 *
 * O header é o `WindowHeader` comum: grip de arraste no canto superior
 * esquerdo, selo + título e fechar à direita. As alças de `ResizeHandles`
 * ficam nas bordas da **janela**, fora do painel (que tem uma margem), para o
 * resize pegar no pixel mais externo.
 */
export function PanelFrame({
  label,
  title,
  subtitle,
  icon,
  titleExtra,
  actions,
  children,
  footer,
}: PanelFrameProps) {
  return (
    <div className="panel-shell">
      <ResizeHandles />
      <div className="panel" data-route={label}>
        <WindowHeader
          className="panel__header"
          title={title}
          subtitle={subtitle}
          icon={icon}
          titleExtra={titleExtra}
          actions={actions}
          onStartDrag={() => void panelStartDrag()}
          onClose={() => void panelHide(label)}
        />
        <div className="panel__body">{children}</div>
        {footer && <footer className="panel__footer">{footer}</footer>}
      </div>
    </div>
  );
}
