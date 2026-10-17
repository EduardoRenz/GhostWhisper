import { useCallback, useEffect, useState } from "react";
import { getSettings } from "../../adapters/ipc/settings";
import { defaultSettings, type Settings } from "../domain/settings";

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
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  return { settings, reload };
}
