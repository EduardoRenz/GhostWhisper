# 18 — `already_answered`: repetição deslocada e fragmento

- **Status:** CONCLUDED
- **Depende de:** 14 (janela de 180), 17 (alvo entre blocos)
- **Habilita:** —
- **Issues:** ISS-081 · **BR:** BR-101 (revisada) · **R23/R24**
- **Refs:** [`../issues.md`](../issues.md) ISS-081,
  [`../../business-rules.md`](../../business-rules.md) BR-101,
  [`../architecture/suggestion-gating.md`](../architecture/suggestion-gating.md) §2

## Objetivo

Fazer a guarda lexical de `already_answered` reconhecer a mesma pergunta quando
ela volta **deslocada** (começa em outro ponto) ou só como **cauda/fragmento**,
sem usar palavras-chave ou idioma.

## Causa (sessão `5f8595a7…`, 2026-10-28)

As 7 sugestões geradas saíram como `question`; nenhuma repetição virou
`already_answered`.

- **Load balancer:** a 1ª sugestão guardou só a 2ª metade, porque o ME "e pode
  perguntar então." cortou o alvo. A repetição começa em `web que receberá...`;
  com os dois lados truncados em 180, o Jaccard ficou em ~0,22.
- **Amigo/dinheiro:** a repetição veio só como cauda (`... C. Ignora e muda de
  assunto.`); o Jaccard simétrico ficou em ~0,12.
- **Carteira:** a 1ª ocorrência terminou com ME e não gerou sugestão, então não
  entrou no histórico (BR-101); fora do escopo desta task.

## Entregáveis

- `PriorQuestion` guarda o texto da pergunta até **600 caracteres**
  (`QUESTION_HISTORY_MAX_CHARS`); o prompt do Laya mostra até **180**
  (`QUESTION_PROMPT_MAX_CHARS`).
- O gate grava `question_title(&full_block)` ao gerar sugestão (não só o alvo).
- `is_repeated_question`:
  - Jaccard do texto completo;
  - maior Jaccard entre **janelas deslizantes** (24 tokens, passo 6);
  - **contenção** (`|A∩B|/min(|A|,|B|)` ≥ 0,75) com ≥ 6 tokens em comum e ≥ 8
    no lado menor;
  - nenhuma lista de palavras/idioma (R23).
- A guarda do Laya avalia o **alvo** e o **bloco inteiro** (cauda/deslocamento).
- Testes:
  - domínio com os textos reais (load balancer com começo diferente, cauda do
    amigo/dinheiro, fragmento da carteira) e negativos (pergunta diferente,
    "Beleza.");
  - adaptador: Laya devolve `question` e a guarda sobrescreve para
    `already_answered` nas duas repetições; pergunta diferente chega ao Laya;
  - gate ponta a ponta com os segmentos reais de `5f8595a7…`: o histórico
    guarda as duas metades e a repetição vira `already_answered` (R24).

## Critérios de aceite

- [x] Load balancer repetido com começo diferente → `already_answered`.
- [x] Cauda do amigo/dinheiro → `already_answered`.
- [x] Fragmento da carteira (com histórico semeado) → `already_answered`.
- [x] Pergunta diferente e ACK curto continuam pergunta/no_action.
- [x] Histórico guarda o bloco inteiro; prompt continua limitado a 180.
- [x] Nenhuma palavra-chave/idioma; R23/R24 respeitadas.
- [x] `cargo test`, suíte Laya, clippy, fmt, `pnpm test` e `tsc` verdes.

## Resultado (2026-10-28)

- Texto completo + janelas + contenção; histórico do bloco inteiro.
- `cargo test --lib` **851 ✓** (11 ignorados) · Laya **9 ✓** · frontend **521 ✓**
  · clippy/fmt/tsc ✓.
- A 1ª ocorrência da carteira continua fora do histórico (não gerou sugestão,
  BR-101); registrá-la exigiria outra decisão de produto.
