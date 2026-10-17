import { useEffect, useState } from "react";
import {
  clearProviderKey,
  getProviders,
  getSettings,
  hasProviderKey,
  providerLabel,
  saveSettings,
  secretsPersistent,
  setProviderKey,
  validateKey,
} from "../../adapters/ipc/settings";
import {
  getShortcutCatalog,
  setShortcuts as setShortcutsIpc,
} from "../../adapters/ipc/shortcuts";
import {
  SHORTCUT_ACTIONS,
  defaultBindings,
  type ShortcutAction,
  type ShortcutConflict,
  type ShortcutInfo,
} from "../../core/domain/shortcuts";
import { AudioTestPanel } from "./AudioTestPanel";
import {
  LANGUAGES,
  PROVIDERS,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_MODELS,
  WHISPER_MODEL_SIZES,
  defaultSettings,
  providerRequiresBaseUrl,
  type Language,
  type ProviderKind,
  type Settings,
} from "../../core/domain/settings";

type Status = { kind: "info" | "ok" | "error"; text: string } | null;

/** Página de configurações: provider, modelo e chave de API (BYO key). */
export function SettingsPage() {
  const [settings, setSettings] = useState<Settings>(defaultSettings());
  const [providers, setProviders] = useState<ProviderKind[]>(PROVIDERS);
  const [keyInput, setKeyInput] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [persistent, setPersistent] = useState(true);
  const [label, setLabel] = useState("");
  const [catalog, setCatalog] = useState<ShortcutInfo[]>([]);
  const [conflicts, setConflicts] = useState<ShortcutConflict[]>([]);
  const [status, setStatus] = useState<Status>(null);
  const [busy, setBusy] = useState(false);

  const refreshKeyState = async () => {
    setHasKey((await hasProviderKey()) ?? false);
    setLabel((await providerLabel()) ?? "");
  };

  useEffect(() => {
    void (async () => {
      const loaded = await getSettings();
      if (loaded) {
        setSettings(
          loaded.shortcuts.length === 0
            ? { ...loaded, shortcuts: defaultBindings() }
            : loaded,
        );
      }
      const list = await getProviders();
      if (list && list.length > 0) {
        setProviders(list);
      }
      const shortcutCatalog = await getShortcutCatalog();
      if (shortcutCatalog && shortcutCatalog.length > 0) {
        setCatalog(shortcutCatalog);
      }
      setPersistent((await secretsPersistent()) ?? true);
      await refreshKeyState();
    })();
  }, []);

  const update = <K extends keyof Settings>(field: K, value: Settings[K]) =>
    setSettings((prev) => ({ ...prev, [field]: value }));

  const updateAccelerator = (action: ShortcutAction, accelerator: string) =>
    setSettings((prev) => {
      const exists = prev.shortcuts.some((binding) => binding.action === action);
      const shortcuts = exists
        ? prev.shortcuts.map((binding) =>
            binding.action === action ? { ...binding, accelerator } : binding,
          )
        : [...prev.shortcuts, { action, accelerator }];
      return { ...prev, shortcuts };
    });

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setStatus(null);
    try {
      await action();
    } catch (error) {
      setStatus({ kind: "error", text: String(error) });
    } finally {
      setBusy(false);
    }
  };

  const onSave = () =>
    run(async () => {
      const result = await saveSettings(settings);
      setConflicts(result ?? []);
      await refreshKeyState();
      setStatus({ kind: "ok", text: "Configurações salvas." });
    });

  const onSaveShortcuts = () =>
    run(async () => {
      const result = await setShortcutsIpc(settings.shortcuts);
      setConflicts(result ?? []);
      const hasConflict = (result?.length ?? 0) > 0;
      setStatus({
        kind: hasConflict ? "error" : "ok",
        text: hasConflict
          ? "Alguns atalhos não puderam ser registrados."
          : "Atalhos salvos.",
      });
    });

  const onSetKey = () =>
    run(async () => {
      if (keyInput.trim() === "") {
        setStatus({ kind: "error", text: "Informe uma chave de API." });
        return;
      }
      await setProviderKey(keyInput);
      setKeyInput("");
      await refreshKeyState();
      setStatus({
        kind: "ok",
        text: persistent
          ? "Chave salva no keyring."
          : "Keyring indisponível: chave válida apenas nesta sessão.",
      });
    });

  const onClearKey = () =>
    run(async () => {
      await clearProviderKey();
      await refreshKeyState();
      setStatus({ kind: "info", text: "Chave removida." });
    });

  const onValidate = () =>
    run(async () => {
      const reply = await validateKey();
      setStatus({
        kind: "ok",
        text: `Conexão validada: ${reply ?? "(sem resposta)"}`,
      });
    });

  return (
    <main className="app settings" data-route="settings">
      <h1 className="app__title">GhostWhisper — Configurações</h1>

      <section className="settings__group">
        <label className="settings__field">
          <span>Provider</span>
          <select
            value={settings.provider}
            onChange={(event) =>
              update("provider", event.target.value as ProviderKind)
            }
          >
            {providers.map((provider) => (
              <option key={provider} value={provider}>
                {provider}
              </option>
            ))}
          </select>
        </label>

        <label className="settings__field">
          <span>Modelo</span>
          <input
            type="text"
            value={settings.model}
            onChange={(event) => update("model", event.target.value)}
          />
        </label>

        {providerRequiresBaseUrl(settings.provider) && (
          <label className="settings__field">
            <span>Base URL (formato OpenAI)</span>
            <input
              type="url"
              placeholder="http://localhost:11434/v1"
              value={settings.base_url ?? ""}
              onChange={(event) =>
                update("base_url", event.target.value || null)
              }
            />
          </label>
        )}

        <label className="settings__field">
          <span>Modelo whisper (local)</span>
          <select
            value={settings.whisper_model}
            onChange={(event) => update("whisper_model", event.target.value)}
          >
            {WHISPER_MODELS.map((model) => (
              <option key={model} value={model}>
                {model} ({WHISPER_MODEL_SIZES[model] ?? "?"})
              </option>
            ))}
          </select>
        </label>
        <p className="settings__hint">
          O modelo é baixado apenas ao iniciar a primeira sessão com ele.
        </p>

        <label className="settings__field">
          <span>Idioma de origem</span>
          <select
            value={settings.source_language}
            onChange={(event) =>
              update("source_language", event.target.value as Language)
            }
          >
            {LANGUAGES.map((language) => (
              <option key={language} value={language}>
                {SOURCE_LANGUAGE_LABELS[language]}
              </option>
            ))}
          </select>
        </label>

        <label className="settings__field">
          <span>Idioma de destino</span>
          <select
            value={settings.target_language}
            onChange={(event) =>
              update("target_language", event.target.value as Language)
            }
          >
            {LANGUAGES.map((language) => (
              <option key={language} value={language}>
                {TARGET_LANGUAGE_LABELS[language]}
              </option>
            ))}
          </select>
        </label>
        <p className="settings__hint">
          Escolher um idioma de origem fixa o reconhecimento; "Auto" deixa o
          Whisper detectar. Destino "Auto" (ou igual à origem) não traduz.
        </p>

        <label className="settings__field settings__field--row">
          <input
            type="checkbox"
            checked={settings.translation_enabled}
            onChange={(event) =>
              update("translation_enabled", event.target.checked)
            }
          />
          <span>Tradução ao vivo</span>
        </label>

        <label className="settings__field settings__field--row">
          <input
            type="checkbox"
            checked={settings.stealth}
            onChange={(event) => update("stealth", event.target.checked)}
          />
          <span>Modo stealth (fora da barra de tarefas)</span>
        </label>

        <div className="settings__actions">
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onSave}
          >
            Salvar configurações
          </button>
        </div>
      </section>

      <section className="settings__group">
        <div className="settings__field settings__field--row">
          <span>
            Chave de API{" "}
            <strong>{hasKey ? "(configurada)" : "(ausente)"}</strong>
            {!persistent && " · somente sessão"}
          </span>
        </div>
        <label className="settings__field">
          <span>Nova chave</span>
          <input
            type="password"
            autoComplete="off"
            placeholder="sk-..."
            value={keyInput}
            onChange={(event) => setKeyInput(event.target.value)}
          />
        </label>
        <div className="settings__actions">
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onSetKey}
          >
            Salvar chave
          </button>
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onValidate}
          >
            Validar chave
          </button>
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onClearKey}
          >
            Remover chave
          </button>
        </div>
        {label && (
          <p className="settings__hint">Provider ativo: {label}</p>
        )}
      </section>

      <section className="settings__group">
        <span className="settings__group-title">Atalhos globais</span>
        {SHORTCUT_ACTIONS.map((action) => {
          const info = catalog.find((entry) => entry.action === action);
          const binding = settings.shortcuts.find(
            (entry) => entry.action === action,
          );
          return (
            <label
              key={action}
              className="settings__field settings__field--row settings__shortcut"
            >
              <span className="settings__shortcut-label">
                {info?.label ?? action}
              </span>
              <input
                type="text"
                value={binding?.accelerator ?? ""}
                placeholder={info?.default_accelerator ?? ""}
                onChange={(event) =>
                  updateAccelerator(action, event.target.value)
                }
              />
            </label>
          );
        })}
        <div className="settings__actions">
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onSaveShortcuts}
          >
            Salvar atalhos
          </button>
        </div>
        {conflicts.length > 0 && (
          <ul className="settings__conflicts">
            {conflicts.map((conflict) => (
              <li key={`${conflict.action}-${conflict.accelerator}`}>
                {conflict.accelerator}: {conflict.reason}
              </li>
            ))}
          </ul>
        )}
      </section>

      <AudioTestPanel />

      {status && (
        <p className={`settings__status settings__status--${status.kind}`}>
          {status.text}
        </p>
      )}
    </main>
  );
}
