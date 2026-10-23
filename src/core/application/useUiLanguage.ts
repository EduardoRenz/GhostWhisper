/**
 * Mantém o store de idioma sincronizado com as settings da janela.
 *
 * Cada janela Tauri é um webview próprio: este hook garante que todas lêem o
 * mesmo `ui_language` no boot e reagem ao evento `settings_changed` disparado
 * pelo save (inclusive quando o idioma é trocado em Configurações).
 */

import { useEffect } from "react";
import { setUiLanguage } from "../l10n";
import { useSettings } from "./useSettings";

/** Aplica o idioma persistido sempre que as settings mudam. */
export function useUiLanguage(): void {
  const { settings } = useSettings();

  useEffect(() => {
    if (settings.ui_language) {
      setUiLanguage(settings.ui_language);
    }
  }, [settings.ui_language]);
}
