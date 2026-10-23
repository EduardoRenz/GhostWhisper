import { GripIcon } from "./icons";
import { useTranslation } from "../core/l10n";

interface DragHandleProps {
  /** Inicia o arraste da janela. */
  onStartDrag: () => void;
  /** Rótulo acessível e tooltip. */
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
 */
export function DragHandle({
  onStartDrag,
  label,
  className,
}: DragHandleProps) {
  const { t } = useTranslation();
  const resolvedLabel = label ?? t("common_move_window");
  return (
    <span
      className={["drag-handle", className ?? ""].filter(Boolean).join(" ")}
      title={resolvedLabel}
      aria-label={resolvedLabel}
      onPointerDown={(event) => {
        event.preventDefault();
        onStartDrag();
      }}
    >
      <GripIcon size={16} />
    </span>
  );
}
