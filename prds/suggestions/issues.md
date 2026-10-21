# Issues — PRD Sugestões

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser único
no repositório). Próximo ID livre: **ISS-048**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-046 | Contagem/anel do auto-sugerir incoerentes e geração sem feedback | Alta | FIXED |

---

## ISS-046 — Contagem/anel do auto-sugerir incoerentes e geração sem feedback

- **Severidade:** Alta · **Status:** FIXED ([`tasks/09-task.md`](tasks/09-task.md))
- **Sintoma:** a contagem regressiva “pulava” e o anel reiniciava a cada fala;
  era preciso abrir outra janela para começar a contar; às vezes a geração
  automática começava sem nenhum aviso até a primeira resposta.
- **Causa raiz:** três somas.
  1. A cadência contava desde o **início** do último segmento final
     (`MAX(timestamp)`), não do fim; uma fala longa encurtava o intervalo e
     podia disparar na hora.
  2. O timer era um **silêncio**: cada fala nova reiniciava a contagem, então
     em conversa contínua o anel quase nunca completava e parecia aleatório.
  3. A geração automática roda de forma síncrona no agendador; o primeiro
     evento `suggestion` só chegava depois do LLM, sem nenhum estado “gerando”.
- **Correção (task 09):** o intervalo vira um **cooldown** (tempo mínimo entre
  sugestões), ancorado na última geração/abertura da janela — fala nova não
  reinicia, só libera a geração quando o contador zera; sem fala nova, espera e
  gera assim que a próxima fala finalizar. O alerta usa o **fim** do segmento
  (`latest_final_end_ms`). O evento de cadência ganha `generating`, publicado
  antes do LLM ler; a UI mostra “Gerando” e o anel vira um arco indeterminado.
- **Arquivos:** `src-tauri/src/application/suggestion_scheduler.rs`,
  `src-tauri/src/lib.rs`, `src-tauri/src/ports/event_publisher.rs`,
  `src-tauri/src/ports/repositories.rs`,
  `src-tauri/src/adapters/persistence/{sqlite/local_transcriptions_repository,memory/in_memory_transcriptions_repository}.rs`,
  `src/core/application/useSuggestionCadence.ts`,
  `src/features/suggestions/{AutoSuggestControl,SuggestionsWindow}.tsx`,
  `src/styles/globals.css`.
- **Task:** [`tasks/09-task.md`](tasks/09-task.md)
- **Dano em dados existentes:** nenhum (a sugestão em cache e o histórico não
  mudam).

---

<!--
Modelo:

## ISS-037 — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`NN-task.md`](tasks/NN-task.md)
-->
