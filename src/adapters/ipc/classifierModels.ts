/** Adaptador IPC do model manager do classificador (BR-099). */
import { invokeSafe } from "./client";
import type {
  ClassifierBackendStatus,
  ClassifierModelCatalog,
  InstalledClassifierModel,
} from "../../core/domain/classifierModel";

/** Engine Laya neste build + modelo configurado. */
export const classifierBackendStatus = (): Promise<
  ClassifierBackendStatus | undefined
> => invokeSafe<ClassifierBackendStatus>("classifier_backend_status");

/** Catálogo publicado. Só lê metadados; não baixa o modelo. */
export const classifierModelCatalog = (): Promise<
  ClassifierModelCatalog | undefined
> => invokeSafe<ClassifierModelCatalog>("classifier_model_catalog");

/** Versões instaladas (com a flag de ativa). */
export const classifierModelInstalled = (): Promise<
  InstalledClassifierModel[] | undefined
> => invokeSafe<InstalledClassifierModel[]>("classifier_model_installed");

/** Versão ativa, se houver. */
export const classifierModelActive = (): Promise<
  InstalledClassifierModel | null | undefined
> => invokeSafe<InstalledClassifierModel | null>("classifier_model_active");

/** Baixa, verifica e ativa uma versão (ação explícita do usuário). */
export const classifierModelInstall = (
  modelId: string,
  version: string,
): Promise<InstalledClassifierModel | undefined> =>
  invokeSafe<InstalledClassifierModel>("classifier_model_install", {
    modelId,
    version,
  });

/** Ativa uma versão já instalada. */
export const classifierModelActivate = (
  modelId: string,
  version: string,
): Promise<InstalledClassifierModel | undefined> =>
  invokeSafe<InstalledClassifierModel>("classifier_model_activate", {
    modelId,
    version,
  });

/** Remove uma versão instalada (a ativa é protegida pelo core). */
export const classifierModelRemove = (
  modelId: string,
  version: string,
): Promise<void | undefined> =>
  invokeSafe<void>("classifier_model_remove", { modelId, version });
