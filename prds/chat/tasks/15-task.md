# 15 — Concorrência e ciclo de vida das sugestões (single-flight e janela)

- **Status:** CONCLUDED
- **Depende de:** 13
- **Habilita:** 14
- **Issues:** — · **BR:** BR-042, BR-041
- **Refs:** [`../prd.md`](../prd.md) §9 (US-13),
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §5.4,
  [`../../suggestions/architecture/suggestions-window.md`](../../suggestions/architecture/suggestions-window.md) §5.4

## Objetivo

Fechar o comportamento quando **já existe uma sugestão em andamento** — do
automático ou de uma manual anterior — e quando a janela de Sugestões está
**aberta, fechada ou oculta**. Sem streams intercalados, sem geração duplicada
ao reabrir e sem perder o contexto manual.

Regras de ouro:

1. **Uma geração por vez no serviço** (*single-flight*): o automático e a manual
   nunca emitem `quick`/`full` ao mesmo tempo.
2. **Manual espera, não cancela**: o clique manual entra na vez; o botão mostra
   "Aguardando a sugestão em andamento…" enquanto o automático termina.
3. **Manual tem prioridade de leitura**: com contexto manual ativo, a sugestão
   automática vai para o **histórico** e não substitui o cartão manual.
4. **Abrir não gera** (BR-028 revisada): a janela hidrata com `suggestionLast`
   e deixa o automático (janela aberta + intervalo) ou o botão gerar.
5. **Troca de sessão zera a leitura**: eventos da sessão antiga são ignorados.

## Entregáveis

**Núcleo**

- `src-tauri/src/application/suggestion_service.rs`:
  - o `gate` **existente** já serializa as gerações; garantir que ele cobre
    fast-first + streaming + persistência e todas as entradas (janela ×
    agendador × manual). Ajustar a documentação do campo se necessário;
  - o automático e o manual chamam `generate` normalmente; o segundo espera o
    primeiro terminar (sem cancelamento de LLM);
  - o gate não cobre `pin` (não chama LLM).
- `src-tauri/src/ports/suggestion_generator.rs`:
  - `fn last_suggestion(&self, session_id: &str) -> Result<Option<Suggestion>>`
    — consulta o cache `last` (sem LLM, sem lock de geração) para a janela
    reidratar o cartão ao montar/reabrir;
  - `SuggestionSource { speaker, at_ms, preview }` — snapshot **leve** da
    mensagem que originou a manual (`preview` = primeiros ~160 caracteres, sem
    repetir o texto inteiro em cada delta);
  - `Suggestion.source: Option<SuggestionSource>` em todas as fases; `None` no
    automático/“Sugerir agora”;
  - usar o `generation_id` introduzido na 13 como identidade: a UI passa a usá-lo
    como chave de `current`, `quick` e histórico (o `at_ms` fica só para
    exibição/ordenação). Isso evita colisão de duas gerações no mesmo `at_ms`
    (ex.: relógio fixo em teste);
  - a UI usa o snapshot para rotular a manual com a mensagem certa mesmo que o
    contexto já tenha mudado. O texto completo continua no
    `ManualSuggestionContext`/prompt, não no evento.
- Testes Rust:
  - dois `generate` concorrentes → o segundo só começa depois do primeiro
    (fake LLM com barreira/atraso);
  - `source` presente na manual e ausente no automático;
  - `last_suggestion` devolve a última sem chamar LLM e respeita `session_id`;
  - `quick`/`full` da mesma geração nunca se intercalam com outra geração.

**Frontend**

- `src/adapters/ipc/suggestions.ts`: `suggestionLast(sessionId)` → última
  sugestão em cache (sem gerar).
- `src/core/application/useSuggestions.ts`:
  - `manualContextRef` para o handler de eventos saber se o contexto manual está
    ativo sem depender de closure antiga;
  - `applyIncoming` passa a casar `quick`/`full` por `generation_id` (não por
    `at_ms`); o histórico deduplica por id. Com contexto manual ativo,
    - `quick` automática é **ignorada** (efêmera);
    - `full` automática vai para o **histórico**, sem trocar `current`/`quick`;
    - manual continua aplicando `quick`/`full` normalmente;
  - ao limpar o contexto, o automático volta a poder ocupar o cartão;
  - `generating` continua `loading || streamActive`; o botão manual e o
    "Sugerir agora" ficam desabilitados durante a geração;
  - guardar a origem do stream em andamento (`auto`/`manual`) para o pill:
    "Gerando sugestão manual…" / "Sugestão automática em andamento — vai para o
    histórico" / "Gerando uma nova sugestão…";
  - ao montar/trocar sessão, busca `suggestionLast` e hidrata `current` se ainda
    estiver vazio (sem chamar LLM);
  - **abrir não gera**: o efeito de abertura nunca chama `generate`; a janela só
    exibe o que veio de `suggestionLast`/dos eventos ou espera o botão;
  - se o serviço devolver "sem fala" no botão, o erro amigável existente aparece
    e não entra em loop;
  - no máximo um `generate` manual por vez (`inFlight` já existe; preservar).
- `src/features/suggestions/SuggestionsWindow.tsx`:
  - estado visual de espera: o botão "Gerar sugestão manual" fica desabilitado
    com rótulo **"Aguardando a sugestão em andamento…"** (ou equivalente) quando
    `generating`;
  - o bloco de contexto manual permanece visível durante a espera e mostra a
    dica **"A sugestão manual tem prioridade; as automáticas vão para o
    histórico enquanto este contexto estiver ativo."**;
  - se a janela for oculta durante a geração manual, o resultado continua no
    estado ao reabrir.
