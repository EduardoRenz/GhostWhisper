# 09 — Cadência coerente do auto-sugerir + feedback de geração

- **Status:** CONCLUDED
- **Depende de:** 03
- **Habilita:** —
- **Issues:** ISS-046 · **BR:** BR-028, BR-039

## Objetivo

Tornar a contagem regressiva e o anel do auto-sugerir **previsíveis** e dar
feedback **antes** do primeiro evento do LLM. A contagem não pode parecer
aleatória nem depender de abrir outra janela.

## Diagnóstico

- `latest_final_ms` usava `MAX(timestamp)` (início do segmento): uma fala longa
  consumia o intervalo antes de terminar.
- O timer era um silêncio desde a última fala; cada fala nova reiniciava a
  contagem e, em conversa contínua, o anel quase nunca completava.
- A geração automática é síncrona no agendador; o primeiro evento `suggestion`
  só chegava depois da resposta do LLM, sem estado “gerando”.

## Entregáveis

- **Core** (`application/suggestion_scheduler.rs`, `lib.rs`):
  - intervalo = **cooldown** mínimo entre sugestões, ancorado na última
    geração/abertura da janela/sessão/reativação do automático;
  - fala nova **não** reinicia; ela libera a geração quando o contador zera;
  - sem fala nova, `awaiting_speech`; ao chegar fala nova, gera no próximo tick;
  - `latest_final_end_ms` (fim da última fala final) para o conteúdo novo;
  - evento `suggestion_cadence` ganha `generating`, publicado **antes** do LLM;
  - cadência “fresca” publicada logo após a geração (limpa o “Gerando”).
- **Frontend**:
  - `useSuggestionCadence` expõe `generating`;
  - `AutoSuggestControl`: pill “Gerando” / “Aguardando” / “Sem escuta” e arco
    indeterminado durante a geração;
  - `SuggestionsWindow`: `StatusPill` imediato quando o auto começa a gerar,
    inclusive sem sugestão atual; “Sugerir agora” desabilitado enquanto gera.
- Testes Rust (scheduler/repositórios) e TS (controle/janela/hook).

## Critérios de aceite

- [x] Abrir Sugestões inicia a contagem cheia, sem precisar abrir outra janela.
- [x] Falar várias vezes **não** reinicia a contagem; ela segue o ciclo.
- [x] Ao zerar com fala nova, gera; sem fala nova, mostra “Aguardando” e gera
      assim que a fala finalizar.
- [x] Durante a geração, o pill “Gerando” aparece **antes** da primeira resposta.
- [x] O anel esvazia de forma estável e vira arco girando durante a geração.
- [x] `cargo test`, `clippy -D warnings`, `rustfmt` (tocados), `pnpm test`,
      `tsc --noEmit` e `pnpm build` limpos.

## Como testar

1. Abra Sugestões com o automático ligado e uma sessão ativa: a contagem começa
   cheia e desce sozinha.
2. Fale durante a contagem: os segundos **não** reiniciam.
3. Espere zerar com fala nova: o pill “Gerando sugestão automática…” aparece
   imediatamente e depois a sugestão chega.
4. Espere zerar sem fala nova: a pill mostra “Aguardando”; fale e a sugestão sai
   logo em seguida.
5. Pause/retome: “Pausado” → ciclo cheio ao retomar.

## Verificação (2026-10-20)

- `cargo test --lib` 544 ✓ (9 ignorados) · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- `pnpm test` e `tsc --noEmit` ✓ · `pnpm build` ✓.
- AppImage/deb recompilados às 22:00 (Vulkan).
