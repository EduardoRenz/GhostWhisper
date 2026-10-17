/** Adaptador IPC de atalhos globais. */
import { invokeSafe } from "./client";
import type {
  ShortcutBinding,
  ShortcutConflict,
  ShortcutInfo,
} from "../../core/domain/shortcuts";

/** Atalhos configurados. */
export const getShortcuts = (): Promise<ShortcutBinding[] | undefined> =>
  invokeSafe<ShortcutBinding[]>("shortcuts_get");

/** Atalhos efetivamente registrados no sistema. */
export const getActiveShortcuts = (): Promise<ShortcutBinding[] | undefined> =>
  invokeSafe<ShortcutBinding[]>("shortcuts_active");

/** Catálogo de ações e padrões. */
export const getShortcutCatalog = (): Promise<ShortcutInfo[] | undefined> =>
  invokeSafe<ShortcutInfo[]>("shortcuts_catalog");

/** Persiste e reaplica os atalhos; devolve conflitos. */
export const setShortcuts = (
  bindings: ShortcutBinding[],
): Promise<ShortcutConflict[] | undefined> =>
  invokeSafe<ShortcutConflict[]>("shortcuts_set", { bindings });
