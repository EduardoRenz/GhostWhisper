/** Adaptador IPC de configurações e chaves de API. */
import { invokeSafe } from "./client";
import type { ShortcutConflict } from "../../core/domain/shortcuts";
import type { ProviderKind, Settings } from "../../core/domain/settings";

/** Lista os providers suportados. */
export const getProviders = (): Promise<ProviderKind[] | undefined> =>
  invokeSafe<ProviderKind[]>("settings_providers");

/** Carrega as configurações atuais. */
export const getSettings = (): Promise<Settings | undefined> =>
  invokeSafe<Settings>("settings_get");

/** Persiste as configurações; devolve conflitos de atalho encontrados. */
export const saveSettings = (
  settings: Settings,
): Promise<ShortcutConflict[] | undefined> =>
  invokeSafe<ShortcutConflict[]>("settings_set", { settings });

/** Grava a chave do provider ativo (nunca é devolvida). */
export const setProviderKey = (key: string): Promise<void | undefined> =>
  invokeSafe<void>("settings_set_provider_key", { key });

/** Indica se há chave configurada para o provider ativo. */
export const hasProviderKey = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("settings_has_provider_key");

/** Remove a chave do provider ativo. */
export const clearProviderKey = (): Promise<void | undefined> =>
  invokeSafe<void>("settings_clear_provider_key");

/** Testa a conexão com o provider ativo. */
export const validateKey = (): Promise<string | undefined> =>
  invokeSafe<string>("settings_validate_key");

/** Rótulo do provider/modelo ativo (sem segredos). */
export const providerLabel = (): Promise<string | undefined> =>
  invokeSafe<string>("settings_provider_label");

/** Versão mascarada da chave ativa (`••••••••·abcd`) ou "" quando ausente. */
export const keyPreview = (): Promise<string | undefined> =>
  invokeSafe<string>("settings_key_preview");

/** Indica se as chaves são persistidas no keyring ou só na sessão. */
export const secretsPersistent = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("settings_secrets_persistent");
