import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SessionEnd } from "../../core/domain/recording";
import { sessionEndContext } from "../../adapters/ipc/sessionEnd";
import { recordingDiscard } from "../../adapters/ipc/recording";
import { SessionEndWindow } from "./SessionEndWindow";

vi.mock("../../adapters/ipc/sessionEnd", () => ({
  sessionEndContext: vi.fn(),
  sessionEndClose: vi.fn(async () => undefined),
  sessionEndOpen: vi.fn(),
}));

vi.mock("../../adapters/ipc/recording", () => ({
  recordingDiscard: vi.fn(async () => undefined),
}));

vi.mock("../../adapters/ipc/export", () => ({
  pickMarkdownPath: vi.fn(async () => "/tmp/reuniao.md"),
  saveSessionMarkdown: vi.fn(async () => "/tmp/reuniao.md"),
}));

vi.mock("../../adapters/ipc/session", () => ({
  sessionStart: vi.fn(async () => "s2"),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(async () => undefined),
}));

const BASE: SessionEnd = {
  session_id: "s1",
  subject: "Reunião",
  started_at_ms: 0,
  ended_at_ms: 872_000,
  duration_ms: 872_000,
  transcriptions: 18,
  suggestions: 4,
  recording: {
    kind: "video",
    status: "ready",
    path: "/tmp/reuniao.mp4",
    size_bytes: 214 * 1024 * 1024,
    duration_ms: 872_000,
    width: 1920,
    height: 1080,
    fps: 8,
    error: null,
  },
};

const context = vi.mocked(sessionEndContext);

describe("SessionEndWindow — modal de fim de sessão", () => {
  beforeEach(() => {
    context.mockReset();
    vi.mocked(recordingDiscard).mockClear();
  });

  it("mostra duração, mídia e itens processados", async () => {
    context.mockResolvedValue(BASE);
    render(<SessionEndWindow />);

    expect(await screen.findByText("Sessão Finalizada")).toBeDefined();
    expect(screen.getByText("14 min 32 seg")).toBeDefined();
    expect(screen.getByText(/214\.0 MB \(MP4/)).toBeDefined();
    expect(screen.getByText("18 falas · 4 sugestões IA")).toBeDefined();
    expect(
      (screen.getByRole("button", { name: /Abrir local/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(false);
  });

  it("sem gravação: cartão 'Sem gravação' e 'Abrir local' desabilitado", async () => {
    context.mockResolvedValue({ ...BASE, recording: null });
    render(<SessionEndWindow />);

    expect(await screen.findByText("Sem gravação")).toBeDefined();
    expect(
      (screen.getByRole("button", { name: /Abrir local/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    expect(
      (screen.getByRole("button", { name: /Descartar gravação/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });

  it("mostra o progresso enquanto finaliza", async () => {
    context.mockResolvedValue({
      ...BASE,
      recording: { ...BASE.recording!, status: "finalizing", path: null },
    });
    render(<SessionEndWindow />);

    expect(
      await screen.findByText(/Finalizando gravação…/),
    ).toBeDefined();
  });

  it("descartar exige confirmação em dois toques", async () => {
    context.mockResolvedValue(BASE);
    render(<SessionEndWindow />);

    const first = await screen.findByRole("button", {
      name: /Descartar gravação/,
    });
    fireEvent.click(first);
    expect(
      screen.getByRole("button", { name: /Confirmar descarte/ }),
    ).toBeDefined();
    expect(recordingDiscard).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: /Confirmar descarte/ }),
    );
    await waitFor(() => {
      expect(recordingDiscard).toHaveBeenCalledWith("s1");
    });
  });

  it("baixar a sessão usa o export Markdown do Histórico", async () => {
    const { saveSessionMarkdown } = await import("../../adapters/ipc/export");
    context.mockResolvedValue(BASE);
    render(<SessionEndWindow />);

    fireEvent.click(await screen.findByRole("button", { name: /Baixar Sessão/ }));
    await waitFor(() => {
      expect(vi.mocked(saveSessionMarkdown)).toHaveBeenCalledWith(
        "s1",
        "/tmp/reuniao.md",
      );
    });
  });
});
