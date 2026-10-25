# 15 — Perguntar com memória longa: resumo no contexto, em paralelo

- **Status:** CONCLUDED
- **Depende de:** 03; PRD `precision-and-performance` (resumo incremental)
- **Issues:** — · **BR:** BR-053, BR-024 · **Rules:** R16
- **Refs:** [`../prd.md`](../prd.md) US-02;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §5.1

## Objetivo

Fechar o buraco de memória do **Perguntar**: a janela de 180 s responde sobre os
últimos momentos, mas uma pergunta aos 30 min (“o que decidimos no começo?”) não
tinha de onde vir. O resumo incremental da sessão — que já é gerado desde o
início por cursor — passa a entrar no prompt como **memória longa**, sem
sacrificar a velocidade da resposta.

## Entregáveis

- `application/ask_service.rs`:
  - `AskService` recebe `Arc<dyn SummaryGenerator>` (porta já existente).
  - `build_context` lê o resumo gravado em uma consulta barata; falha de
    leitura é **não-fatal** (vira `None`).
  - prompt ganha o bloco `<meeting_summary>` (assunto + markdown) **antes** do
    `<transcript>`; o system prompt declara que a fala recente tem precedência.
  - resumo limitado a **6 000 chars** (corte pelo fim, sinalizado com `…`).
  - quando o resumo da reunião está presente, a janela de resumos de perguntas
    antigas cai de **5 para 3** (orçamento do prompt).
  - atualização do resumo em **thread própria**, disparada antes da chamada ao
    LLM da resposta e **nunca aguardada**:
    - só quando o resumo falta ou está mais velho que a janela de 180 s;
    - single-flight por sessão (sem duas atualizações simultâneas);
    - cooldown de **60 s** depois de uma falha (não martela o provedor);
    - erro só em log; a pergunta continua normalmente.
- `lib.rs`: injeta `Arc::clone(&summary)` no composition root.
- Testes: resumo no prompt (ordem/assunto), ausência do bloco, janela 3,
  resumo fresco não dispara, falha não falha a resposta, prova de paralelismo
  (resumo bloqueado + resposta concluída), single-flight/cooldown, staleness e
  corte UTF-8.

## Critérios de aceite

- [x] O resumo da sessão entra no prompt do Perguntar quando existe.
- [x] Sem resumo, o comportamento anterior é preservado (só janela + diário).
- [x] Resumo ausente/velho é atualizado em background, em paralelo à resposta.
- [x] A resposta **não** espera a geração/atualização do resumo.
- [x] Falha do resumo não impede nem atrasa a pergunta.
- [x] Duas perguntas seguidas não disparam duas atualizações concorrentes.
- [x] O prompt continua limitado (transcript 6k + resumo 6k + 3 resumos de
      perguntas + system) — sem crescer com a duração da reunião.

## Verificação

- `cargo test --manifest-path src-tauri/Cargo.toml --lib` **692 ✓** (10
  ignorados) · `clippy --all-targets -D warnings` ✓ · `cargo fmt --check` ✓.
- Nenhuma mudança de frontend.

## Limite conhecido

Se a primeira pergunta acontecer antes de qualquer resumo existir, ela ainda
responde **sem** a memória longa (velocidade primeiro); a geração roda em
background e a **próxima** já a utiliza. Um warm-up na abertura da barra foi
deixado de fora para não gerar uma chamada extra ao provedor a cada abertura e
para não duplicar a geração disparada pelo próprio Perguntar.
