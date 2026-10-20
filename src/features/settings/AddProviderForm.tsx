import { useState } from "react";
import {
  MODEL_PLACEHOLDER,
  PROVIDER_DEFAULT_BASE_URLS,
  PROVIDER_LABELS,
  PROVIDERS,
  newSourceId,
  providerRequiresBaseUrl,
  type AiSource,
  type ProviderKind,
} from "../../core/domain/settings";

interface AddProviderFormProps {
  /** Provedores suportados (vindos do core). */
  providers: ProviderKind[];
  /** Cria a fonte no rascunho (ainda não salva). */
  onAdd: (source: AiSource) => void;
}

/**
 * Formulário "Adicionar Nova Fonte de IA" (BR-044).
 *
 * O **modelo é digitado pelo usuário** (sem catálogo — a lista de modelos
 * desatualiza rápido e trava quem usa um modelo novo); a fonte nasce **no
 * rascunho** e só é persistida pelo "Salvar Alterações".
 */
export function AddProviderForm({ providers, onAdd }: AddProviderFormProps) {
  const [kind, setKind] = useState<ProviderKind>("openai");
  const [model, setModel] = useState("");
  const [baseUrl, setBaseUrl] = useState("");

  const changeKind = (next: ProviderKind) => {
    setKind(next);
    setModel("");
    setBaseUrl(PROVIDER_DEFAULT_BASE_URLS[next] ?? "");
  };

  const canAdd =
    model.trim() !== "" &&
    (!providerRequiresBaseUrl(kind) || baseUrl.trim() !== "");

  const submit = () => {
    if (!canAdd) return;
    onAdd({
      id: newSourceId(),
      kind,
      label: PROVIDER_LABELS[kind],
      model: model.trim(),
      base_url:
        kind === "custom" ? baseUrl.trim() : (PROVIDER_DEFAULT_BASE_URLS[kind] ?? null),
      enabled: true,
    });
  };

  return (
    <section className="settings__card settings__card--add">
      <header className="settings__card-header">
        <h2 className="settings__card-title">Adicionar Nova Fonte de IA</h2>
        <span className="settings__card-note">Armazenamento seguro no keyring do sistema</span>
      </header>

      <div className="settings__source-add">
        <label className="settings__field">
          <span>Selecionar Provedor</span>
          <select
            value={kind}
            onChange={(event) => changeKind(event.target.value as ProviderKind)}
          >
            {providers.length > 0 ? (
              providers.map((provider) => (
                <option key={provider} value={provider}>
                  {PROVIDER_LABELS[provider]}
                </option>
              ))
            ) : (
              PROVIDERS.map((provider) => (
                <option key={provider} value={provider}>
                  {PROVIDER_LABELS[provider]}
                </option>
              ))
            )}
          </select>
        </label>

        <label className="settings__field">
          <span>Modelo</span>
          <input
            type="text"
            value={model}
            placeholder={MODEL_PLACEHOLDER}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => setModel(event.target.value)}
          />
        </label>

        {providerRequiresBaseUrl(kind) && (
          <label className="settings__field">
            <span>Base URL (formato OpenAI)</span>
            <input
              type="url"
              value={baseUrl}
              placeholder="http://localhost:11434/v1"
              onChange={(event) => setBaseUrl(event.target.value)}
            />
          </label>
        )}

        <button
          type="button"
          className="settings__button settings__button--primary"
          disabled={!canAdd}
          onClick={submit}
        >
          Adicionar
        </button>
      </div>
      <p className="settings__hint">
        O nome do modelo é digitado por você — a lista de modelos de cada
        provedor envelhece rápido e deixaria você preso num modelo antigo.
      </p>
    </section>
  );
}
