import { useCallback, useEffect, useRef, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  closeSettings,
  getProviders,
  getSettings,
  gpuInfo,
  previewFontScales,
  previewOverlayOpacity,
  saveSettings,
  secretsPersistent,
  setFontScales,
  setOverlayOpacity,
  settingsDefaults,
  type GpuInfo,
} from "../../adapters/ipc/settings";
import {
  getShortcutCatalog,
  setShortcuts as setShortcutsIpc,
} from "../../adapters/ipc/shortcuts";
import { startWindowDrag } from "../../adapters/ipc/windowDrag";
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
import {
  applyFontScale,
  type FontScales,
} from "../../core/application/useFontScale";
import { AudioTestPanel } from "./AudioTestPanel";
import { AssistantsSettings } from "./AssistantsSettings";
import { CadenceSettings } from "./CadenceSettings";
import { LogsSettings } from "./LogsSettings";
import { LanguageSettings } from "./LanguageSettings";
import { RecordingSettings } from "./RecordingSettings";
import { AiSourcesSettings } from "./AiSourcesSettings";
import { HudSettings } from "./HudSettings";
import { SuggestionsActivationBlock } from "./SuggestionsActivationBlock";
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
  ACCELERATION_MODES,
  DEFAULT_AI_FONT_SCALE,
  DEFAULT_FONT_SCALE,
  DEFAULT_MIC_GAIN,
  DEFAULT_OVERLAY_OPACITY,
  LANGUAGES,
  MAX_MIC_GAIN,
  MIN_MIC_GAIN,
  WHISPER_GPU_MODELS,
  WHISPER_MODELS,
  WHISPER_MODEL_SIZES,
  clampAiFontScale,
  clampFontScale,
  defaultSettings,
  type AccelerationMode,
  type Language,
  type ProviderKind,
  type Settings,
} from "../../core/domain/settings";
import {
  getUiLanguage,
  setUiLanguage,
  useTranslation,
  type TranslationKey,
} from "../../core/l10n";
import {
  accelerationKey,
  shortcutActionKey,
  sourceLanguageKey,
  targetLanguageKey,
} from "../../core/l10n/labels";

type Status = { kind: "info" | "ok" | "error"; text: string } | null;

/**
 * Abas da janela de configurações (as seções do produto).
 *
 * **Geral & HUD é a primeira** (decisão de produto): idioma da interface,
 * comportamento do HUD e cadências são o que o usuário procura primeiro.
 */
const TABS: { id: TabId; label: TranslationKey; Icon: typeof SlidersIcon }[] = [
  { id: "general", label: "settings_tab_general", Icon: SlidersIcon },
  { id: "sources", label: "settings_tab_sources", Icon: BoltIcon },
  { id: "transcription", label: "settings_tab_transcription", Icon: WaveformIcon },
  { id: "recording", label: "settings_tab_recording", Icon: VideoCameraIcon },
  { id: "assistant", label: "settings_tab_assistant", Icon: UsersIcon },
  { id: "shortcuts", label: "settings_tab_shortcuts", Icon: KeyboardIcon },
];

type TabId =
  | "general"
  | "sources"
  | "transcription"
  | "recording"
  | "assistant"
  | "shortcuts";

/** Aba inicial pela URL (`?tab=…`) — atalho de desenvolvimento. */
function initialTab(): TabId {
  try {
    const target = new URLSearchParams(window.location.search).get("tab");
    const found = TABS.find((entry) => entry.id === target);
    if (found) return found.id;
  } catch {
    // Sem DOM (SSR/testes): começa na primeira aba.
  }
  return "general";
}

/**
 * Tela de Configurações (BR-047).
 *
 * Casca nova: cabeçalho próprio, barra de abas com ícone, conteúdo em cards e
 * **um** "Salvar Alterações" no rodapé fixo. As seções são as mesmas de
 * antes — a lista de fontes de IA é a primeira delas.
 */
