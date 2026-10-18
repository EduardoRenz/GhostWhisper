import { useCallback, useEffect, useState } from "react";
import {
  clearProviderKey,
  getProviders,
  getSettings,
  gpuInfo,
  hasProviderKey,
  keyPreview,
  providerLabel,
  saveSettings,
  secretsPersistent,
  setProviderKey,
  validateKey,
  type GpuInfo,
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
import { AssistantSettings } from "./AssistantSettings";
import {
  LANGUAGES,
  MAX_MIC_GAIN,
  MIN_MIC_GAIN,
  PROVIDERS,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_GPU_MODELS,
  WHISPER_MODELS,
  WHISPER_MODEL_SIZES,
  DEFAULT_MIC_GAIN,
  defaultSettings,
  providerRequiresBaseUrl,
  type Language,
  type ProviderKind,
  type Settings,
} from "../../core/domain/settings";

type Status = { kind: "info" | "ok" | "error"; text: string } | null;

/** Abas da janela de configurações (Perssua usa `tabs` por seção). */
const TABS = [
  { id: "sources", label: "Fontes de IA" },
  { id: "transcription", label: "Transcrição & Áudio" },
  { id: "assistant", label: "Assistente" },
  { id: "general", label: "Ajustes" },
  { id: "shortcuts", label: "Atalhos" },
] as const;

type TabId = (typeof TABS)[number]["id"];

/** Página de configurações: provider, modelo e chave de API (BYO key). */
export function SettingsPage() {
  const [settings, setSettings] = useState<Settings>(defaultSettings());
  const [providers, setProviders] = useState<ProviderKind[]>(PROVIDERS);
  const [tab, setTab] = useState<TabId>("sources");
  const [keyInput, setKeyInput] = useState("");
  const [keyEditing, setKeyEditing] = useState(false);
  const [hasKey, setHasKey] = useState(false);
  const [keyMasked, setKeyMasked] = useState("");
  const [persistent, setPersistent] = useState(true);
  const [label, setLabel] = useState("");
  const [gpu, setGpu] = useState<GpuInfo | null>(null);
  const [catalog, setCatalog] = useState<ShortcutInfo[]>([]);
  const [conflicts, setConflicts] = useState<ShortcutConflict[]>([]);
  const [status, setStatus] = useState<Status>(null);
  const [busy, setBusy] = useState(false);

  const refreshKeyState = useCallback(async () => {
    setHasKey((await hasProviderKey()) ?? false);
    setKeyMasked((await keyPreview()) ?? "");
    setLabel((await providerLabel()) ?? "");
  }, []);

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
      setGpu((await gpuInfo()) ?? null);
      await refreshKeyState();
    })();
  }, [refreshKeyState]);

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
      setKeyEditing(false);
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
      setKeyEditing(false);
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

  /** Campo da chave: mostra a versão redactada quando já configurada. */
  const keyField =
    hasKey && !keyEditing ? (
      <>
        <label className="settings__field">
          <span>Chave de API</span>
          <input
            className="settings__key"
            type="text"
            readOnly
            value={keyMasked || "••••••••"}
            aria-label="Chave de API (mascarada)"
            onChange={undefined}
            onFocus={undefined}
          />
        </label>
        <p className="settings__hint">
          Chave armazenada{" "}
          {persistent ? "no keyring do sistema" : "apenas nesta sessão"} —
          exibição mascarada (últimos 4 caracteres). Use "Trocar chave" para
          substituí-la.
        </p>
        <div className="settings__actions">
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={() => setKeyEditing(true)}
          >
            Trocar chave
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
      </>
    ) : (
      <>
        <label className="settings__field">
          <span>{hasKey ? "Trocar chave" : "Nova chave"}</span>
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
          {hasKey && (
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={() => setKeyEditing(false)}
            >
              Cancelar
            </button>
          )}
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onClearKey}
          >
            Remover chave
          </button>
        </div>
      </>
    );

  return (
    <main className="app settings" data-route="settings">
      <h1 className="app__title">GhostWhisper — Configurações</h1>

      <nav className="settings__nav" role="tablist">
        {TABS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={tab === entry.id}
            className={
              tab === entry.id ? "settings__tab settings__tab--active" : "settings__tab"
            }
            onClick={() => setTab(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </nav>

      {tab === "sources" && (
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

          {keyField}

          {label && <p className="settings__hint">Provider ativo: {label}</p>}

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
      )}

      {tab === "transcription" && (
        <section className="settings__group">
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

          {gpu && (
            <p className="settings__hint" data-testid="gpu-hint">
              {gpu.available
                ? `GPU detectada: ${gpu.detail}. ${
                    gpu.acceleration_enabled
                      ? `Modelo sugerido: ${gpu.recommended_model}.`
                      : "Este build ainda transcreve em CPU; a aceleração por GPU chega em uma próxima entrega."
                  }`
                : "Nenhuma GPU detectada: a transcrição roda em CPU (use tiny/small)."}
            </p>
          )}

          {gpu &&
            !gpu.acceleration_enabled &&
            WHISPER_GPU_MODELS.includes(settings.whisper_model) && (
              <p className="settings__hint">
                {settings.whisper_model} exige GPU para acompanhar a fala em tempo
                real; em CPU a transcrição fica lenta.
              </p>
            )}

          <label className="settings__field">
            <span>Prompt de decodificação (opcional)</span>
            <textarea
              rows={3}
              placeholder="Only transcribe what is clearly spoken. If uncertain, transcribe nothing."
              value={settings.whisper_prompt}
              onChange={(event) =>
                update("whisper_prompt", event.target.value)
              }
            />
          </label>
          <p className="settings__hint">
            Instrui o Whisper a não inventar quando não tem certeza; a regra de
            idioma é acrescentada automaticamente. Deixar vazio desliga o prompt.
          </p>

          <div className="settings__field">
            <span>Sensibilidade do microfone</span>
            <input
              type="range"
              aria-label="Sensibilidade do microfone"
              min={MIN_MIC_GAIN}
              max={MAX_MIC_GAIN}
              step={0.5}
              value={settings.mic_gain}
              onChange={(event) =>
                update("mic_gain", Number(event.target.value))
              }
            />
            <span className="settings__value">
              {settings.mic_gain === DEFAULT_MIC_GAIN
                ? `${settings.mic_gain.toFixed(1)}× (padrão)`
                : `${settings.mic_gain.toFixed(1)}×`}
            </span>
          </div>
          <p className="settings__hint">
            Amplifica o microfone antes da transcrição — o áudio do sistema não é
            alterado. Se a fala quase não aparece, aumente; se disparar com
            ruído de fundo, diminua. Vale a partir da próxima captura: use o
            “Teste de áudio” abaixo para conferir o nível.
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

          <AudioTestPanel micGain={settings.mic_gain} />
        </section>
      )}

      {tab === "assistant" && (
        <section className="settings__group">
          <AssistantSettings settings={settings} update={update} />
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
      )}

      {tab === "general" && (
        <section className="settings__group">
          <label className="settings__field settings__field--row">
            <input
              type="checkbox"
              checked={settings.stealth}
              onChange={(event) => update("stealth", event.target.checked)}
            />
            <span>Modo stealth (fora da barra de tarefas)</span>
          </label>
          <p className="settings__hint">
            O modo stealth esconde o overlay da barra de tarefas e do
            alt-tab. <strong>Ele não o esconde do compartilhamento de tela</strong>:
            no Linux não existe API para excluir uma janela de captura (BR-006).
            Para reuniões, compartilhe <em>uma janela</em> em vez da tela inteira
            ou deixe o overlay em outro monitor.
          </p>

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
      )}

      {tab === "shortcuts" && (
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
      )}

      {status && (
        <p className={`settings__status settings__status--${status.kind}`}>
          {status.text}
        </p>
      )}
    </main>
  );
}
