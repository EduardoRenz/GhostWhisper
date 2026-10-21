import { describe, expect, it } from "vitest";

import {
  estimateSizePerHour,
  formatBytes,
  formatDuration,
  mediaLabel,
  type RecordingSummary,
} from "./recording";
import { defaultSettings } from "./settings";

const VIDEO: RecordingSummary = {
  kind: "video",
  status: "ready",
  path: "/tmp/reuniao.mp4",
  segments_dir: null,
  size_bytes: 214 * 1024 * 1024,
  duration_ms: 872_000,
  width: 1920,
  height: 1080,
  fps: 8,
  error: null,
};

describe("gravação — formatação", () => {
  it("mostra a duração no formato do mockup", () => {
    expect(formatDuration(14 * 60_000 + 32_000)).toBe("14 min 32 seg");
    expect(formatDuration(3_600_000)).toBe("1 h 00 min");
    expect(formatDuration(45_000)).toBe("45 seg");
  });

  it("formata bytes em MB/GB", () => {
    expect(formatBytes(28.4 * 1024 * 1024)).toBe("28.4 MB");
    expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe("2.00 GB");
    expect(formatBytes(0)).toBe("0 MB");
  });

  it("descreve a mídia com o formato e as specs", () => {
    expect(mediaLabel(VIDEO)).toBe("214.0 MB (MP4 · 1080p · 8 fps)");
    expect(
      mediaLabel({ ...VIDEO, kind: "audio", status: "ready" }),
    ).toBe("214.0 MB (FLAC)");
    expect(mediaLabel(null)).toBe("Sem gravação");
    expect(mediaLabel({ ...VIDEO, status: "discarded" })).toBe("Sem gravação");
    expect(mediaLabel({ ...VIDEO, status: "failed", path: null })).toBe(
      "Falhou (segmentos preservados)",
    );
  });

  it("estima o custo por hora a partir das settings", () => {
    const settings = defaultSettings();
    const estimate = estimateSizePerHour(settings);
    expect(estimate.min).toBeGreaterThan(0);
    expect(estimate.max).toBeGreaterThan(estimate.min);
    // 8 fps equilibrado + 96 kbps fica na faixa de reunião típica.
    expect(estimate.max).toBeLessThan(250);

    const audio = estimateSizePerHour({ ...settings, recording_mode: "audio" });
    expect(audio).toEqual({ min: 117, max: 117 });
  });

  it("mais FPS custa mais", () => {
    const settings = defaultSettings();
    const slow = estimateSizePerHour({ ...settings, recording_fps: 5 });
    const fast = estimateSizePerHour({ ...settings, recording_fps: 30 });
    expect(fast.max).toBeGreaterThan(slow.max);
  });
});