export function SettingsPage() {
  const { t } = useTranslation();
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
  /** Debounce do autosave das escalas de fonte (BR-087). */
  const fontTimer = useRef<number | null>(null);
  const latestFont = useRef<FontScales>({
    fontScale: DEFAULT_FONT_SCALE,
    aiFontScale: DEFAULT_AI_FONT_SCALE,
  });

  useEffect(() => {
    void (async () => {
      const loaded = await getSettings();
      if (loaded) {
        latestOpacity.current = loaded.overlay_opacity;
        latestFont.current = {
          fontScale: clampFontScale(loaded.font_scale),
          aiFontScale: clampAiFontScale(loaded.ai_font_scale),
        };
        // Configurações monta direto (testes/rota): alimenta o store de idioma.
        setUiLanguage(loaded.ui_language);
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

  // Outra janela salvou (ex.: o seletor global de agentes do notch — BR-080):
  // mantém a badge "Ativo" em tempo real. Só o agente é sincronizado; o resto
  // do rascunho fica intocado (não atrapalha edição/autosave da opacidade).
  useEffect(() => {
    let stop: (() => void) | undefined;
    void subscribe<null>("settings_changed", () => {
      void getSettings().then((loaded) => {
        if (!loaded) {
          return;
        }
        setUiLanguage(loaded.ui_language);
        setDraft((previous) =>
          previous.assistant_mode_id === loaded.assistant_mode_id &&
          previous.ui_language === loaded.ui_language
            ? previous
            : {
                ...previous,
                assistant_mode_id: loaded.assistant_mode_id,
                ui_language: loaded.ui_language,
              },
        );
      });
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, []);

  // `Esc` fecha a janela (mesmo atalho do botão `✕`). Com um modal aberto, o
  // `Dialog` consome o evento antes (o modal fecha, a janela fica).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        void closeSettings();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  /** Persiste só a opacidade, sem reaplicar atalhos/geometria. */
  const persistOpacity = useCallback(
    async (value: number) => {
      try {
        await setOverlayOpacity(value);
      } catch (error) {
        setStatus({
          kind: "error",
          text: t("settings_failed_save_opacity_error", {
            error: String(error),
          }),
        });
      }
    },
    [t],
  );

  /** Persiste só as escalas de fonte, sem reaplicar atalhos/geometria. */
  const persistFontScales = useCallback(
    async (fontScale: number, aiFontScale: number) => {
      try {
        await setFontScales(fontScale, aiFontScale);
      } catch (error) {
        setStatus({
          kind: "error",
          text: t("settings_failed_save_font_scale_error", {
            error: String(error),
          }),
        });
      }
    },
    [t],
  );

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

      // Mesmo tratamento da opacidade para as escalas de fonte (BR-087): o
      // preview vale já no arraste, em todas as janelas de conteúdo.
      if (field === "font_scale" || field === "ai_font_scale") {
        const next: FontScales = {
          fontScale:
            field === "font_scale"
              ? clampFontScale(Number(value))
              : latestFont.current.fontScale,
          aiFontScale:
            field === "ai_font_scale"
              ? clampAiFontScale(Number(value))
              : latestFont.current.aiFontScale,
        };
        latestFont.current = next;
        applyFontScale(next.fontScale, next.aiFontScale, true);
        void previewFontScales(next.fontScale, next.aiFontScale).catch(
          () => undefined,
        );
        if (fontTimer.current !== null) {
          window.clearTimeout(fontTimer.current);
        }
        fontTimer.current = window.setTimeout(() => {
          fontTimer.current = null;
          void persistFontScales(
            latestFont.current.fontScale,
            latestFont.current.aiFontScale,
          );
        }, 200);
        return;
      }

      setHasUnsavedChanges(true);
    },
    [persistOpacity, persistFontScales],
  );

  // Se a janela for fechada/desmontada antes do debounce, grava o último valor.
  useEffect(
    () => () => {
      if (opacityTimer.current !== null) {
        window.clearTimeout(opacityTimer.current);
        opacityTimer.current = null;
        void setOverlayOpacity(latestOpacity.current).catch(() => undefined);
      }
      if (fontTimer.current !== null) {
        window.clearTimeout(fontTimer.current);
        fontTimer.current = null;
        void setFontScales(
          latestFont.current.fontScale,
          latestFont.current.aiFontScale,
        ).catch(() => undefined);
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
      if (fontTimer.current !== null) {
        window.clearTimeout(fontTimer.current);
        fontTimer.current = null;
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
        setUiLanguage(draft.ui_language);
        setStatus({ kind: "ok", text: t("settings_settings_saved") });
      } catch (error) {
        // Se o save completo falhar, a opacidade e as escalas de fonte
        // (autosave) ainda são gravadas pelos seus próprios comandos.
        await persistOpacity(latestOpacity.current);
        await persistFontScales(
          latestFont.current.fontScale,
          latestFont.current.aiFontScale,
        );
        throw error;
      }
    });

  const onRestoreDefaults = () => {
    const confirmed =
      typeof window === "undefined" ||
      typeof window.confirm !== "function" ||
      window.confirm(
        t("settings_restore_defaults_resets_models_shortcuts_ai",
        ),
      );
    if (!confirmed) return;
    void run(async () => {
      // Os agentes de fábrica (com os contratos completos) vêm do core: o
      // frontend não os reproduz (BR-075).
      const defaults = await settingsDefaults();
      setDraft({
        ...(defaults ?? defaultSettings()),
        // O idioma é escolha do usuário: restaurar padrões não o troca.
        ui_language: draft.ui_language ?? getUiLanguage(),
        shortcuts: defaultBindings(),
      });
      setHasUnsavedChanges(true);
      setStatus({
        kind: "info",
        text: t("settings_defaults_restored_click_save_changes_confirm",
        ),
      });
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
          ? t("settings_some_shortcuts_could_not_registered")
          : t("settings_shortcuts_registered_system"),
      });
    });

  return (
    <main className="app panel-shell settings-shell" data-route="settings">
      <ResizeHandles />
      <div className="settings">
        <WindowHeader
          className="settings__header"
          title={t("settings_settings")}
          subtitle={t("settings_customize_audio_capture_local_ai_privacy",
          )}
          icon={<TranscriptIcon size={18} />}
          hint={<kbd className="window-header__hint">Esc</kbd>}
          onStartDrag={() => void startWindowDrag()}
          onClose={() => void closeSettings()}
          closeLabel={t("settings_close_settings")}
          dragLabel={t("settings_move_settings")}
          headingLevel="h1"
        />

        <nav
          className="settings__tabs"
          role="tablist"
          aria-label={t("settings_sections")}
        >
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
                <span>{t(entry.label)}</span>
              </button>
            );
          })}
        </nav>

        <div className="settings__body">
          {tab === "sources" && (
            <>
              <SuggestionsActivationBlock settings={draft} update={update} />
              <AiSourcesSettings
                sources={draft.ai_sources}
                providers={providers}
                onChange={(sources) => update("ai_sources", sources)}
                onStatus={setStatus}
              />
            </>
          )}

          {tab === "transcription" && (
            <section className="settings__card">
              <header className="settings__card-header">
                <h2 className="settings__card-title">
                  {t("settings_transcription_audio")}
                </h2>
                <span className="settings__card-note">
                  {t("settings_local_model_acceleration_input_levels")}
                </span>
              </header>

              <label className="settings__field">
                <span>{t("settings_whisper_model_local")}</span>
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
                {t("settings_model_downloaded_only_start_first_session",
                )}
              </p>

              {gpu && (
                <p className="settings__hint" data-testid="gpu-hint">
                  {gpu.available
                    ? `${t("settings_gpu_detected_detail_suggested_model_model", {
                        detail: gpu.detail,
                        model: gpu.recommended_model,
                      })}${
                        gpu.acceleration_enabled
                          ? ""
                          : ` ${t("settings_build_still_transcribes_cpu_gpu_acceleration",
                            )}`
                      }`
                    : t("settings_no_gpu_detected_transcription_runs_cpu",
                      )}
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
                      onClick={() =>
                        update("whisper_model", gpu.recommended_model)
                      }
                    >
                      {t("settings_use_model_recommended_gpu", {
                        model: gpu.recommended_model,
                      })}
                    </button>
                  </div>
                )}

              {gpu &&
                !gpu.acceleration_enabled &&
                WHISPER_GPU_MODELS.includes(draft.whisper_model) && (
                  <p className="settings__hint">
                    {t("settings_model_requires_gpu_keep_up_speech",
                      { model: draft.whisper_model },
                    )}
                  </p>
                )}

              <label className="settings__field">
                <span>{t("settings_acceleration")}</span>
                <select
                  value={draft.acceleration}
                  onChange={(event) =>
                    update(
                      "acceleration",
                      event.target.value as AccelerationMode,
                    )
                  }
                >
                  {ACCELERATION_MODES.map((mode) => (
                    <option key={mode} value={mode}>
                      {t(accelerationKey(mode))}
                    </option>
                  ))}
                </select>
              </label>
              <p className="settings__hint">
                {gpu?.acceleration_enabled
                  ? gpu.available
                    ? t("settings_build_accelerates_gpu_vulkan_auto_uses",
                      )
                    : t("settings_build_supports_gpu_acceleration_but_no",
                      )
                  : t("settings_build_not_include_gpu_acceleration_transcription",
                    )}
              </p>

              <label className="settings__field">
                <span>{t("settings_transcription_context_optional")}</span>
                <textarea
                  rows={3}
                  placeholder={t("settings_e_g_grafana_kubernetes_team_names")}
                  value={draft.whisper_prompt}
                  onChange={(event) =>
                    update("whisper_prompt", event.target.value)
                  }
                />
              </label>
              <p className="settings__hint">
                {t("settings_use_carefully_only_context_names_acronyms",
                )}
              </p>

              <label className="settings__field settings__field--row">
                <input
                  type="checkbox"
                  checked={draft.aec_enabled}
                  onChange={(event) =>
                    update("aec_enabled", event.target.checked)
                  }
                />
                <span>{t("settings_cancel_echo_aec")}</span>
              </label>
              <p className="settings__hint">
                {t("settings_subtracts_speaker_audio_other_side_call",
                )}
              </p>

              <div className="settings__field">
                <span>{t("settings_microphone_sensitivity")}</span>
                <input
                  type="range"
                  aria-label={t("settings_microphone_sensitivity")}
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
                    ? t("settings_value_default", {
                        value: draft.mic_gain.toFixed(1),
                      })
                    : t("settings_value", { value: draft.mic_gain.toFixed(1) })}
                </span>
              </div>
              <p className="settings__hint">
                {t("settings_amplifies_microphone_before_transcription_system_audio",
                )}
              </p>

              <label className="settings__field">
                <span>{t("settings_source_language")}</span>
                <select
                  value={draft.source_language}
                  onChange={(event) =>
                    update("source_language", event.target.value as Language)
                  }
                >
                  {LANGUAGES.map((language) => (
                    <option key={language} value={language}>
                      {t(sourceLanguageKey(language))}
                    </option>
                  ))}
                </select>
              </label>

              <label className="settings__field">
                <span>{t("settings_target_language")}</span>
                <select
                  value={draft.target_language}
                  onChange={(event) =>
                    update("target_language", event.target.value as Language)
                  }
                >
                  {LANGUAGES.map((language) => (
                    <option key={language} value={language}>
                      {t(targetLanguageKey(language))}
                    </option>
                  ))}
                </select>
              </label>
              <p className="settings__hint">
                {t("settings_choose_source_language_note")}
              </p>

              <label className="settings__field settings__field--row">
                <input
                  type="checkbox"
                  checked={draft.translation_enabled}
                  onChange={(event) =>
                    update("translation_enabled", event.target.checked)
                  }
                />
                <span>{t("settings_live_translation")}</span>
              </label>

              <AudioTestPanel micGain={draft.mic_gain} />
            </section>
          )}

          {tab === "recording" && (
            <RecordingSettings settings={draft} update={update} />
          )}

          {tab === "assistant" && (
            <AssistantsSettings settings={draft} update={update} />
          )}

          {tab === "general" && (
            <>
              <LanguageSettings settings={draft} update={update} />
              <HudSettings settings={draft} update={update} />
              <CadenceSettings settings={draft} update={update} />
              <LogsSettings />
            </>
          )}

          {tab === "shortcuts" && (
            <section className="settings__card">
              <header className="settings__card-header">
                <h2 className="settings__card-title">
                  {t("settings_global_shortcuts")}
                </h2>
                <span className="settings__card-note">
                  {t("settings_operating_system_keyboard_shortcut")}
                </span>
              </header>
              <p className="settings__hint">
                {t("settings_button_below_registers_shortcuts_system_shortcut",
                )}
              </p>
              {SHORTCUT_ACTIONS.map((action) => {
                const info = catalog.find(
                  (entry) => entry.action === action,
                );
                const binding = draft.shortcuts.find(
                  (entry) => entry.action === action,
                );
                return (
                  <label
                    key={action}
                    className="settings__field settings__field--row settings__shortcut"
                  >
                    <span className="settings__shortcut-label">
                      {t(shortcutActionKey(action))}
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
                  {t("settings_apply_shortcuts")}
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
                ? t("settings_unsaved_changes")
                : t("settings_all_changes_synced_locally")}
            </span>
            {" · "}
            <span>
              {persistent
                ? t("settings_keys_system_keyring")
                : t("settings_keys_only_session")}
            </span>
          </span>
          <div className="settings__footer-actions">
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={onRestoreDefaults}
            >
              {t("settings_restore_defaults")}
            </button>
            <button
              type="button"
              className="settings__button settings__button--primary"
              disabled={busy}
              onClick={onSave}
            >
              {t("settings_save_changes")}
            </button>
          </div>
        </footer>
      </div>
    </main>
  );
}
