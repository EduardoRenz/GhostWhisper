# Spike — Wayland nativo e "invisibilidade" no compartilhamento de tela

- **Data:** 2026-10-17 (Tarefa 10)
- **Ambiente avaliado:** Ubuntu 24.04 x64, sessão **X11** (`loginctl Type=x11`),
  GNOME Shell; sem compositor Wayland ativo.
- **Relaciona-se a:** BR-006 (privacidade), BR-007 (X11/XWayland no MVP),
  `prds/mvp/architecture/overlay.md`.

## Objetivo
Decidir (go/no-go) se o MVP deve (a) suportar Wayland nativo e (b) prometer
invisibilidade no compartilhamento de tela — e registrar riscos e caminhos.

## Evidências coletadas
| Item | Observação |
|------|-----------|
| Tipo de sessão | `loginctl` → `Type=x11`; `WAYLAND_DISPLAY` vazio; `DISPLAY=:0` |
| Compositor | `gnome-shell` presente (sessão X11) |
| `xdg-desktop-portal` | 1.18.4 (+ backends `-gnome` 46.2, `-gtk` 1.15.1) |
| `gtk-layer-shell` | **ausente** (sem lib/pkg-config) |
| Compositores wlroots | ausentes (`sway`/`weston` não instalados) |
| Ferramentas de output virtual | ausentes (`wlr-randr`, `wayvnc`, `cage`) |
| Click-through atual | polling global do cursor via X11 (`x11rb` / `XQueryPointer`) |
| Atalhos globais | `tauri-plugin-global-shortcut` → `global-hotkey` (X11 `XGrabKey`) |
| Empacotamento | `.deb` 7,9 MiB e AppImage 85,6 MiB gerados; release roda |

## Análise

### 1. Wayland nativo com `gtk-layer-shell`
- Ancorar o overlay (topo/base, sempre-no-topo) em Wayland exige
  `wlr-layer-shell` via **`gtk-layer-shell`**, que **não está disponível** e
  adicionaria dependência de runtime por compositor.
- **Click-through por região não existe** no protocolo Wayland: nosso desenho
  depende de **ler a posição global do cursor**, que o Wayland **não expõe** a
  clientes (modelo de segurança). O `set_ignore_cursor_events` global + polling
  X11 é o que faz o ISS-001 funcionar hoje.
- **Atalhos globais** em Wayland não passam por `XGrabKey`; exigem o portal
  `GlobalShortcuts` ou APIs por desktop. O plugin atual não cobre isso.

### 2. Invisibilidade via output virtual dedicado
- Renderizar o overlay num **monitor virtual** e capturar o monitor real
  funcionaria apenas em compositores que exponham outputs virtuais
  (wlroots: `wlr-randr`); **GNOME/KDE não expõem** esse caminho.
- Não é portável e não resolve X11, onde a captura normalmente inclui a janela.
- O `xdg-desktop-portal` (ScreenCast/PipeWire) permite **escolher fontes**, mas
  não oferece exclusão confiável de janelas específicas de forma transversal.

### 3. Limites do portal
- `xdg-desktop-portal` + PipeWire permitem capturar tela/janela com consentimento
  do usuário; **não** há API para "esconder esta janela da captura".
- Portanto **invisibilidade não é garantida** (BR-006) — nem em X11 nem Wayland.

## Decisão

| Questão | Decisão |
|---------|---------|
| Wayland **nativo** no MVP | **NO-GO** — manter **X11/XWayland** (BR-007) |
| Invisibilidade no compartilhamento | **NO-GO** — não prometer; documentar limite (BR-006) |
| Output virtual dedicado | **Adiar** — opt-in futuro, apenas em wlroots |

Sob Wayland, o app roda via **XWayland** (comportamento X11), com as ressalvas de
atalhos/cursor impostas pelo compositor. A degradação deve ser registrada em log
(como já ocorre quando o X11 não está disponível).

## Riscos e caminhos futuros
- **Camada:** `gtk-layer-shell` + `wlr-layer-shell` para ancoragem/always-on-top.
- **Entrada:** `libei` / portal `RemoteDesktop` para posição de cursor e
  click-through sem polling X11.
- **Atalhos:** portal `GlobalShortcuts` (com fallback por desktop).
- **Privacidade:** nunca prometer invisibilidade; oferecer, no máximo, "output
  virtual dedicado" como recurso opt-in e restrito a wlroots.

## Consequências para o MVP
- Alvos de empacotamento e instruções assumem X11/XWayland.
- O README e a tela de privacidade mantêm o aviso de que a exclusão do
  compartilhamento **não é garantida**.
