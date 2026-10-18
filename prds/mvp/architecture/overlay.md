# Arquitetura — Overlay (janela)

Detalha `overlay/*` e a interação com a UI para a janela sempre-no-topo,
transparente, discreta e click-through.

---

## 1. Requisitos da janela

- Transparente, sem moldura (`decorations: false`).
- `always_on_top: true`, `skip_taskbar` (modo stealth).
- Não roubar foco ao interagir.
- Posicionável; por padrão, centralizada/ancorada na base do monitor escolhido.
- Click-through nas regiões não interativas.

## 2. Configuração (Tauri)

`tauri.conf.json` define a janela `overlay`:

```json
{
  "label": "overlay",
  "transparent": true,
  "decorations": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "focus": false,
  "width": 900,
  "height": 260
}
```

> Transparência exige compositor ativo. Sem compositor, degradar para fundo opaco
> e registrar aviso (R-04).

## 3. Click-through dinâmico

O protocolo gráfico Linux não permite "furar" o clique por região via API de
janela, mas o **X11** expõe a extensão **SHAPE** (`ShapeRectangles` com
`kind=Input`), que define a região de entrada da janela:

1. A UI (React) marca regiões interativas e envia ao core os *rects* clicáveis.
2. O core (`adapters/overlay/x11_input_shape_click_through.rs`) obtém o `xid` da
   janela (`raw-window-handle`) e aplica o input shape = **união dos rets**
   (físicos = CSS × scale). Fora deles o clique atravessa; dentro, a janela é
   interativa **de imediato** — sem polling e sem corrida (ISS-001).
3. Regiões vazias (antes de a UI publicar) → janela inteira interativa, para não
   deixar o overlay "morto".

- **Fallback** (`adapters/overlay/cursor_click_through.rs`): quando não há X11
  (Wayland puro) ou SHAPE, usa a alternância global com polling do cursor +
  histerese (`ClickThroughLatche`) e margem de conforto.
- No Wayland o polling global de cursor é restrito → comportamento degradado
  (documentado). Foco do MVP é X11/XWayland (BR-007).

## 4. Discretude ("stealth") — revisado na task 19

- `set_skip_taskbar(true)` e título vazio quando stealth ativo, **reaplicados** em
  `show()` e ao trocar de superfície (redimensionar remapeia a janela e alguns
  compositores readicionam a barra de tarefas).
- Por paridade com o Perssua (`setContentProtection`), também chamamos
  `set_content_protected` — que no **Linux é no-op** (a API só tem efeito no
  macOS/Windows). Não somos piores que a referência de produto; também não estamos
  prometendo o que o Linux não faz.
- **Invisibilidade no compartilhamento de tela não é garantida** (BR-006). Não
  existe propriedade de janela que a exclua da captura no X11/XWayland: o Chrome
  (Meet) captura por XComposite/portal PipeWire e o overlay entra na imagem.
  - **Único caminho real no Linux:** KDE Plasma **6.6+** ("Hide from Screencast";
    regra de janela KWin "Exclude from screencast" e `PlasmaWindowInterface`
    `excludeFromCaptureRequested`). Por compositor — opt-in futuro.
  - **Mitigações de uso** (documentadas na UI e no README): compartilhar **uma
    janela** na reunião (o overlay fica de fora por não ser a fonte) ou mover o
    overlay para um **monitor que não está sendo compartilhado**.
- Atalho global para mostrar/ocultar; o spike da T10 avaliou (a) overlay em
  **output virtual dedicado** (só wlroots) e (b) alinhamento com o portal de
  captura — ambos descartados para o MVP.

## 5. Posicionamento e multi-monitor

- Seleção de monitor nas settings (`selected_output`).
- Reancorar após mudança de resolução/monitor (evento do Tauri/Winit).
- Bounds persistidos nas settings.

## 6. Alternância de superfície

- **Estados:** `hidden`, `actionbar` (barra fina), `expanded` (transcrição/insights).
- A UI alterna via evento; o core ajusta tamanho/posição da janela.
- **Arraste:** a ActionBar tem um handle (`overlay_start_drag`) que chama
  `start_dragging()`; após o usuário mover, a janela **para de reancorar** pela
  base e a posição é **persistida** (`settings.overlay_position`) e restaurada no
  próximo boot.

## 7. Arquivos envolvidos

- Novos: `ports/overlay_window.rs`,
  `adapters/overlay/{tauri_overlay_window,cursor_click_through}.rs`.
- UI: `src/features/overlay/{Overlay,ActionBar}.tsx`.
- Modificados: `tauri.conf.json`, `lib.rs`, `ipc/mod.rs`, `state.rs`.
