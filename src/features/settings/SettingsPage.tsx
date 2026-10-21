import { useCallback, useEffect, useRef, useState } from "react";
import {
  closeSettings,
  getProviders,
  getSettings,
  gpuInfo,
  previewOverlayOpacity,
  saveSettings,
  secretsPersistent,
  setOverlayOpacity,
  type GpuInfo,
} from "../../adapters/ipc/settings";
import {
  getShortcutCatalog,
  setShortcuts as setShortcutsIpc,
} from "../../adapters/ipc/shortcuts";
import { panelStartDrag } from "../../adapters/ipc/panels";
import {
  SHORTCUT_ACTIONS,
  defaultBindings,
  type ShortcutAction,
  type ShortcutConflict,
  type ShortcutInfo,
} from "../../core/domain/shortcuts";
import { ResizeHandles } from "../panels/ResizeHandles";
import { WindowHeader } from "../../components/WindowHeader";
import { applyHudOpacity } from "../../core/application/useHudOpacity";
import { AudioTestPanel } from "./AudioTestPanel";
import { AssistantSettings } from "./AssistantSettings";
import { RecordingSettings } from "./RecordingSettings";
import { AiSourcesSettings } from "./AiSourcesSettings";
import { HudSettings } from "./HudSettings";
import {
  BoltIcon,
  KeyboardIcon,
  SlidersIcon,
  TranscriptIcon,
  UsersIcon,
  VideoCameraIcon,
  WaveformIcon,
} from "../../components/icons";
import {
  ACCELERATION_LABELS,
  ACCELERATION_MODES,
  DEFAULT_MIC_GAIN,
  LANGUAGES,
  MAX_MIC_GAIN,
  MIN_MIC_GAIN,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_GPU_MODELS,
  WHISPER_MODELS,
  WHISPER_MODEL_SIZES,
  DEFAULT_OVERLAY_OPACITY,
  defaultSettings,
  type AccelerationMode,
  type Language,
  type ProviderKind,
  type Settings,
} from "../../core/domain/settings";

type Status = { kind: "info" | "ok" | "error"; text: string } | null;

/** Abas da janela de configurações (as 5 seções do produto). */
const TABS = [
  { id: "sources", label: "Fontes de IA", Icon: BoltIcon },
  { id: "transcription", label: "Transcrição & Áudio", Icon: WaveformIcon },
  { id: "recording", label: "Gravação", Icon: VideoCameraIcon },
  { id: "assistant", label: "Assistente", Icon: UsersIcon },
  { id: "general", label: "Geral & HUD", Icon: SlidersIcon },
  { id: "shortcuts", label: "Atalhos", Icon: KeyboardIcon },
] as const;

type TabId = (typeof TABS)[number]["id"];

/** Aba inicial pela URL (`?tab=…`) — atalho de desenvolvimento. */
function initialTab(): TabId {
  try {
    const target = new URLSearchParams(window.location.search).get("tab");
    const found = TABS.find((entry) => entry.id === target);
    if (found) return found.id;
  } catch {
    // Sem DOM (SSR/testes): começa na primeira aba.
  }
  return "sources";
}

/**
 * Tela de Configurações (BR-047).
 *
 * Casca nova: cabeçalho próprio, barra de abas com ícone, conteúdo em cards e
 * **um** "Salvar Alterações" no rodapé fixo. As seções são as mesmas de
 * antes — a lista de fontes de IA é a primeira delas.
 */