- `src/features/suggestions/SuggestionCard.tsx` / `SuggestionHistory.tsx`:
  - usam `generation_id` como chave estável (inclusive no relógio "Há 12s");
  - cartão/histórico da manual mostram o **source snapshot** (ex.: "Manual ·
    Microfone 10:42:15"), quando disponível;
  - a automática que foi para o histórico não é confundida com a manual.
- Testes TS:
  - automática `full` com contexto manual ativo → vai para o histórico e não
    troca `current`;
  - automática `quick` com contexto manual ativo → ignorada;
  - limpar o contexto → a próxima automática pode ocupar o cartão;
  - abrir a janela em qualquer estado → **não** chama `generate`;
  - `suggestionLast` hidrata o cartão sem LLM;
  - duas chamadas manuais → só uma em voo;
  - resultado manual com `source` mostra o rótulo correto;
  - sessão antiga não aplica evento/resultado na nova.

## Matriz de cenários

| Cenário | Comportamento |
|---|---|
| Janela oculta + geração iniciada quando aberta | A janela segue montada e acumula os eventos; ao abrir, mostra o estado e **não** dispara outra geração |
| Abrir vazia (auto ligado ou não) | Não chama `generate`; `suggestionLast` hidrata se houver; o auto só gera com a janela aberta e após o intervalo |
| Janela fechada + clique no `[✦]` | Contexto é gravado e a janela abre/foca; se havia geração, o botão manual espera; se não, fica pronto para gerar |
| Janela aberta + automática em andamento + clique no `[✦]` | Contexto atualiza; botão manual desabilitado com "Aguardando…"; a automática vai para o histórico quando completar |
| Manual em andamento + novo `[✦]` | O contexto novo substitui o antigo; a manual antiga termina e é rotulada com o `source` antigo; o botão fica desabilitado até o fim e depois libera para o contexto novo |
| Manual em andamento + janela oculta | A geração continua; ao reabrir, o resultado/source está lá |
| Troca de sessão | UI limpa `current`/`quick`/histórico/contexto; eventos e resultados da sessão antiga são ignorados |
| Automática depois de uma manual | Com contexto manual ativo, vai para o histórico; sem contexto, pode ocupar o cartão |

## Critérios de aceite

- [ ] O serviço nunca tem dois streams `quick`/`full` ao mesmo tempo.
- [ ] Duas gerações com o mesmo `at_ms` (relógio fixo) continuam distintas no
      estado/histórico pelo `generation_id`.
- [ ] Manual clicada durante o automático espera e gera depois; o botão mostra o
      estado de espera e não dispara chamadas concorrentes.
- [ ] Com contexto manual ativo, a automática **não** substitui o cartão manual;
      ela aparece no histórico e o contador da seção reflete isso.
- [ ] **Abrir a janela nunca chama o LLM**; `suggestionLast` hidrata o cartão sem
      LLM.
- [ ] Ocultar/mostrar a janela durante a geração não perde o resultado.
- [ ] Um novo `[✦]` durante uma manual não cria duas gerações: a antiga termina
      rotulada com seu `source` e o contexto novo fica pronto.
- [ ] Trocar de sessão não aplica eventos antigos na nova.
- [ ] `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`,
      `tsc --noEmit` e `pnpm build` verdes.

## Verificação

- Automático ligado, falar e, durante o streaming, clicar no `[✦]` de outra
  mensagem: conferir a espera, o histórico e a manual no fim.
- Repetir com a janela de Sugestões fechada e com ela aberta.
- Clicar em duas mensagens diferentes com uma manual em andamento.
- Ocultar a janela durante a manual e reabrir.
- Encerrar/iniciar outra sessão no meio de uma geração.

## Notas

- **Sem token de cancelamento** no `LlmController`: a manual espera a geração
  atual. Preempção imediata exigiria uma task própria (cancelamento/abort).
- **Com `generation_id`**: o gate evita intercalação; o id dá **identidade** à
  geração (quick/full/deltas e histórico) sem depender de `at_ms`, que pode
  empatar (relógio fixo/testes). Não vai ao banco.
- `trigger` e `source` continuam sendo os rótulos; o id é a chave técnica.
- A UI pode não saber do automático antes do primeiro evento; a garantia de
  exclusão mútua é o **gate no core**. O `loading` manual cobre a espera.
- O gate não deve segurar o `Mutex` do `last`; a ordem é sempre
  `gate` → `last`, para evitar deadlock. `last_suggestion` e `pin` não
  adquirem o gate.

## Resultado (2026-10-19)

- O `gate` existente do `SuggestionService` serializa todas as gerações; o
  scheduler e o botão manual esperam a vez (sem cancelamento de LLM).
- `SuggestionService::last_suggestion` expõe o cache; a janela hidrata sem LLM.
- A janela **não gera ao abrir** (BR-028 revisada); com contexto manual ativo, a
  automática `quick` é ignorada e a `full` vai para o histórico.
- `generation_id` é a chave de `current`/`quick`/histórico; `source` rotula a
  manual e evita confundir resultados antigos com o contexto novo.
- Testes de prioridade manual, hidratação, geração manual e store.
- `cargo test --lib` 378 ✓ · `pnpm test` 261 ✓ · `clippy`/`fmt`/`build` ✓.
