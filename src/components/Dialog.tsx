/**
 * Modal **in-page** do design system (BR-077).
 *
 * O app não tinha modal: as janelas de "Sessão Finalizada" e do seletor de
 * gravação são janelas Tauri separadas. Este primitivo cobre o caso de um
 * diálogo dentro de uma janela já aberta (o editor de agente das
 * Configurações):
 *
 * - `role="dialog"` + `aria-modal` + `aria-labelledby`;
 * - fecha por `Esc`, pelo backdrop e pelo `✕`;
 * - prende o foco dentro do cartão e o devolve ao final;
 * - trava o scroll da página enquanto estiver aberto.
 */

import { useEffect, useId, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { CloseIcon } from "./icons";
import { useTranslation } from "../core/l10n";

export interface DialogProps {
  /** Abre/fecha o modal (controlado). */
  open: boolean;
  /** Título (rotula o `aria-labelledby`). */
  title: string;
  /** Linha de apoio sob o título. */
  subtitle?: string;
  /** Fecha (botão `✕`, backdrop e `Esc`). */
  onClose: () => void;
  /** Conteúdo (o formulário). */
  children: ReactNode;
  /** Rodapé com as ações. */
  footer?: ReactNode;
}

/** Seletor dos elementos que recebem foco dentro do modal. */
const FOCUSABLE = [
  "button:not([disabled])",
  "[href]",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

/** Modal padrão do produto. */
export function Dialog({
  open,
  title,
  subtitle,
  onClose,
  children,
  footer,
}: DialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const cardRef = useRef<HTMLDivElement | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) {
      return;
    }
    const previous = document.activeElement as HTMLElement | null;
    const card = cardRef.current;
    const focusables = () =>
      Array.from(card?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []);

    // O foco entra no primeiro campo do formulário (não no `✕` do cabeçalho).
    const body = card?.querySelector<HTMLElement>(".dialog__body");
    const firstField = body?.querySelector<HTMLElement>(FOCUSABLE);
    (firstField ?? focusables()[0] ?? card)?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        // Não deixa o `Esc` chegar na janela (que fecharia as Configurações).
        event.preventDefault();
        event.stopPropagation();
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }
      const items = focusables();
      if (items.length === 0) {
        event.preventDefault();
        return;
      }
      const index = items.indexOf(document.activeElement as HTMLElement);
      if (event.shiftKey && index <= 0) {
        event.preventDefault();
        items[items.length - 1]?.focus();
      } else if (!event.shiftKey && index === items.length - 1) {
        event.preventDefault();
        items[0]?.focus();
      }
    };

    // Captura no `document`: roda antes do listener de `Esc` da janela.
    document.addEventListener("keydown", onKey, true);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKey, true);
      document.body.style.overflow = previousOverflow;
      previous?.focus?.();
    };
  }, [open]);

  if (!open) {
    return null;
  }

  const content = (
    <div className="dialog">
      <div
        className="dialog__backdrop"
        aria-hidden="true"
        onClick={() => onCloseRef.current()}
      />
      <div
        ref={cardRef}
        className="dialog__card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <header className="dialog__header">
          <div className="dialog__heading">
            <h2 className="dialog__title" id={titleId}>
              {title}
            </h2>
            {subtitle ? <p className="dialog__subtitle">{subtitle}</p> : null}
          </div>
          <button
            type="button"
            className="icon-button"
            aria-label={t("common_close")}
            title={t("common_close")}
            onClick={() => onCloseRef.current()}
          >
            <CloseIcon size={16} />
          </button>
        </header>
        <div className="dialog__body">{children}</div>
        {footer ? <footer className="dialog__footer">{footer}</footer> : null}
      </div>
    </div>
  );

  // Portal no `body`: o modal não sofre com o scroll/overflow da aba que o
  // abriu (as Configurações rolam no `.settings__body`).
  if (typeof document === "undefined") {
    return content;
  }
  return createPortal(content, document.body);
}
