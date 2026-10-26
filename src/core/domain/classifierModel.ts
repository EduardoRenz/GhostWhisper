/**
 * Modelo do classificador (BR-099).
 *
 * O artefato ONNX é versionado e publicado fora do binário. A UI só mostra o
 * que o model manager do core reporta; nenhum download acontece sem confirmação
 * explícita.
 */

/** Um arquivo do artefato (manifesto). */
export interface ClassifierModelFile {
  path: string;
  url: string;
  size_bytes: number;
  sha256: string;
}

/** Calibração publicada com o checkpoint. */
export interface ClassifierModelCalibration {
  threshold?: number | null;
  ece?: number | null;
  notes?: string | null;
}

/** Métricas do eval publicado. */
export interface ClassifierModelMetrics {
  accuracy?: number | null;
  false_positive_rate?: number | null;
  p50_ms?: number | null;
  p95_ms?: number | null;
}

/** Manifesto de uma versão. */
export interface ClassifierModelManifest {
  schema: number;
  model_id: string;
  version: string;
  checkpoint: string;
  layout: string;
  label?: string | null;
  size_bytes: number;
  license: string;
  files: ClassifierModelFile[];
  calibration: ClassifierModelCalibration;
  metrics: ClassifierModelMetrics;
  created_at: string;
}

/** Catálogo publicado (`catalog.json`); o primeiro item é o recomendado. */
export interface ClassifierModelCatalog {
  schema: number;
  models: ClassifierModelManifest[];
}

/** Versão instalada no disco. */
export interface InstalledClassifierModel {
  model_id: string;
  version: string;
  checkpoint: string;
  path: string;
  size_bytes: number;
  active: boolean;
  verified: boolean;
  installed_at_ms: number;
}

/** Estado do engine Laya neste build. */
export interface ClassifierBackendStatus {
  /** O binário foi compilado com `--features laya`. */
  laya_feature: boolean;
  /** Há uma versão ativa no model manager. */
  model_active: boolean;
  /** Há override de dev, versão ativa ou modelo empacotado. */
  model_configured: boolean;
  /** O AppImage/deb traz o modelo nos resources. */
  model_bundled: boolean;
}

/** Fase do download/instalação. */
export type SuggestionModelDownloadPhase =
  | "downloading"
  | "verifying"
  | "installing"
  | "ready"
  | "failed";

/** Evento `suggestion_model_download` (BR-099). */
export interface SuggestionModelDownloadEvent {
  model_id: string;
  version: string;
  phase: SuggestionModelDownloadPhase;
  file: string | null;
  file_index: number;
  file_count: number;
  bytes_done: number;
  bytes_total: number;
  error: string | null;
}

/** Formata bytes em MB com uma casa (o modelo tem ~1,2 GB). */
export function formatModelSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const gigabytes = bytes / 1_000_000_000;
  if (gigabytes >= 1) return `${gigabytes.toFixed(1)} GB`;
  const megabytes = bytes / 1_000_000;
  return `${megabytes.toFixed(megabytes >= 10 ? 0 : 1)} MB`;
}
