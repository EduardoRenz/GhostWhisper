# 09 — Transporte: toggle único (start/pause) e stop único

- **Status:** PENDING
- **Depende de:** 03, 04
- **Habilita:** —

## Objetivo

Ajustar o transporte à referência do Perssua, eliminando o stop duplicado: os três
botões `[▶][⏸][⏹]` viram **um** controle que alterna start/pause/retomar (um
"switch"), e o **único** stop passa a ser a pílula vermelha de sessão, que já tem o
tempo decorrido e a confirmação "Encerrar sessão? Sim / Não".

Evidência no Perssua (`5684.18f69cc0.chunk.js`, `unified-session-controls`): o
controle é `[play_arrow ↔ pause]` (um botão cujo ícone alterna) **mais** `[stop]`;
a pílula de sessão (`.actionbar-session-pill`) é quem pede o "End session?". Isso
substitui o desvio consciente registrado na 03/04 (dois botões play/pause e o ⏹
também confirmando).

## Entregáveis

- `src/features/session/SessionControls.tsx`: os `[▶][⏸][⏹]` viram um **único**
  controle — parado → `▶ Iniciar escuta` (inicia com os canais escolhidos);
  gravando → `⏸` (pausa); pausado → `▶` (retoma). Um clique = start/pause/resume.
- `SessionControls.tsx`/`ActionBar.tsx`: o `⏹` do transporte sai; encerrar passa a
  ser só pela pílula `SessionPill` (tempo + confirmação Sim/Não).
- `src/styles/globals.css`: ajusta o controle único; remove `.transport__button
  --stop` e as sobras da pílula de três ícones. `StopIcon` continua (a pílula usa).
- Seletor de canais e "Nova sessão" preservados (BR-019).
- Revisão da regra: `prds/business-rules.md` BR-022 deixa de citar "play e pause
  são dois botões" e o ⏹ confirmando — passa a descrever o toggle único e o stop
  só na pílula (tasks 03/04 continuam como histórico).

## Critérios de aceite

- [ ] Parado: `▶ Iniciar escuta` inicia a captura com os canais escolhidos.
- [ ] Gravando: o controle mostra `⏸` e um clique pausa.
- [ ] Pausado: o controle mostra `▶` e um clique retoma (sem zerar o relógio —
      BR-019).
- [ ] **Existe um só stop**: só a pílula de sessão pede o encerramento, via
      "Encerrar sessão? Sim / Não"; nenhum encerramento sem confirmação.
- [ ] Nenhum rótulo de texto no controle de toggle quando há sessão aberta.
- [ ] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc`/`build` limpos.

## Verificação

- Frontend: `SessionControls.test.tsx` (matriz parado/gravando/pausado →
  start/pause/resume; garante que **não** há ⏹), `ActionBar.test.tsx` (um só
  caminho de encerramento), `SessionPill.test.tsx` segue válido.

## Notas

- O `StopIcon` permanece em `components/TransportIcons.tsx` (usado pela pílula);
  não confundir com o botão removido.
