import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { IconButton } from "./IconButton";
import { GearIcon } from "./icons";

describe("IconButton", () => {
  it("expõe o nome acessível e a dica de ferramenta", () => {
    render(
      <IconButton label="Configurações" onClick={() => {}}>
        <GearIcon />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Configurações" });
    expect(button.getAttribute("title")).toBe("Configurações");
    expect(button.getAttribute("data-interactive")).not.toBeNull();
  });

  it("usa um title próprio quando informado", () => {
    render(
      <IconButton label="Configurações" title="Abrir ajustes" onClick={() => {}}>
        <GearIcon />
      </IconButton>,
    );
    expect(
      screen.getByRole("button", { name: "Configurações" }).getAttribute("title"),
    ).toBe("Abrir ajustes");
  });

  it("aciona o clique", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Ocultar" onClick={onClick}>
        <GearIcon />
      </IconButton>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Ocultar" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("marca o estado ligado e o aria-pressed de alternância", () => {
    render(
      <IconButton label="Transcrição" active pressed onClick={() => {}}>
        <GearIcon />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Transcrição" });
    expect(button.className).toContain("icon-button--on");
    expect(button.getAttribute("aria-pressed")).toBe("true");
  });

  it("desabilitado não dispara o clique", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Configurações" disabled onClick={onClick}>
        <GearIcon />
      </IconButton>,
    );
    const button = screen.getByRole("button", {
      name: "Configurações",
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });
});
