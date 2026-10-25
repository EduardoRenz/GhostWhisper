import type { MouseEvent, ReactNode } from "react";
import { DragHandle } from "./DragHandle";
import { CloseIcon } from "./icons";
import { useTranslation } from "../core/l10n";
import { shouldStartHeaderDrag } from "./windowHeaderDrag";

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
  /** Inicia o arraste da janela (header inteiro e alça). */
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
 * (e subtítulo), ações e fechar à direita. O header **inteiro** arrasta a
 * janela (BR-064); a alça fica visível como indicação. Controles como botões,
 * abas, links e campos não iniciam o arraste, para preservar o clique.
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
  closeLabel,
  dragLabel,
  headingLevel = "h2",
  className,
}: WindowHeaderProps) {
  const { t } = useTranslation();
  const Heading = headingLevel;
  const resolvedCloseLabel = closeLabel ?? t("common_close");
  const resolvedDragLabel = dragLabel ?? t("common_move_window");

  const handleMouseDown = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0 || !shouldStartHeaderDrag(event.target)) {
      return;
    }
    // Evita selecionar o título; o WM só aceita o gesto no press do mouse
    // (`mousedown`), que chega depois da ativação da janela (ISS-063).
    event.preventDefault();
    onStartDrag();
  };

  return (
    <header
      className={["window-header", className ?? ""].filter(Boolean).join(" ")}
      onMouseDown={handleMouseDown}
    >
      <DragHandle
        className="window-header__drag"
        label={resolvedDragLabel}
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
        aria-label={resolvedCloseLabel}
        title={resolvedCloseLabel}
        onClick={onClose}
      >
        <CloseIcon size={16} />
      </button>
    </header>
  );
}
