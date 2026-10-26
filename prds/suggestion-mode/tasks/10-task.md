# 10 — Integração no agendador (modo classificador)

- **Status:** CONCLUDED (2026-10-26) — núcleo + sonda aberta + fallback + testes de decisão; teste manual no app fica na task 11
- **Depende de:** 03, 04, 05, 06, 07, 08
- **Habilita:** 11
- **Issues:** — · **BR:** BR-092, BR-093, BR-094, BR-096

## Objetivo

Ligar o gate ao agendador real, com três caminhos explícitos (`off`, `auto`,
`classifier`), sem tocar no comportamento do modo automático. É a única task que
altera o loop de sugestões.

## Entregáveis

- `src-tauri/src/lib.rs` (`spawn_suggestion_scheduler`):
  - ramo `off`: não classifica/gera; publica estado pausado;
  - ramo `auto`: código atual preservado (cadence/should_trigger/cooldown);
  - ramo `classifier`:
    - lê settings/sessão;
    - busca falas recentes;
    - monta blocos;
    - chama o gate para o bloco fechado novo;
    - aplica a sonda de bloco aberto para hesitação;
    - publica `suggestion_classification` / estado (`Listening`, `Analyzing`,
      `Ready`, `Error`);
    - gera com `SuggestionRequest { trigger: Classifier, cue, focus }`;
    - publica `generating` antes do LLM e limpa depois;
    - cooldown/pendência/dedupe.
- `src-tauri/src/application/suggestion_service.rs`:
  - `SuggestionRequest` com `cue`/`focus`;
  - prompt do gerador recebe uma linha curta de foco (sem sobrescrever o
    contrato do agente);
  - `Suggestion.cue` preenchido;
  - testes de prompt.
- `src-tauri/src/state.rs` / `lib.rs`: wiring do gate e do backend
  (`heuristic`/`laya`), revisão de settings para recriar adaptador quando
  necessário.
- `src-tauri/src/ports/event_publisher.rs`: publicar estado do classificador
  (se ainda não estiver na task 02).
- Testes de scheduler/gate.

## Critérios de aceite

- [x] `auto` não chama o classificador (branch separado; a decisão de
      classificação vive em `application/classifier_tick`).
- [x] `auto` mantém a cadência/contagem/feedback atuais.
- [x] `off` não classifica nem gera; “Sugerir agora” continua via IPC.
- [x] `classifier` classifica **uma vez** por bloco fechado (cache do gate); o
      mesmo bloco não gera duas sugestões.
- [x] Só gera com `should_suggest`, cue permitida, confiança ≥ limiar, cooldown
      vencido e sem geração em curso.
- [x] Decisão positiva com cooldown pendente gera ao zerar sem reclassificar
      (`pending` no gate).
- [x] Bloco novo substitui pendência; decisão negativa limpa.
- [x] Sonda aberta: `probe_open` no gate (ME no fim, fala contínua ≥ 4 s,
      ≥ 3 palavras, throttle 2 s, allowlist de `hesitation`, sem bloco novo).
      O scheduler chama a sonda **antes** do bloco fechado.
- [x] Timeout/erro do classificador não gera (fallback `skip`), publica estado
      de erro e mantém a última válida.
- [x] Fallback `auto`: erro do classificador cai no caminho automático quando
      `suggestion_classifier_fallback=auto`; `skip` continua o padrão.
- [x] Trocar de sessão reseta cursor/pendência/cache.
- [x] Nenhum log de transcrição.
- [x] `pnpm test`/`cargo test` existentes continuam verdes.

## Testes

- [x] `open_probe_requires_me_hesitating_in_an_open_block` (domínio)
- [x] `open_probe_triggers_hesitation_and_is_throttled` (gate)
- [x] `open_probe_respects_settings_and_closed_blocks` (gate)
- [x] `prefers_the_open_probe` / `uses_the_closed_block_when_the_probe_has_nothing`
- [x] `no_block_is_skip_not_failure`
- [x] `probe_error_skips_by_default_and_falls_back_when_configured`
- [x] `closed_error_skips_by_default_and_falls_back_when_configured`
- [x] `does_not_classify_same_block_twice` (gate)
- [x] `classifies_newest_closed_block_with_both_speakers` (gate)
- [x] `skips_cooldown_and_duplicate_block` / `waits_cooldown_then_generates_pending`
- [x] `suggestion_prompt_includes_cue_and_focus_without_overriding_agent`
- [ ] Teste manual no app (task 11).

## Verificação (2026-10-26)

- O scheduler chama `application::classifier_tick::classify_once`, que decide
  sonda aberta → bloco fechado → fallback; a geração usa
  `SuggestionRequest` com `trigger=Classifier`, cue/foco e
  `since_ms=block_end_ms`, publica `generating` antes/depois do LLM e marca o
  bloco gerado.
- `probe_open` aplica elegibilidade + throttle + dedupe por bloco; o heurístico
  detecta hesitação em bloco aberto com os dois falantes.
- `SuggestionService` respeita `request.trigger` e injeta cue/foco no prompt.
- Rust **806 passed**, 11 ignored (sem feature); **809 passed**, 12 ignored
  (com `--features laya`); clippy/fmt limpos.
- Frontend **514 passed**; `tsc`/build limpos.
- Bug encontrado no teste manual: o gate comparava época Unix com timestamps
  relativos da sessão e não achava bloco (ISS-060). Corrigido com
  `relative_now` + `NoClassifiableBlock`; teste de regressão
  `uses_session_relative_time_for_transcriptions`.
- Teste manual: a hesitação em bloco só de ME não era detectada pelo
  heurístico; corrigido (ISS-061) e `classify_once` passou a consumir o
  `pending` do cooldown.
- Pendente: roteiro manual da task 11.

## Como testar

1. Modo automático: repetir o roteiro da task 09 do PRD de sugestões (contagem,
   pausa, gerando) e confirmar que o fake do classificador não foi chamado.
2. Modo classificador + heurístico:
   - pergunta de THEY → classifica `question` → sugestão;
   - ME hesita → `hesitation` → sugestão;
   - conversa fluente → `no_suggestion` → nenhuma chamada ao LLM;
   - mesmo bloco não duplica.
3. Modo classificador + Laya: repetir com o ONNX `laya-multilingual` carregado;
   depois apontar para um caminho inválido e confirmar estado de erro e
   “Sugerir agora”.
4. Trocar de sessão e confirmar reset.

```bash
cargo test --lib application::suggestion_scheduler
cargo test --lib application::suggestion_service
cargo test --lib lib::suggestion_scheduler
```

## Notas

- O loop é síncrono; classificação + geração podem atrasar o tick. Para o POC é
  aceitável; evolução futura é separar a geração em thread.
- O `AppState` não deve guardar o adaptador concreto; guardar o trait.
- Se o backend mudar em runtime, recriar o adaptador no próximo boot ou por
  revisão de settings; documentar o comportamento escolhido.
