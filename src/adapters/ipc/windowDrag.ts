/**
 * Arraste/redimensionamento das janelas do app.
 *
 * O gesto é iniciado no `mousedown` por um comando Rust que usa a API do GTK
 * com timestamp X11 válido (`window_commands.rs`). O `start_dragging` padrão do
 * Tauri usa timestamp zero e é ignorado no primeiro press de uma janela
 * inativa (ISS-063).
 */
import { invokeSafe } from "./client";

/** Direções de redimensionamento aceitas pelo GTK/Tauri. */
export type WindowResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

/** Arrasta a janela atual; fora do Tauri é um no-op seguro. */
export async function startWindowDrag(): Promise<void> {
  try {
    await invokeSafe<void>("window_start_drag");
  } catch {
    // Wayland/compositor pode recusar; não derruba a janela.
  }
}

/** Redimensiona a janela atual; fora do Tauri é um no-op seguro. */
export async function startWindowResize(
  direction: WindowResizeDirection,
): Promise<void> {
  try {
    await invokeSafe<void>("window_start_resize", { direction });
  } catch {
    // Wayland nativo pode não suportar; não derruba a janela.
  }
}
