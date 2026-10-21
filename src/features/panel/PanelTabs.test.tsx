import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PanelTabs } from "./PanelTabs";

const idle = { microphone: false, system: false };

const tabs = () => ({
  transcription: screen.getByRole("tab", { name: "Transcrição" }),
  summary: screen.getByRole("tab", { name: "Resumo" }),
  session: screen.getByRole("tab", { name: "Sessão" }),
});

describe("PanelTabs — sinal de transcrição", () => {
  it("mantém a aba Transcrição ociosa sem classes de streaming", () => {
    render(
      <PanelTabs
        active="session"
        onChange={() => {}}
        transcriptionActivity={idle}
      />,
    );
    const { transcription, summary, session } = tabs();
    expect(transcription.className).not.toContain("--streaming");
    expect(transcription.getAttribute("title")).toBe("Transcrição em silêncio");
    expect(transcription.querySelector(".panel-tabs__status-dot")).not.toBeNull();
    // O efeito é só na aba Transcrição.
    expect(summary.className).not.toContain("--streaming");
    expect(session.className).not.toContain("--streaming");
  });

  it("aplica o sweep do microfone e o do Outros ao mesmo tempo", () => {
    render(
      <PanelTabs
        active="session"
        onChange={() => {}}
        transcriptionActivity={{ microphone: true, system: true }}
      />,
    );
    const transcription = tabs().transcription;
    expect(transcription.className).toContain("panel-tabs__tab--streaming-mic");
    expect(transcription.className).toContain(
      "panel-tabs__tab--streaming-system",
    );
    expect(transcription.getAttribute("title")).toBe(
      "Transcrição processando microfone e Outros",
    );
  });

  it("troca de aba ao clicar (o botão existente continua sendo a aba)", () => {
    const onChange = vi.fn();
    render(
      <PanelTabs
        active="session"
        onChange={onChange}
        transcriptionActivity={idle}
      />,
    );
    fireEvent.click(tabs().transcription);
    expect(onChange).toHaveBeenCalledWith("transcription");
  });
});
