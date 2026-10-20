/** Adaptador IPC das fontes de IA (BR-044, BR-045). */
import { invokeSafe } from "./client";
import type { AiSource, AiSourceHealth } from "../../core/domain/settings";

/** Lista as fontes cadastradas, na ordem de prioridade. */
export const listSources = (): Promise<AiSource[] | undefined> =>
  invokeSafe<AiSource[]>("ai_sources_list");

/** Informe de saúde de cada fonte. */
export const sourcesHealth = (): Promise<AiSourceHealth[] | undefined> =>
  invokeSafe<AiSourceHealth[]>("ai_sources_health");

/** Grava a chave da fonte no keyring (nunca é devolvida). */
export const setSourceKey = (
  sourceId: string,
  key: string,
): Promise<void | undefined> =>
  invokeSafe<void>("ai_source_set_key", { sourceId, key });

/** Indica se a fonte tem chave configurada. */
export const hasSourceKey = (sourceId: string): Promise<boolean | undefined> =>
  invokeSafe<boolean>("ai_source_has_key", { sourceId });

/** Remove a chave da fonte. */
export const clearSourceKey = (sourceId: string): Promise<void | undefined> =>
  invokeSafe<void>("ai_source_clear_key", { sourceId });

/** Versão mascarada da chave (`••••••••·abcd`) ou "" quando ausente. */
export const sourceKeyPreview = (sourceId: string): Promise<string | undefined> =>
  invokeSafe<string>("ai_source_key_preview", { sourceId });

/** Testa a conexão com a fonte ("Validar chave"). */
export const validateSource = (source: AiSource): Promise<string | undefined> =>
  invokeSafe<string>("ai_source_validate", { source });
