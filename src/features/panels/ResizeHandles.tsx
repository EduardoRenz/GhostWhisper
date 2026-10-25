import type { MouseEvent as ReactMouseEvent } from "react";
import {
  startWindowResize,
  type WindowResizeDirection,
} from "../../adapters/ipc/windowDrag";

interface Handle {
  direction: WindowResizeDirection;
  position: string;
}

/** As 8 alças: 4 bordas + 4 cantos. */
const HANDLES: Handle[] = [
  { direction: "North", position: "n" },
  { direction: "South", position: "s" },
  { direction: "West", position: "w" },
  { direction: "East", position: "e" },
  { direction: "NorthWest", position: "nw" },
  { direction: "NorthEast", position: "ne" },
  { direction: "SouthWest", position: "sw" },
  { direction: "SouthEast", position: "se" },
];

/**
 * Alças invisíveis de redimensionamento para janelas **sem moldura**.
 *
 * No Linux, uma janela sem decoração não tem bordas para o gerenciador de
 * janelas pegar; estas alças focam a janela e chamam `startResizeDragging`
 * (X11/XWayland), pelo mesmo motivo do arraste (ISS-063).
 */
export function ResizeHandles() {
  const onMouseDown =
    (direction: WindowResizeDirection) => (event: ReactMouseEvent) => {
      if (event.button !== 0) {
        return;
      }
      event.preventDefault();
      void startWindowResize(direction);
    };

  return (
    <>
      {HANDLES.map(({ direction, position }) => (
        <span
          key={direction}
          className={`panel__resize panel__resize--${position}`}
          data-direction={direction}
          aria-hidden="true"
          onMouseDown={onMouseDown(direction)}
        />
      ))}
    </>
  );
}
