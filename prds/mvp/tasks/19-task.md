# 19 — Revisão do modo stealth (e o limite real no Linux)

- **Status:** CONCLUDED
- **Depende de:** 09, 10, 16
- **Habilita:** —

## Objetivo
Responder ao usuário — *"testei no Google Meeting, mas o overlay aparece lá; não foi
implementado ou o desafio é maior?"* — e entregar o que **é possível** no Linux.

**Veredito: o desafio é maior, não é um bug nosso.** O "modo stealth" do GhostWhisper
já faz o que o Perssua faz no Linux: sai da barra de tarefas e do alt-tab. O
invisibilidade na captura é que não existe como API no Linux.

## Evidências (REA + pesquisa)
- **Perssua 0.27.5** (`electron/ipc/window-management/stealthModeApplication.js`):
  `setContentProtection(true)` + `setSkipTaskbar(true)` + `setTitle("")`. O
  `setContentProtection` é **no-op no Linux** (a API do Electron só tem efeito no
  macOS/Windows) — a própria memória do kickoff já registrava isso. Ou seja:
  **não somos piores que a referência de produto** no Linux.
- **X11/XWayland**: o Chrome (Meet) captura por XComposite/portal PipeWire; a
  janela é composta na tela inteira. Não há propriedade que a exclua da captura —
  nada equivalente a `WDA_EXCLUDEFROMCAPTURE` (Windows) ou `NSWindowSharingNone`
  (macOS).
- **KDE Plasma 6.6+**: ganhou "Hide from Screencast" (*Exclude from capture*),
  exposto como regra de janela ("Exclude from screencast", KWin MR !8828) e via
  DBus (`PlasmaWindowInterface::excludeFromCaptureRequested`, MR !8442/!9171). É o
  **único caminho real** no Linux e é por compositor (KWin).
- Sessão do autor: `XDG_SESSION_TYPE=x11`, GNOME — nenhum desses caminhos existe.

## Entregáveis
- **Paridade**: `TauriOverlayWindow::set_stealth` passou a chamar também
  `set_content_protected` (a API do Tauri existe; no Linux é no-op, mas o
  comportamento fica correto se um dia formos para macOS/Windows).
- **Reafirmação**: o estado desejado do stealth é guardado e **reaplicado** em
  `show()` e em `set_surface()` (redimensionar remapeia a janela e alguns
  compositores readicionam barra de tarefas/título) — transposto de
  `scheduleOverlayTaskbarStateReassertion`.
- **Copy honesta**: tooltip do botão na ActionBar e aviso na aba *Geral* das
  configurações: o stealth **não** esconde o overlay do compartilhamento, com as
  saídas reais (compartilhar **uma janela** no Meet, ou overlay em outro monitor).
- **Documentação**: `architecture/overlay.md` §4 reescrito, `wayland-spike.md` com
  o achado do KDE 6.6, README com a limitação e as alternativas, **BR-006**
  atualizada.

## Critérios de aceite
- [x] Nenhuma promessa de invisibilidade (BR-006 mantida e explicitada na UI).
- [x] Stealth reaplicado quando a janela reaparece ou muda de superfície.
- [x] Paridade com o Perssua (`set_content_protected`), documentada como no-op no
      Linux.
- [x] Caminhos realmente possíveis documentados (janela específica no Meet, 2º
      monitor, KDE 6.6 como opt-in futuro).
- [x] `cargo test`/`clippy`/`fmt` verdes; nenhum teste dependente de comportamento
      gráfico não determinístico.

## Pendências / futuro
- **Opt-in KDE**: pedir a exclusão via `PlasmaWindowInterface` (DBus) quando
  KWin ≥ 6.6 — exige dependência DBus e detecção do compositor; só KDE.
- **Output virtual dedicado** (wlroots) continua fora de escopo (não cobre
  GNOME/KDE/X11).
- Teste manual em GNOME/KDE X11 com Meet/Jitsi: documentar o comportamento real
  observado (compartilhar janela × tela inteira).
