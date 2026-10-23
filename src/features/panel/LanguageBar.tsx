import { useEffect, useState } from "react";
import { sessionSetLanguage } from "../../adapters/ipc/session";
import { LanguageIcon } from "../../components/icons";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import {
  LANGUAGES,
  type Language,
} from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import { sourceLanguageKey } from "../../core/l10n/labels";

/**
 * Barra inferior da aba **Transcrição**: o seletor de **idioma de origem** e o
 * estado real do motor (BR-040).
 *
 * Trocar o idioma aplica ao **próximo passe** de decodificação da sessão em
 * andamento (BR-034) — sem reiniciar a sessão. O destino da tradução **não**
 * muda por aqui (fica nas Configurações). Não há play/pause nesta barra: o
 * transporte vive no notch (BR-022).
 */
export function LanguageBar() {
  const { t } = useTranslation();
  const { state } = useSessionState();
  const { settings, reload } = useSettings();
  const [value, setValue] = useState<Language>(settings.source_language);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(settings.source_language);
  }, [settings.source_language]);

  const onChange = async (next: Language) => {
    const previous = value;
    setValue(next);
    setError(null);
    try {
      await sessionSetLanguage(next);
      await reload();
    } catch (caught) {
      setError(String(caught));
      setValue(previous);
    }
  };

  const engineStatus = !state.listening
    ? t("panel_no_session")
    : state.paused
      ? t("panel_paused")
      : t("panel_recording");

  return (
    <div className="language-bar" data-interactive>
      <LanguageIcon className="language-bar__icon" size={16} />
      <label className="language-bar__field">
        <span className="language-bar__label">{t("panel_input_language")}</span>
        <select
          className="language-bar__select"
          aria-label={t("panel_transcription_language")}
          title={t("panel_transcription_source_language_changes_immediately_without",
          )}
          value={value}
          onChange={(event) => void onChange(event.target.value as Language)}
        >
          {LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {t(sourceLanguageKey(language))}
            </option>
          ))}
        </select>
      </label>
      {error ? (
        <span className="language-bar__error">{error}</span>
      ) : (
        <span className="language-bar__status">
          Whisper {settings.whisper_model} · {engineStatus}
        </span>
      )}
    </div>
  );
}
