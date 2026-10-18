# 04 — Pílula de sessão (tempo + confirmação) e bolinha de status

- **Status:** CONCLUDED
- **Depende de:** 03
- **Habilita:** 05

## Objetivo

Dar ao overlay a leitura de sessão do Perssua: **bolinha de status** (gravando /
pausado / parado), **tempo decorrido** na pílula de sessão e **confirmação** antes
de encerrar.

## Entregáveis

- `SessionStateEvent` (+ espelho TS `SessionState`) ganha `started_at_ms` opcional,
  preenchido enquanto há sessão aberta (`SessionService::emit_active`).
- `ActionBar`: bolinha de status (vermelha **pulsando** gravando, âmbar pausado,
  cinza parado) ao lado do texto atual.
- Pílula de sessão (fundo/borda/texto vermelhos como a do Perssua) com `mm:ss`
  tabular, atualizada a cada segundo.
- Confirmação inline **"Encerrar sessão? Sim / Não"** disparada pela pílula de
  sessão **e** pelo ⏹ do transporte (desvio consciente: um só caminho confiável de
  encerramento). "Sim" encerra; "Não" volta ao estado normal.

## Critérios de aceite

- [ ] Tempo em `mm:ss` correto em gravar/pausar (a pausa não zera o relógio).
- [ ] A bolinha pisca só gravando.
- [ ] Encerrar sempre pede confirmação; "Não" não encerra nem pausa.
- [ ] Sem sessão: pílula de sessão não aparece e o tempo não existe.

## Verificação

- Rust: `session_service` garante `started_at_ms` presente/ausente conforme o
  estado publicado.
- Frontend: `ActionBar.test.tsx` — tempo, bolinha, confirmação Sim/Não.

## Resultado (2026-10-18)

- Rust: `state_event_carries_the_session_start_for_the_timer` — `started_at_ms`
  presente ao iniciar/pausar/retomar (e igual) e ausente sem sessão.
- Frontend: `SessionPill.test.tsx` (tempo `mm:ss`, o relógio anda a cada segundo,
  clique pede encerramento mas não encerra, "Sim" encerra / "Não" desiste, botões
  desabilitados durante a operação) com relógio falso.
