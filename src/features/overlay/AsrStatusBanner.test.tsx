import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AsrStatusBanner } from "./AsrStatusBanner";

describe("AsrStatusBanner", () => {
  it("não renderiza sem status", () => {
    const { container } = render(<AsrStatusBanner status={null} />);
    expect(container.firstChild).toBeNull();
  });

  it("some quando o modelo está pronto", () => {
    const { container } = render(
      <AsrStatusBanner
        status={{ phase: "ready", model: "tiny", message: "Modelo tiny pronto" }}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("mostra o aviso de download com modelo e tamanho", () => {
    render(
      <AsrStatusBanner
        status={{
          phase: "preparing",
          model: "medium",
          message: "Preparando modelo medium… (pode baixar ~1,5 GB)",
        }}
      />,
    );
    expect(screen.getByRole("status").textContent).toContain("1,5 GB");
  });

  it("destaca falha", () => {
    const { container } = render(
      <AsrStatusBanner
        status={{
          phase: "failed",
          model: "medium",
          message: "falha ao carregar whisper",
        }}
      />,
    );
    expect(container.querySelector(".asr-status--failed")).not.toBeNull();
  });
});