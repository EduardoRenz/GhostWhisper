import { useEffect, useState } from "react";
import { sessionSetLanguage } from "../../adapters/ipc/session";
import { LanguageIcon } from "../../components/icons";
import { useSettings } from "../../core/application/useSettings";
import {
  LANGUAGES,
  SOURCE_LANGUAGE_LABELS,
  type Language,
} from "../../core/domain/settings";

/**
 * Barra inferior da aba **Transcrição**: o seletor de **idioma de origem**.
 *
 * Trocar o idioma aplica ao **próximo passe** de decodificação da sessão em
 * andamento (BR-034) — sem reiniciar a sessão. O destino da tradução **não**
 * muda por aqui (fica nas Configurações). Não há play/pause nesta barra: o
 * transporte vive no notch (BR-022).
 */
export function LanguageBar() {
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

  return (
    <div className="language-bar" data-interactive>
      <LanguageIcon className="language-bar__icon" size={16} />
      <label className="language-bar__field">
        <span className="language-bar__label">Idioma</span>
        <select
          className="language-bar__select"
          aria-label="Idioma da transcrição"
          title="Idioma de origem da transcrição — muda na hora, sem reiniciar a sessão"
          value={value}
          onChange={(event) => void onChange(event.target.value as Language)}
        >
          {LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {SOURCE_LANGUAGE_LABELS[language]}
            </option>
          ))}
        </select>
      </label>
      {error ? (
        <span className="language-bar__error">{error}</span>
      ) : (
        <span className="language-bar__hint">muda na hora</span>
      )}
    </div>
  );
}
