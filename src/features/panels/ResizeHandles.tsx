import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PointerEvent as ReactPointerEvent } from "react";
import { isTauriRuntime } from "../../adapters/ipc/client";
import { useTranslation, type TranslationKey } from "../../core/l10n";

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
  label: TranslationKey;
}

/** As 8 alças: 4 bordas + 4 cantos. */
const HANDLES: Handle[] = [
  { direction: "North", position: "n", label: "resize_up" },
  { direction: "South", position: "s", label: "resize_down" },
  { direction: "West", position: "w", label: "resize_left" },
  { direction: "East", position: "e", label: "resize_right" },
  { direction: "NorthWest", position: "nw", label: "resize_top_left" },
  { direction: "NorthEast", position: "ne", label: "resize_top_right" },
  { direction: "SouthWest", position: "sw", label: "resize_bottom_left" },
  { direction: "SouthEast", position: "se", label: "resize_bottom_right" },
];

/**
 * Alças invisíveis de redimensionamento para janelas **sem moldura**.
 *
 * No Linux, uma janela sem decoração não tem bordas para o gerenciador de
 * janelas pegar; estas alças chamam `startResizeDragging` (X11/XWayland).
 */
export function ResizeHandles() {
  const { t } = useTranslation();
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
          title={t(label)}
          onPointerDown={onPointerDown(direction)}
        />
      ))}
    </>
  );
}