export function SettingsPage() {
  const [draft, setDraft] = useState<Settings>(defaultSettings());
  const [providers, setProviders] = useState<ProviderKind[]>([]);
  const [tab, setTab] = useState<TabId>(initialTab);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [gpu, setGpu] = useState<GpuInfo | null>(null);
  const [catalog, setCatalog] = useState<ShortcutInfo[]>([]);
  const [conflicts, setConflicts] = useState<ShortcutConflict[]>([]);
  const [status, setStatus] = useState<Status>(null);
  const [busy, setBusy] = useState(false);
  const [persistent, setPersistent] = useState(true);
  /** Debounce do autosave da opacidade (BR-062). */
  const opacityTimer = useRef<number | null>(null);
  const latestOpacity = useRef(DEFAULT_OVERLAY_OPACITY);

  useEffect(() => {
    void (async () => {
      const loaded = await getSettings();
      if (loaded) {
        latestOpacity.current = loaded.overlay_opacity;
        setDraft(
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
    })();
  }, []);

  // `Esc` fecha a janela (mesmo atalho do botão `✕`).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void closeSettings();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  /** Persiste só a opacidade, sem reaplicar atalhos/geometria. */
  const persistOpacity = useCallback(async (value: number) => {
    try {
      await setOverlayOpacity(value);
    } catch (error) {
      setStatus({
        kind: "error",
        text: `Falha ao salvar a opacidade: ${String(error)}`,
      });
    }
  }, []);

  const update = useCallback(
    <K extends keyof Settings>(field: K, value: Settings[K]) => {
      setDraft((prev) => ({ ...prev, [field]: value }));

      // A opacidade é global e precisa refletir em todas as janelas abertas
      // antes do save (BR-062). O autosave coalesce o arraste do slider.
      if (field === "overlay_opacity") {
        const next = Number(value);
        latestOpacity.current = next;
        applyHudOpacity(next);
        void previewOverlayOpacity(next).catch(() => undefined);
        if (opacityTimer.current !== null) {
          window.clearTimeout(opacityTimer.current);
        }
        opacityTimer.current = window.setTimeout(() => {
          opacityTimer.current = null;
          void persistOpacity(latestOpacity.current);
        }, 200);
        return;
      }

      setHasUnsavedChanges(true);
    },
    [persistOpacity],
  );

  // Se a janela for fechada/desmontada antes do debounce, grava o último valor.
  useEffect(
    () => () => {
      if (opacityTimer.current !== null) {
        window.clearTimeout(opacityTimer.current);
        opacityTimer.current = null;
        void setOverlayOpacity(latestOpacity.current).catch(() => undefined);
      }
    },
    [],
  );

  const updateAccelerator = (action: ShortcutAction, accelerator: string) => {
    setDraft((prev) => {
      const exists = prev.shortcuts.some((binding) => binding.action === action);
      const shortcuts = exists
        ? prev.shortcuts.map((binding) =>
            binding.action === action ? { ...binding, accelerator } : binding,
          )
        : [...prev.shortcuts, { action, accelerator }];
      return { ...prev, shortcuts };
    });
    setHasUnsavedChanges(true);
  };

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
      if (opacityTimer.current !== null) {
        window.clearTimeout(opacityTimer.current);
        opacityTimer.current = null;
      }
      try {
        // Perguntas prontas em branco (linha recém-criada) não são persistidas.
        const result = await saveSettings({
          ...draft,
          saved_questions: (draft.saved_questions ?? []).filter(
            (question) => question.text.trim() !== "",
          ),
        });
        setConflicts(result ?? []);
        setHasUnsavedChanges(false);
        setStatus({ kind: "ok", text: "Configurações salvas." });
      } catch (error) {
        // Se o save completo falhar, a opacidade (autosave) ainda é gravada
        // pelo seu próprio comando.
        await persistOpacity(latestOpacity.current);
        throw error;
      }
    });

  const onRestoreDefaults = () => {
    const confirmed =
      typeof window === "undefined" ||
      typeof window.confirm !== "function" ||
      window.confirm(
        "Restaurar os padrões? Isto zera modelos, atalhos, agentes e as fontes de IA (as chaves no keyring não são apagadas).",
      );
    if (!confirmed) return;
    setDraft({ ...defaultSettings(), shortcuts: defaultBindings() });
    setHasUnsavedChanges(true);
    setStatus({
      kind: "info",
      text: "Padrões restaurados — clique em “Salvar Alterações” para confirmar.",
    });
  };

  const onSaveShortcuts = () =>
    run(async () => {
      const result = await setShortcutsIpc(draft.shortcuts);
      setConflicts(result ?? []);
      const hasConflict = (result?.length ?? 0) > 0;
      setStatus({
        kind: hasConflict ? "error" : "ok",
        text: hasConflict
          ? "Alguns atalhos não puderam ser registrados."
          : "Atalhos registrados no sistema.",
      });
    });

  return (
    <main className="app panel-shell settings-shell" data-route="settings">
      <ResizeHandles />
      <div className="settings">
        <WindowHeader
          className="settings__header"
          title="Configurações"
          subtitle="Personalize captura de áudio, IA local, privacidade e atalhos do GhostWhisper"
          icon={<TranscriptIcon size={18} />}
          hint={<kbd className="window-header__hint">Esc</kbd>}
          onStartDrag={() => void panelStartDrag()}
          onClose={() => void closeSettings()}
          closeLabel="Fechar configurações"
          dragLabel="Mover configurações"
          headingLevel="h1"
        />

        <nav className="settings__tabs" role="tablist" aria-label="Seções">
        {TABS.map((entry) => {
          const Icon = entry.Icon;
          return (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={tab === entry.id}
              className={
                tab === entry.id
                  ? "settings__tab settings__tab--active"
                  : "settings__tab"
              }
              onClick={() => setTab(entry.id)}
            >
              <Icon size={16} />
              <span>{entry.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="settings__body">
        {tab === "sources" && (
          <AiSourcesSettings
            sources={draft.ai_sources}
            providers={providers}
            onChange={(sources) => update("ai_sources", sources)}
            onStatus={setStatus}
          />
        )}

        {tab === "transcription" && (
          <section className="settings__card">
            <header className="settings__card-header">
              <h2 className="settings__card-title">Transcrição & Áudio</h2>
              <span className="settings__card-note">
                Modelo local, aceleração e níveis de entrada
              </span>
            </header>

            <label className="settings__field">
              <span>Modelo whisper (local)</span>
              <select
                value={draft.whisper_model}
                onChange={(event) =>
                  update("whisper_model", event.target.value)
                }
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
              gpu.available &&
              gpu.acceleration_enabled &&
              draft.whisper_model !== gpu.recommended_model && (
                <div className="settings__actions">
                  <button
                    type="button"
                    className="settings__button"
                    onClick={() => update("whisper_model", gpu.recommended_model)}
                  >
                    Usar {gpu.recommended_model} (recomendado para a sua GPU)
                  </button>
                </div>
              )}

            {gpu &&
              !gpu.acceleration_enabled &&
              WHISPER_GPU_MODELS.includes(draft.whisper_model) && (
                <p className="settings__hint">
                  {draft.whisper_model} exige GPU para acompanhar a fala em
                  tempo real; em CPU a transcrição fica lenta.
                </p>
              )}

            <label className="settings__field">
              <span>Aceleração</span>
              <select
                value={draft.acceleration}
                onChange={(event) =>
                  update("acceleration", event.target.value as AccelerationMode)
                }
              >
                {ACCELERATION_MODES.map((mode) => (
                  <option key={mode} value={mode}>
                    {ACCELERATION_LABELS[mode]}
                  </option>
                ))}
              </select>
            </label>
            <p className="settings__hint">
              {gpu?.acceleration_enabled
                ? gpu.available
                  ? "Este build acelera por GPU (Vulkan). “Auto” usa a GPU quando ela está disponível."
                  : "Este build acelera por GPU, mas nenhuma GPU foi detectada: a transcrição roda em CPU."
                : "Este build não inclui aceleração por GPU — a transcrição roda em CPU."}
            </p>

            <label className="settings__field">
              <span>Contexto da transcrição (opcional)</span>
              <textarea
                rows={3}
                placeholder="Ex.: Grafana, Kubernetes, nomes da equipe"
                value={draft.whisper_prompt}
                onChange={(event) => update("whisper_prompt", event.target.value)}
              />
            </label>
            <p className="settings__hint">
              Use com cautela e apenas para <strong>contexto</strong>: nomes,
              siglas e jargões que costumam aparecer. <strong>Não escreva
              instruções</strong> como “não invente” ou “transcreva só o que
              estiver claro”: o Whisper trata este texto como fala anterior e
              pode repeti-lo quando o áudio está incerto, atrapalhando a
              transcrição. O recomendado é deixar vazio.
            </p>

            <label className="settings__field settings__field--row">
              <input
                type="checkbox"
                checked={draft.aec_enabled}
                onChange={(event) => update("aec_enabled", event.target.checked)}
              />
              <span>Cancelar eco (AEC)</span>
            </label>
            <p className="settings__hint">
              Subtrai do microfone o áudio que sai pelo alto-falante (o outro
              lado da chamada), usando o áudio do sistema como referência. Vale a
              partir da próxima captura; com fones não faz diferença.
            </p>

            <div className="settings__field">
              <span>Sensibilidade do microfone</span>
              <input
                type="range"
                aria-label="Sensibilidade do microfone"
                min={MIN_MIC_GAIN}
                max={MAX_MIC_GAIN}
                step={0.5}
                value={draft.mic_gain}
                onChange={(event) =>
                  update("mic_gain", Number(event.target.value))
                }
              />
              <span className="settings__value">
                {draft.mic_gain === DEFAULT_MIC_GAIN
                  ? `${draft.mic_gain.toFixed(1)}× (padrão)`
                  : `${draft.mic_gain.toFixed(1)}×`}
              </span>
            </div>
            <p className="settings__hint">
              Amplifica o microfone antes da transcrição — o áudio do sistema não
              é alterado. Se a fala quase não aparece, aumente; se disparar com
              ruído de fundo, diminua. Vale a partir da próxima captura: use o
              “Teste de áudio” abaixo para conferir o nível.
            </p>

            <label className="settings__field">
              <span>Idioma de origem</span>
              <select
                value={draft.source_language}
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
                value={draft.target_language}
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
                checked={draft.translation_enabled}
                onChange={(event) =>
                  update("translation_enabled", event.target.checked)
                }
              />
              <span>Tradução ao vivo</span>
            </label>

            <AudioTestPanel micGain={draft.mic_gain} />
          </section>
        )}

        {tab === "recording" && (
          <RecordingSettings settings={draft} update={update} />
        )}

        {tab === "assistant" && (
          <section className="settings__card">
            <header className="settings__card-header">
              <h2 className="settings__card-title">Assistente</h2>
              <span className="settings__card-note">
                Agentes, cadências e personalização
              </span>
            </header>
            <AssistantSettings settings={draft} update={update} />
          </section>
        )}

        {tab === "general" && (
          <HudSettings settings={draft} update={update} />
        )}

        {tab === "shortcuts" && (
          <section className="settings__card">
            <header className="settings__card-header">
              <h2 className="settings__card-title">Atalhos Globais</h2>
              <span className="settings__card-note">
                Atalho de teclado do sistema operacional
              </span>
            </header>
            <p className="settings__hint">
              O botão abaixo <strong>registra</strong> os atalhos no sistema. O
              valor do atalho é salvo junto com o resto (botão “Salvar
              Alterações”).
            </p>
            {SHORTCUT_ACTIONS.map((action) => {
              const info = catalog.find((entry) => entry.action === action);
              const binding = draft.shortcuts.find(
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
                className="settings__button settings__button--primary"
                disabled={busy}
                onClick={onSaveShortcuts}
              >
                Aplicar atalhos
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
      </div>

      <footer className="settings__footer">
        <span className="settings__footer-note">
          <span
            className={`settings__footer-dot${
              hasUnsavedChanges ? " settings__footer-dot--dirty" : ""
            }`}
            aria-hidden="true"
          />
          <span>
            {hasUnsavedChanges
              ? "Alterações não salvas"
              : "Todas as alterações são sincronizadas localmente"}
          </span>
          {" · "}
          <span>
            {persistent ? "Chaves no keyring do sistema" : "Chaves só nesta sessão"}
          </span>
        </span>
        <div className="settings__footer-actions">
          <button
            type="button"
            className="settings__button"
            disabled={busy}
            onClick={onRestoreDefaults}
          >
            Restaurar Padrões
          </button>
          <button
            type="button"
            className="settings__button settings__button--primary"
            disabled={busy}
            onClick={onSave}
          >
            Salvar Alterações
          </button>
        </div>
      </footer>
      </div>
    </main>
  );
}
