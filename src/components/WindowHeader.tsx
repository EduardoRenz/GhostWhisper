import type { ReactNode } from "react";
import { DragHandle } from "./DragHandle";
import { CloseIcon } from "./icons";

interface WindowHeaderProps {
  /** Título exibido no cabeçalho. */
  title: ReactNode;
  /** Linha de apoio abaixo do título (opcional). */
  subtitle?: ReactNode;
  /** Selo/ícone à esquerda do título. */
  icon?: ReactNode;
  /** Conteúdo extra ao lado do título (ex.: pill LIVE). */
  titleExtra?: ReactNode;
  /** Controles antes do fechar. */
  actions?: ReactNode;
  /** Conteúdo auxiliar antes do fechar (ex.: atalho `Esc`). */
  hint?: ReactNode;
  /** Inicia o arraste da janela. */
  onStartDrag: () => void;
  /** Fecha/esconde a janela. */
  onClose: () => void;
  closeLabel?: string;
  dragLabel?: string;
  /** `h1` para telas próprias (Configurações); `h2` nos painéis. */
  headingLevel?: "h1" | "h2";
  className?: string;
}

/**
 * Cabeçalho padrão das janelas do GhostWhisper.
 *
 * A ordem é a do mockup: **grip de arraste à esquerda**, selo do ícone, título
 * (e subtítulo), ações e fechar à direita. A alça fica no canto superior
 * esquerdo em todas as janelas; o restante do header não arrasta, para não
 * roubar o clique dos controles.
 */
export function WindowHeader({
  title,
  subtitle,
  icon,
  titleExtra,
  actions,
  hint,
  onStartDrag,
  onClose,
  closeLabel = "Fechar",
  dragLabel = "Mover janela",
  headingLevel = "h2",
  className,
}: WindowHeaderProps) {
  const Heading = headingLevel;

  return (
    <header
      className={["window-header", className ?? ""].filter(Boolean).join(" ")}
    >
      <DragHandle
        className="window-header__drag"
        label={dragLabel}
        onStartDrag={onStartDrag}
      />

      {icon ? (
        <span className="window-header__icon" aria-hidden="true">
          {icon}
        </span>
      ) : null}

      <div className="window-header__text">
        <div className="window-header__title-row">
          <Heading className="window-header__title">{title}</Heading>
          {titleExtra}
        </div>
        {subtitle ? (
          <span className="window-header__subtitle">{subtitle}</span>
        ) : null}
      </div>

      {actions ? <div className="window-header__actions">{actions}</div> : null}
      {hint}

      <button
        type="button"
        className="window-header__close"
        aria-label={closeLabel}
        title={closeLabel}
        onClick={onClose}
      >
        <CloseIcon size={16} />
      </button>
    </header>
  );
}
