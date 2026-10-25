import type { MouseEvent as ReactMouseEvent } from "react";
import { GripIcon } from "./icons";
import { useTranslation } from "../core/l10n";

interface DragHandleProps {
  /** Inicia o arraste da janela. */
  onStartDrag: () => void;
  /** Rótulo acessível. */
  label?: string;
  /** Classe extra (posicionamento/tamanho no contêiner). */
  className?: string;
}

/**
 * Alça de arraste **padrão** das janelas: o grip de 6 pontos.
 *
 * Um único componente (e um único estilo base, `.drag-handle`) para o notch,
 * os painéis (Sugestões, Chat, Histórico) e qualquer janela nova — antes cada
 * tela repetia o próprio desenho.
 *
 * O gesto parte do `mousedown`: no Linux/GTK o WM só honra `start_dragging`
 * num evento de press do mouse, e o `mousedown` chega depois da ativação da
 * janela (ISS-063). Sem `title`, para o tooltip nativo não capturar o clique.
 */
export function DragHandle({
  onStartDrag,
  label,
  className,
}: DragHandleProps) {
  const { t } = useTranslation();
  const resolvedLabel = label ?? t("common_move_window");

  const handleMouseDown = (event: ReactMouseEvent<HTMLSpanElement>) => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    onStartDrag();
  };

  return (
    <span
      className={["drag-handle", className ?? ""].filter(Boolean).join(" ")}
      aria-label={resolvedLabel}
      onMouseDown={handleMouseDown}
    >
      <GripIcon size={16} />
    </span>
  );
}
