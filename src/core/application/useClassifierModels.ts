import { useCallback, useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  classifierBackendStatus,
  classifierModelActivate,
  classifierModelActive,
  classifierModelCatalog,
  classifierModelInstall,
  classifierModelInstalled,
  classifierModelRemove,
} from "../../adapters/ipc/classifierModels";
import { formatModelSize } from "../domain/classifierModel";
import type { Translate } from "../l10n/labels";
import type {
  ClassifierBackendStatus,
  ClassifierModelCatalog,
  ClassifierModelManifest,
  InstalledClassifierModel,
  SuggestionModelDownloadEvent,
} from "../domain/classifierModel";

export interface ClassifierModelsView {
  catalog: ClassifierModelCatalog | null;
  /** Engine Laya neste build (feature/modelo). */
  backend: ClassifierBackendStatus | null;
  /** Primeiro modelo do catálogo (recomendado). */
  recommended: ClassifierModelManifest | null;
  installed: InstalledClassifierModel[];
  active: InstalledClassifierModel | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  progress: SuggestionModelDownloadEvent | null;
  refresh: () => Promise<void>;
  install: (modelId: string, version: string) => Promise<void>;
  activate: (modelId: string, version: string) => Promise<void>;
  remove: (modelId: string, version: string) => Promise<void>;
}

/**
 * Estado do model manager do classificador (BR-099).
 *
 * O core é a fonte da verdade: a UI nunca calcula checksum/versão nem baixa
 * nada por conta própria. O download só começa em `install`, chamado após a
 * confirmação explícita do usuário.
 */
export function useClassifierModels(): ClassifierModelsView {
  const [catalog, setCatalog] = useState<ClassifierModelCatalog | null>(null);
  const [backend, setBackend] = useState<ClassifierBackendStatus | null>(null);
  const [installed, setInstalled] = useState<InstalledClassifierModel[]>([]);
  const [active, setActive] = useState<InstalledClassifierModel | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<SuggestionModelDownloadEvent | null>(
    null,
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const nextBackend = await classifierBackendStatus();

      // Build sem o engine: não adianta bater no catálogo (a URL oficial é um
      // placeholder e o erro de DNS só confundiria). Ainda listamos o que já
      // está instalado no disco.
      if (nextBackend && !nextBackend.laya_feature) {
        const [nextInstalled, nextActive] = await Promise.all([
          classifierModelInstalled(),
          classifierModelActive(),
        ]);
        setBackend(nextBackend);
        setCatalog(null);
        setInstalled(nextInstalled ?? []);
        setActive(nextActive ?? null);
        setError(null);
        return;
      }

      if (nextBackend?.model_bundled) {
        const [nextInstalled, nextActive] = await Promise.all([
          classifierModelInstalled(),
          classifierModelActive(),
        ]);
        // O modelo já vai no app; o catálogo é só atualização. Um catálogo
        // placeholder/offline não deve virar erro vermelho.
        let nextCatalog: ClassifierModelCatalog | null = null;
        try {
          nextCatalog = (await classifierModelCatalog()) ?? null;
        } catch {
          nextCatalog = null;
        }
        setBackend(nextBackend);
        setCatalog(nextCatalog);
        setInstalled(nextInstalled ?? []);
        setActive(nextActive ?? null);
        setError(null);
        return;
      }

      const [nextCatalog, nextInstalled, nextActive] = await Promise.all([
        classifierModelCatalog(),
        classifierModelInstalled(),
        classifierModelActive(),
      ]);
      // Fora do Tauri os adaptadores devolvem `undefined`: estado vazio, sem
      // erro (permite desenvolver a UI no navegador).
      setCatalog(nextCatalog ?? null);
      setInstalled(nextInstalled ?? []);
      setActive(nextActive ?? null);
      setBackend(nextBackend ?? null);
      setError(null);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();

    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void subscribe<SuggestionModelDownloadEvent>(
      "suggestion_model_download",
      (event) => {
        if (cancelled || !event) return;
        setProgress(event);
        if (event.phase === "ready") {
          void refresh();
        }
      },
    ).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unlisten = stop;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refresh]);

  const run = useCallback(
    async (action: () => Promise<unknown>) => {
      setBusy(true);
      setError(null);
      try {
        await action();
        await refresh();
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  const install = useCallback(
    (modelId: string, version: string) =>
      run(async () => {
        setProgress({
          model_id: modelId,
          version,
          phase: "downloading",
          file: null,
          file_index: 0,
          file_count: 0,
          bytes_done: 0,
          bytes_total: 0,
          error: null,
        });
        await classifierModelInstall(modelId, version);
      }),
    [run],
  );

  const activate = useCallback(
    (modelId: string, version: string) =>
      run(() => classifierModelActivate(modelId, version)),
    [run],
  );

  const remove = useCallback(
    (modelId: string, version: string) =>
      run(() => classifierModelRemove(modelId, version)),
    [run],
  );

  return {
    catalog,
    backend,
    recommended: catalog?.models[0] ?? null,
    installed,
    active,
    loading,
    busy,
    error,
    progress,
    refresh,
    install,
    activate,
    remove,
  };
}

/** Texto curto do progresso (usado pela UI e pelos testes). */
export function progressLabel(
  event: SuggestionModelDownloadEvent | null,
  t: Translate,
): string | null {
  if (!event) return null;
  const file = event.file ?? "";
  switch (event.phase) {
    case "downloading":
      return t("classifier_model_downloading", {
        file,
        index: event.file_index,
        count: event.file_count,
      });
    case "verifying":
      return t("classifier_model_verifying", { file });
    case "installing":
      return t("classifier_model_installing");
    case "ready":
      return t("classifier_model_ready");
    case "failed":
      return t("classifier_model_failed", {
        error: event.error ?? "",
      });
  }
}

/** Percentual 0–100 quando o tamanho total é conhecido. */
export function progressPercent(
  event: SuggestionModelDownloadEvent | null,
): number | null {
  if (!event || event.bytes_total <= 0) return null;
  const ratio = event.bytes_done / event.bytes_total;
  return Math.min(100, Math.max(0, Math.round(ratio * 100)));
}

/** Resumo do download para a confirmação (tamanho do modelo). */
export function modelDownloadSummary(
  model: ClassifierModelManifest | null,
): string | null {
  if (!model) return null;
  const size = model.size_bytes > 0 ? formatModelSize(model.size_bytes) : null;
  return size;
}
