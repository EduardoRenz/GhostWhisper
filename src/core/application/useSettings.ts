import { useCallback, useEffect, useState } from "react";
import { getSettings } from "../../adapters/ipc/settings";
import { subscribe } from "../../adapters/ipc/events";
import { defaultSettings, type Settings } from "../domain/settings";
import { setUiLanguage } from "../l10n";

/** Carrega as configurações (para idiomas/flag de tradução). */
export function useSettings(): {
  settings: Settings;
  reload: () => Promise<void>;
} {
  const [settings, setSettings] = useState<Settings>(defaultSettings());

  const reload = useCallback(async () => {
    const loaded = await getSettings();
    if (loaded) {
      setSettings(loaded);
      // Cada janela é um webview: o idioma persistido alimenta o store local.
      setUiLanguage(loaded.ui_language);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Recarrega quando outra janela salva (ex.: duração do drill-hover — BR-036).
  useEffect(() => {
    let stop: (() => void) | undefined;
    void subscribe<null>("settings_changed", () => {
      void reload();
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, [reload]);

  return { settings, reload };
}
