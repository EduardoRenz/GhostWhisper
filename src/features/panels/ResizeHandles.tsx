import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PointerEvent as ReactPointerEvent } from "react";
import { isTauriRuntime } from "../../adapters/ipc/client";

/** Direções aceitas pelo `startResizeDragging` do Tauri. */
type PanelResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

interface Handle {
  direction: PanelResizeDirection;
  position: string;
  label: string;
}

/** As 8 alças: 4 bordas + 4 cantos. */
const HANDLES: Handle[] = [
  { direction: "North", position: "n", label: "Redimensionar para cima" },
  { direction: "South", position: "s", label: "Redimensionar para baixo" },
  { direction: "West", position: "w", label: "Redimensionar para a esquerda" },
  { direction: "East", position: "e", label: "Redimensionar para a direita" },
  { direction: "NorthWest", position: "nw", label: "Redimensionar canto superior esquerdo" },
  { direction: "NorthEast", position: "ne", label: "Redimensionar canto superior direito" },
  { direction: "SouthWest", position: "sw", label: "Redimensionar canto inferior esquerdo" },
  { direction: "SouthEast", position: "se", label: "Redimensionar canto inferior direito" },
];

/**
 * Alças invisíveis de redimensionamento para janelas **sem moldura**.
 *
 * No Linux, uma janela sem decoração não tem bordas para o gerenciador de
 * janelas pegar; estas alças chamam `startResizeDragging` (X11/XWayland).
 */
export function ResizeHandles() {
  const onPointerDown =
    (direction: PanelResizeDirection) => (event: ReactPointerEvent) => {
      event.preventDefault();
      if (!isTauriRuntime()) {
        return;
      }
      void getCurrentWindow()
        .startResizeDragging(direction)
        .catch(() => {
          // Wayland nativo pode não suportar; não derruba a janela.
        });
    };

  return (
    <>
      {HANDLES.map(({ direction, position, label }) => (
        <span
          key={direction}
          className={`panel__resize panel__resize--${position}`}
          data-direction={direction}
          aria-hidden="true"
          title={label}
          onPointerDown={onPointerDown(direction)}
        />
      ))}
    </>
  );
}
