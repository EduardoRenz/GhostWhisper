/** Adaptador IPC de configurações e chaves de API. */
import { invokeSafe } from "./client";
import type { ShortcutConflict } from "../../core/domain/shortcuts";
import type { ProviderKind, Settings } from "../../core/domain/settings";

/** Evento de preview da opacidade global (BR-062). */
export const HUD_OPACITY_PREVIEW_EVENT = "hud_opacity_preview";

/** Lista os providers suportados. */
export const getProviders = (): Promise<ProviderKind[] | undefined> =>
  invokeSafe<ProviderKind[]>("settings_providers");

/** Esconde a janela de Configurações. */
export const closeSettings = (): Promise<void | undefined> =>
  invokeSafe<void>("settings_close");

/** Carrega as configurações atuais. */
export const getSettings = (): Promise<Settings | undefined> =>
  invokeSafe<Settings>("settings_get");

/** Persiste as configurações; devolve conflitos de atalho encontrados. */
export const saveSettings = (
  settings: Settings,
): Promise<ShortcutConflict[] | undefined> =>
  invokeSafe<ShortcutConflict[]>("settings_set", { settings });

/** Publica o preview da opacidade global antes do autosave. */
export const previewOverlayOpacity = (opacity: number): Promise<void | undefined> =>
  invokeSafe<void>("settings_preview_overlay_opacity", { opacity });

/** Autosave da opacidade global (não mexe nos demais campos). */
export const setOverlayOpacity = (opacity: number): Promise<void | undefined> =>
  invokeSafe<void>("settings_set_overlay_opacity", { opacity });

/** Testa a conexão com a fonte primária (validar chave). */
export const validateKey = (): Promise<string | undefined> =>
  invokeSafe<string>("settings_validate_key");

/** Rótulo do provider/modelo ativo (sem segredos). */
export const providerLabel = (): Promise<string | undefined> =>
  invokeSafe<string>("settings_provider_label");

/** Indica se as chaves são persistidas no keyring ou só na sessão. */
export const secretsPersistent = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("settings_secrets_persistent");

/** GPU detectada e modelo local sugerido para este hardware (BR-027). */
export interface GpuInfo {
  available: boolean;
  backend: string;
  detail: string;
  /** O build atual acelera por GPU? (chega no entregável E3). */
  acceleration_enabled: boolean;
  recommended_model: string;
}

/** Detecta a GPU local (sugestão de modelo; não altera configurações). */
export const gpuInfo = (): Promise<GpuInfo | undefined> =>
  invokeSafe<GpuInfo>("settings_gpu_info");
