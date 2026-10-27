import { useCallback, useEffect, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { logsPath, logsRecent } from "../../adapters/ipc/logs";
import { useTranslation } from "../../core/l10n";

/** Consulta local do `ghostwhisper.log`; nada é enviado para fora. */
export function LogsSettings() {
  const { t } = useTranslation();
  const [content, setContent] = useState("");
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [file, recent] = await Promise.all([logsPath(), logsRecent(300)]);
      setPath(file ?? "");
      setContent(recent ?? "");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const openFolder = () => {
    if (path) {
      void revealItemInDir(path);
    }
  };

  return (
    <section className="settings__card">
      <header className="settings__card-header">
        <h2 className="settings__card-title">{t("settings_logs_title")}</h2>
        <span className="settings__card-note">{t("settings_logs_note")}</span>
      </header>
      <div className="settings__logs-actions">
        <button
          type="button"
          className="settings__button settings__button--small"
          onClick={() => void load()}
          disabled={loading}
        >
          {t("settings_logs_refresh")}
        </button>
        <button
          type="button"
          className="settings__button settings__button--small"
          onClick={openFolder}
          disabled={!path}
        >
          {t("settings_logs_open_folder")}
        </button>
      </div>
      {error && (
        <p className="settings__hint settings__hint--error">
          {t("settings_logs_error", { error })}
        </p>
      )}
      <pre className="settings__logs">
        {content || t("settings_logs_empty")}
      </pre>
    </section>
  );
}
