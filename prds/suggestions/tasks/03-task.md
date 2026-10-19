# 03 — Cabeçalho novo: faísca, título, LIVE, cadência circular e Sugerir agora

- **Status:** CONCLUDED
- **Depende de:** 02
- **Habilita:** 07
- **Issues:** — · **BR:** BR-039

## Objetivo

O cabeçalho passa a ter a identidade do notch (faísca), título **“Sugestões”**,
badge **LIVE**, o controle de **auto-sugerir com progress circular** e o botão
**Sugerir agora** (sem glow).

## Escopo

- **Core:** novo evento `suggestion_cadence` (`enabled`, `running`,
  `remaining_ms`, `awaiting_speech`, `window_ms`), publicado pelo **agendador**
  (fonte da verdade) ~1/s enquanto a janela de sugestões está aberta. O timer é
  o **intervalo das settings** (`suggestion_interval_secs`) + o *debounce*.
- **Sem fala nova, não há nova sugestão:** o serviço guarda a última sugestão da
  sessão e a devolve (sem chamar o LLM); só as ações explícitas (“Sugerir
  agora”, chat, atalho) forçam a geração.
- **Frontend:** `useSuggestionCadence` (hook) + `AutoSuggestControl`:
  - círculo SVG com `stroke-dashoffset` (sem re-render por quadro);
  - **texto sempre visível** (“Auto-sugerir”) + **pill de contagem**
    (“12s” rodando, **“Pausado”** pausado, “0s” quando a janela venceu sem
    fala nova);
  - ação de **pausar/retomar** (grava `settings.suggestion_auto_refresh`).
- **Header:** `SuggestionHeader` com a faísca (o mesmo ícone do botão do notch),
  título, badge **LIVE**, o controle de auto e **Sugerir agora** (desabilitado
  enquanto gera, **sem glow**).
- O "Sugerir agora" continua forçando a geração (`suggestion_generate`).

## Critérios de aceite

- [x] Cabeçalho com faísca, “Sugestões” e badge LIVE.
- [x] O círculo esvazia a cada janela e a contagem confere com o agendador.
- [x] Ao zerar com fala nova, o auto-sugerir dispara e a contagem reinicia.
- [x] Sem fala nova, o círculo fica vazio e o rótulo diz “aguardando fala”.
- [x] A cadência exibida é a **do agendador** (o automático não mudou de
      comportamento).
- [x] Pausado: círculo parado + “Pausado” + ação de retomar.
- [x] “Sugerir agora” sem glow e desabilitado durante a geração.
- [x] Rust/TS verdes.

## Como testar

1. Abrir Sugestões: ver LIVE, o círculo e a contagem.
2. Falar e esperar: a contagem zera, a sugestão aparece e a contagem reinicia.
3. Parar de falar: o círculo esvazia e o rótulo vira “aguardando fala”.
4. Pausar e retomar o automático; conferir o rótulo e o estado.

## Realizado (2026-10-19)

- **Core**: `SuggestionCadenceEvent` na porta `EventPublisher` + evento
  `suggestion_cadence`; a **decisão é pura**
  (`application::suggestion_scheduler::cadence`) e o agendador publica ~1/s
  **só com a janela visível** — a cadência é o timer que já existe, não um
  timer novo.
- **Cálculo**: `remaining = max(debounce − desde_a_fala, intervalo_mín − desde_a_
  última_execução)`; sem fala nova → `awaiting_speech` (anel vazio).
- **Frontend**: `useSuggestionCadence` (hook) + `AutoSuggestControl` (anel SVG
  com `stroke-dashoffset`, rótulo “12s” / “Pausado” / “aguardando fala”,
  clique pausa/retoma).
- **Cabeçalho**: `PanelFrame` ganhou `heading` (cabeçalho próprio); a janela
  mostra faísca (mesma do notch) + “Sugestões” + badge LIVE/PAUSADO e o
  “Sugerir agora” **sem glow** (`.suggest-now`).
- **Testes**: 6 novos (Rust) + 7 novos (TS) — `cargo test` 366 ✓,
  `pnpm test` 209 ✓, `tsc --noEmit` ✓, clippy/fmt limpos.
- **Ajuste posterior (pedido do usuário, 2026-10-19):** o controle ganhou o
  texto “Auto-sugerir” + a pill de contagem (o cabeçalho agora cabe em uma
  linha na janela de 540 px); o agendador passou a usar o **intervalo das
  settings** (antes configuração morta); e o serviço devolve a **última
  sugestão** quando não há fala nova (sem chamar o LLM).
