/** Adaptador IPC de exportação de sessão (Markdown). */
import { save } from "@tauri-apps/plugin-dialog";
import { invokeSafe, isTauriRuntime } from "./client";

/** Markdown renderizado da sessão (usado para copiar e para salvar). */
export const exportSessionMarkdown = (
  id: string,
): Promise<string | undefined> =>
  invokeSafe<string>("session_export_markdown", { id });

/** Grava o Markdown da sessão no caminho escolhido; devolve o caminho final. */
export const saveSessionMarkdown = (
  id: string,
  path: string,
): Promise<string | undefined> =>
  invokeSafe<string>("session_save_markdown", { id, path });

/**
 * Abre o diálogo nativo de "salvar como" e devolve o caminho escolhido.
 *
 * `null` = o usuário cancelou. Fora do runtime Tauri (dev no navegador) devolve
 * um caminho simbólico só para a UI seguir o fluxo.
 */
export const pickMarkdownPath = async (
  suggestedName: string,
): Promise<string | null> => {
  if (!isTauriRuntime()) {
    return `${suggestedName}.md`;
  }
  const chosen = await save({
    defaultPath: suggestedName,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  return chosen ?? null;
};
