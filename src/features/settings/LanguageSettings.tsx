import {
  UI_LANGUAGES,
  UI_LANGUAGE_NAMES,
  normalizeUiLanguage,
  useTranslation,
  type UiLanguage,
} from "../../core/l10n";
import type { Settings } from "../../core/domain/settings";

interface LanguageSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/**
 * Card "Idioma da Interface" da aba Geral & HUD.
 *
 * O valor inicial vem do sistema **uma única vez** (resolvido no core); a
 * escolha feita aqui é persistida e nunca mais é trocada automaticamente.
 */
export function LanguageSettings({ settings, update }: LanguageSettingsProps) {
  const { t } = useTranslation();
  const current = normalizeUiLanguage(settings.ui_language) ?? "pt";

  return (
    <article className="settings__card settings__card--hud">
      <header className="settings__card-header">
        <h3 className="settings__card-title">{t("language_interface_language")}</h3>
      </header>
      <p className="settings__card-description">
        {t("language_sets_language_app_s_text_labels")}
      </p>
      <label className="settings__field">
        <span>{t("language_interface_language_field")}</span>
        <select
          value={current}
          onChange={(event) =>
            update("ui_language", event.target.value as UiLanguage)
          }
        >
          {UI_LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {UI_LANGUAGE_NAMES[language]}
            </option>
          ))}
        </select>
      </label>
      <p className="settings__hint">
        {t("language_initial_language_follows_system_default_first",
        )}
      </p>
    </article>
  );
}
