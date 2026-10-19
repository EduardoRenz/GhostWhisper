# Issues — PRD Notch

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser único
no repositório). Próximo ID livre: **ISS-036**.

> `ISS-032` segue reservada pelo PRD [`chat`](../chat/issues.md); esta PRD começa em
> `ISS-033` para não colidir.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-033 | Idioma da transcrição preso à sessão (troca só valia na próxima) | Média | FIXED |
| ISS-034 | Notch poluído em repouso e controles por texto (fora do padrão Perssua) | Baixa | FIXED |
| ISS-035 | Hover do drill-hover atravessa duas janelas e pode fechar sem querer | Média | FIXED |

---

## ISS-033 — Idioma da transcrição preso à sessão

- **Severidade:** Média · **Status:** FIXED ([`tasks/04-task.md`](tasks/04-task.md))
- **Sintoma:** trocar o "Idioma de origem" nas Configurações **não** muda a
  transcrição em andamento; só vale depois de encerrar e iniciar outra sessão.
- **Causa raiz:** o idioma faz parte da **chave de cache** do motor em
  `whisper_speech_engine_factory.rs` (`CachedEngine { model, language, use_gpu }`) e
  é um campo **imutável** em `WhisperSpeechToText` (`language: String`). Mudar o
  idioma nas settings não chega ao motor vivo: ou nada muda, ou seria preciso
  recriar o contexto (recarregar o modelo).
- **Correção (task 04):** `language` vira `Mutex<String>` com `update_language()`
  (espelho do `update_prompt` já existente); o idioma **sai da chave do cache** e é
  propagado ao motor vivo; novo IPC `session_set_language`. Vale para o **próximo
  passe** de decodificação (BR-034).
- **Arquivos:** `src-tauri/src/adapters/speech/whisper_speech_to_text.rs`,
  `src-tauri/src/adapters/speech/whisper_speech_engine_factory.rs`,
  `src-tauri/src/ports/speech_engine_factory.rs`,
  `src-tauri/src/ports/session_controller.rs`,
  `src-tauri/src/application/session_service.rs`,
  `src-tauri/src/ipc/session_commands.rs`.
- **Task:** [`tasks/04-task.md`](tasks/04-task.md)
- **Dano em dados existentes:** nenhum (o idioma é configuração; falas antigas já
  transcritas no idioma anterior permanecem).

---

## ISS-034 — Notch poluído em repouso e controles por texto

- **Severidade:** Baixa · **Status:** FIXED ([`tasks/01-task.md`](tasks/01-task.md),
  [`tasks/02-task.md`](tasks/02-task.md))
- **Sintoma:** sem sessão, o overlay ainda exibe rótulos de texto (`Chat`,
  `Histórico`, `Stealth: on/off`, `Ajustes`, `Ocultar`) e os controles de sessão,
  em vez de um notch compacto — diferente da referência de produto (BR-015).
- **Causa raiz:** a `ActionBar` foi construída com botões de texto e sem distinção
  entre o estado **repouso** e o **ativo**; não há pacote de ícones nem botão
  só-ícone.
- **Correção (tasks 01/02):** pacote de ícones SVG próprios, `IconButton` com
  `title`/`aria-label`, e o notch em dois estados com a ação primária verde e o
  atalho em badges (BR-032).
- **Arquivos:** `src/features/overlay/ActionBar.tsx`,
  `src/features/overlay/Overlay.tsx`, `src/components/icons.tsx`,
  `src/components/IconButton.tsx`, `src/components/ShortcutBadges.tsx`,
  `src/styles/globals.css`.
- **Task:** [`tasks/01-task.md`](tasks/01-task.md),
  [`tasks/02-task.md`](tasks/02-task.md)
- **Dano em dados existentes:** nenhum (apresentação).

---

## ISS-035 — Hover do drill-hover atravessa duas janelas

- **Severidade:** Média · **Status:** FIXED ([`tasks/10-task.md`](tasks/10-task.md))
- **Sintoma:** a confirmação é uma janela Tauri separada; ao mover o cursor do
  gatilho para ela, o `pointerleave` do notch pode fechar o popover antes de o
  `pointerenter` da janela `dwell` chegar.
- **Causa raiz:** são dois webviews nativos; a troca de janela gera leave/enter
  separados e não há um `hover` compartilhado.
- **Mitigação (task 10/11):** grace de `dwell_ms` após sair do gatilho **e** da
  confirmação; entrar em qualquer uma das áreas cancela o fechamento; o popover é
  posicionado com gap pequeno. Sem som.
- **Arquivos:** `src/features/dwell/DwellProvider.tsx`,
  `src/features/dwell/DwellPopoverWindow.tsx`,
  `src-tauri/src/adapters/dwell/tauri_dwell_popover.rs`.
- **Task:** [`tasks/10-task.md`](tasks/10-task.md), [`tasks/11-task.md`](tasks/11-task.md)
- **Dano em dados existentes:** nenhum.

---

<!--
Modelo:

## ISS-0NN — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`tasks/NN-task.md`](tasks/NN-task.md)
-->
