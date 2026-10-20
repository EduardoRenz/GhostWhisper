# 06 — Captura de tela: porta `ScreenCapture` e adaptador X11

- **Status:** PENDING
- **Depende de:** —
- **Habilita:** 07
- **Issues:** — · **BR:** BR-007, BR-053, BR-055
- **Refs:** [`../prd.md`](../prd.md) US-03;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §4.2, §5.2

## Objetivo

Capturar a **tela** no Linux (X11/XWayland) e anexar a imagem à pergunta —
com o caminho de **colar imagem** como alternativa (funciona em Wayland).

## Entregáveis

- `src-tauri/src/ports/screen_capture.rs`:
  - trait `ScreenCapture { fn capture(&self) -> Result<CapturedImage>; }`;
  - `CapturedImage { mime, data, width, height }`.
- `src-tauri/src/adapters/screen/x11_screen_capture.rs`:
  - `x11rb` → `GetImage` na **root window** (`ZPixmap`);
  - esconde as janelas do app durante a captura e restaura depois (mesmo
    cuidado do Perssua, `protectPerssuaWindowsForCapture`);
  - reescala para **≤1280 px** no lado maior e recomprime em **JPEG q0.86**
    (paridade com o preset `balanced` do Perssua);
  - dobra: `FakeScreenCapture`.
- `src-tauri/src/ipc/screen_commands.rs`: `screen_capture`.
- `src-tauri/src/lib.rs`: registrar o adaptador no composition root.
- `src/adapters/ipc/screen.ts`: `screenCapture`.
- `src/features/journal/AskComposer.tsx`:
  - botão de **print** (ícone de câmera) → captura e mostra a **miniatura**
    anexada, com botão `×` para remover;
  - **colar imagem** (`Ctrl+V`/`Cmd+V` com foco no campo) → anexa a imagem da
    área de transferência;
  - **1 imagem por pergunta** — pedir uma segunda mostra "Só uma imagem por
    pergunta".

## Critérios de aceite

- [ ] Em X11, o print devolve um JPEG **≤1280 px** com qualidade 0.86.
- [ ] As janelas do app **não** aparecem no print (escondidas durante a
      capture) e voltam depois.
- [ ] Sem display X11, o erro é **explícito** ("Captura de tela indisponível
      neste ambiente — cole a imagem com Ctrl+V") e **nada** falha em
      silêncio.
- [ ] Colar imagem anexa a imagem do clipboard.
- [ ] A segunda imagem é recusada com aviso.
- [ ] A miniatura pode ser removida antes do envio.
- [ ] `cargo test --lib` verde (o `FakeScreenCapture` cobre o caminho
      testável; o X11 real é verificado à mão).

## Notas

- **Wayland nativo não é suportado** (BR-007): o caminho é X11/XWayland +
  colar imagem. É o desvio consciente (R12) — o Perssua usa
  `desktopCapturer` do Electron, que não existe no Tauri.
- O redimensionamento/recompressão fica num módulo **puro** (função que
  recebe bytes e devolve bytes), para ser testável sem display.
- Nada é persistido ainda aqui: a imagem vai para o provider e (na task 01)
  para `journal_images` quando a pergunta é enviada.
