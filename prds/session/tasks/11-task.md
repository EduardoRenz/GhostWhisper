# 11 — Sinal de transcrição em andamento na aba Transcrição

- **Status:** CONCLUDED
- **Depende de:** 02, 04
- **Habilita:** —
- **Issues:** — · **BR:** BR-040, BR-063

## Objetivo

Na aba **Sessão** (Perguntas e respostas), o usuário precisa saber se a fala
ainda está sendo transcrita antes de perguntar. O sinal fica na **aba
Transcrição existente** (o botão de aba do painel), sem criar um botão novo:
gradiente ciano da esquerda para a direita para o microfone, gradiente na cor de
Outros da direita para a esquerda, ambos simultâneos quando os dois canais falam.

## Entregáveis

- **Core** (`ports/event_publisher`, `TauriEventPublisher`):
  - evento `transcript_activity` com `session_id`, `channel` e `processing`;
  - `TranscriptionPipeline` publica `true` no primeiro bloco falado do turno e
    `false` quando o passe final termina (inclui gate/retract/stop).
- **Frontend**:
  - `useTranscriptActivity(sessionId)` assina o evento e mantém um booleano por
    canal, filtrando sessão antiga;
  - `PanelWindow` passa a atividade ao `PanelTabs`;
  - a aba **Transcrição** existente ganha as classes
    `panel-tabs__tab--streaming-mic/system`, o ponto de status e os gradientes
    CSS; o clique continua trocando para a aba.
- Sem botão, janela ou popover novo.

## Critérios de aceite

- [x] O efeito aparece na aba Transcrição existente, não em um botão novo.
- [x] Microfone: sweep ciano L→R; Outros: sweep na cor do falante R→L.
- [x] Ambos simultâneos: os dois sweeps rodam juntos.
- [x] Sem atividade: ponto neutro e nenhum sweep.
- [x] O sinal começa na fala e só apaga quando o final do turno é publicado.
- [x] Eventos de outra sessão não contaminam o estado.
- [x] `cargo test`, `clippy`, `rustfmt`, `pnpm test`, `tsc` e `pnpm build`.

## Como testar

1. Abra o painel na aba **Sessão** e inicie uma escuta.
2. Fale no microfone: a aba **Transcrição** deve mostrar o sweep ciano L→R até o
   texto final aparecer; depois o ponto volta ao neutro.
3. Com áudio do sistema/Outros, o sweep é na cor de Outros e vai R→L.
4. Com os dois canais falando, os dois sweeps aparecem juntos.
5. Clique na aba: o painel troca para Transcrição (comportamento antigo).

## Verificação (2026-10-21)

- `cargo test --lib` 545 ✓ (9 ignorados) · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- `pnpm test` 357 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Validado no AppImage: a aba existente ficou com o ponto + mic do mockup;
  durante áudio do sistema, o ponto mudou para a cor de Outros e o sweep
  apareceu na aba. O clique continua trocando para Transcrição.
