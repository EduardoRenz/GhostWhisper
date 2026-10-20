# 08 — Agente Padrão (contextual), agentes que respondem e código puro

- **Status:** CONCLUDED
- **Depende de:** 01, 04
- **Habilita:** —
- **Issues:** — · **BR:** BR-059, BR-060, BR-061

## Objetivo

Três ajustes no catálogo de agentes:

1. **Agente Padrão** (novo, id `default`): identifica a **cena** da conversa e
   age como o especialista correspondente — reunião → **pauta**; entrevista de
   LeetCode → responde como o agente LeetCode; system design → dimensiona e
   desenha. Ele passa a ser o **agente padrão** do app.
2. **Nenhum agente pergunta de volta**: sempre tentam responder; quando falta
   contexto, a sugestão sai curta, dizendo que há poca informação.
3. **Código puro**: o bloco de código sai sem comentário de linha; a
   explicação fica no Markdown **fora** do bloco.

## Escopo

- `application/prompts/assistant.rs`:
  - `DEFAULT_SYSTEM_PROMPT` + `DEFAULT_SUGGESTION_PROMPT` (agente Padrão).
  - `NEVER_ASK_BACK_RULE` e `PURE_CODE_RULE` (regras comuns, injetadas no
    prompt de sistema de **todo** agente).
  - Contratos de LeetCode/Entrevista Técnica sem “comentário por linha”; o
    `"..."` de resposta vazia sai de todos os contratos.
  - `builtin_modes()`: o Padrão entra **primeiro**.
- `domain/assistant.rs`: `DEFAULT_MODE_ID`, `DEFAULT_MODE_NAME`,
  `LEGACY_DEFAULT_MODE_ID`; `BUILTIN_MODE_IDS` com 5 ids.
- `domain/settings.rs`: `default_assistant_mode()` → `default`; campo
  `assistant_mode_migrated`.
- `application/assistant_mode_migration.rs` (novo): migração **única** da
  seleção antiga (“Reunião” → Padrão) no boot.
- `application/suggestion_service.rs`: o agente Padrão leva o orçamento de
  tokens dos agentes técnicos (pode virar entrevista de código).
- Frontend: `BUILTIN_MODE_IDS`, `defaultSettings()`, fallback do agente
  excluído nas Configurações.

## Critérios de aceite

- [x] O agente **Padrão** é o primeiro da lista e o padrão das settings.
- [x] O Padrão identifica a cena (reunião / entrevista técnica / geral) e
      responde como o especialista correspondente.
- [x] Instalação antiga com a "Reunião" selecionada migra **uma vez** para o
      Padrão; depois disso, a escolha do usuário é respeitada.
- [x] Nenhum agente pede o enunciado, o código ou mais detalhes: a regra está
      no prompt de **todo** agente (embutido e personalizado).
- [x] Contexto escasso → sugestão curta dizendo que há pouca informação (sem
      `"..."`).
- [x] O bloco de código sai **sem comentários de linha**; a explicação fica no
      Markdown fora do bloco.
- [x] `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test` e
      `tsc --noEmit` verdes.

## Como testar

1. Seletor de agente: o primeiro item é **Padrão**.
2. Sugestão em uma reunião: a sugestão acompanha a pauta em discussão
   (ações/pendências).
3. Sugestão em uma entrevista de código: a resposta traz Big O, linguagem e
   **código sem comentários**, com a explicação no texto fora do bloco.
4. Sem contexto (transcrição vazia): a sugestão é curta e diz que há pouca
   informação — **não** é uma pergunta.

## Realizado (2026-10-19)

- **Agente Padrão**: `default` (`sparkle/slate`), primeiro da lista e padrão
  das settings; `DEFAULT_SYSTEM_PROMPT` (cena: reunião → pauta, entrevista →
  especialista, geral → resposta) e `DEFAULT_SUGGESTION_PROMPT` (mesma
  detecção, com o formato de cada cena).
- **Migração única**: `application/assistant_mode_migration.rs` roda no boot
  (`lib.rs`), troca o padrão antigo ("Reunião") pelo Padrão e grava
  `assistant_mode_migrated` — a escolha do usuário passa a valer.
- **Respondem, não perguntam**: `NEVER_ASK_BACK_RULE` entra no prompt de
  sistema de todo agente (inclusive personalizados) e no rascunho *fast-first*;
  os contratos deixaram de devolver `"..."`.
- **Código puro**: `PURE_CODE_RULE` entra no prompt de todo agente; os
  contratos de LeetCode/Entrevista Técnica pedem explicação no Markdown em
  volta do bloco.
- **Testes**: Rust — agente Padrão no catálogo, regras comuns em todos os
  prompts, código sem “comentário por linha”, migração (uma vez, respeita a
  escolha do usuário, não mexe em agente personalizado); TS — id `default` é
  embutido, padrão das settings e fallback ao excluir um agente.
