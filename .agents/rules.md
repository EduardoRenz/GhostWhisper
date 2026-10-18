# Regras de Engenharia — GhostWhisper

Regras vinculantes para todo código, PR e agente que atua neste repositório.
Resumo humano em [`../AGENTS.md`](../AGENTS.md). Decisões registradas em
[`../prds/business-rules.md`](../prds/business-rules.md) (BR-011..BR-014).

---

## 1. Arquitetura — Hexagonal (ports & adapters)

Toda feature é modelada em 4 camadas, com **inversão e injeção de dependência**:

```
domain  <- application <- ports <- (adapters) -> application
```

| Camada | Conteúdo | Pode depender de |
|--------|----------|------------------|
| `domain` | Entidades e regras puras (sem I/O) | nada |
| `application` | Casos de uso / orquestração | `domain`, `ports` |
| `ports` | Traits (contratos) | `domain` |
| `adapters` | Implementações concretas (SQLite, HTTP, audio, OS) | `ports`, `domain` |
| `ipc` | Adapter de **entrada** (Tauri commands/events) | `application`, `ports` |
| `lib.rs` | **Composition root** (wiring/DI) | tudo |

Regras:
- **R1.** `domain` e `application` nunca importam `adapters`. Dependem de traits.
- **R2.** Nenhum `new` de adaptador fora do composition root (`lib.rs`).
- **R3.** Injeção de dependência por construtor. `AppState` guarda traits:
  `Arc<dyn ChatBotProvider>`, `Arc<dyn TranscriptionsRepository>`, etc.
- **R4.** DI manual por padrão; `shaku` é permitido apenas se necessário, e nunca
  como desculpa para esconder dependências.
- **R5.** Uma porta nova = um trait em `ports/`; uma integração nova = um adaptador
  em `adapters/`.
- **R6.** O `ipc` só traduz transporte ↔ caso de uso; não contém regra de negócio.

## 2. SOLID

- **S** — cada struct/trait tem uma responsabilidade; arquivos focados.
- **O** — estenda comportamento com novos adaptadores, não com `if platform`.
- **L** — adaptadores honram o contrato da porta (não lançam por “não suportado”;
  retornam erro de domínio tipado).
- **I** — portas pequenas e específicas; uma porta por capacidade.
- **D** — dependa de abstrações (ports), nunca de concretos.

## 3. Nomenclatura

Sem prefixo `I`. Abstração primeiro; implementação prefixada pelo contexto/tecnologia.

| Papel | Padrão | Exemplo |
|-------|--------|---------|
| Porta (trait/interface) | substantivo da capacidade | `TranscriptionsRepository` |
| Adaptador local | `Local` + porta | `LocalTranscriptionsRepository` |
| Adaptador de tecnologia | `<Tech>` + porta | `OpenAiChatBotProvider`, `GeminiChatBotProvider`, `WhisperSpeechToText`, `KeyringSecretStore` |
| Dobra de teste | `InMemory`/`Fake`/`Stub`/`Spy` | `InMemoryTranscriptionsRepository`, `FakeChatBotProvider` |
| Evento/erro | `<Domínio><Ação>` | `TranscriptSegmentFinalized`, `ProviderAuthError` |

Convenções específicas:
- **Rust:** traits e structs `PascalCase`; arquivos/`snake_case`; sem `Impl`/`Factory`
  no nome; erros com `thiserror`.
- **TypeScript:** interfaces/tipos `PascalCase` (sem `I`); classes `PascalCase`;
  hooks `useX`; componentes `PascalCase`.
- Booleanos: `is*`, `has*`, `should*`. Funções: verbo no infinitivo.
- Eventos/commands Tauri: `snake_case` (ex.: `transcript_segment`).

## 4. Testabilidade

- **R7.** Proibido I/O em `domain`/`application`; portas existem para permitir fakes.
- **R8.** Toda porta tem pelo menos uma dobra de teste (`adapters/.../memory`,
  `Fake*`).
- **R9.** Meta de cobertura: `domain`/`application` **alta (>80%)**; adaptadores
  best-effort via testes de integração.
- **R10.** Sem dependência global mutável; tempo via porta `Clock`, aleatoriedade
  via porta quando necessário.
- Ferramentas: Rust `cargo test`/`mockall`; frontend `vitest` +
  `@testing-library/react`; integração de HTTP com `wiremock`; DB com SQLite
  in-memory; áudio com WAVs de fixture.

## 5. Erros e logs

- Erros tipados (`thiserror` no Rust), sem `unwrap()`/`expect()` em caminho de
  produção (permitido em testes/build).
- Nunca logar segredos/áudio. Chaves vivem só na porta `SecretStore`.

## 6. Processo (spec-driven)

- Toda mudança relevante começa em `prds/`: PRD → arquitetura → task.
- Atualize `prds/mvp/tasks/NN-task.md` e o status em `_tasks.md`.
- Registre decisões em `prds/business-rules.md` e o histórico em `prds/memory.md`.

## 6.1 Referência de produto: Perssua é a fonte de verdade

- **R11.** Ao decidir **como** resolver um problema de produto/técnica, priorize a
  solução que o **Perssua** já usa — ele é um produto maduro e validado. A única
  transposição esperada é de **tecnologia** (Electron/JS/SQLite/whisper.cpp →
  Tauri/Rust), nunca de comportamento.
- **R12.** Antes de inventar um protocolo, heurística ou parâmetro novo, procure o
  equivalente no Perssua (extraído com o REA em `Perssua-*.AppImage` →
  `resources/app.asar`, módulos como `electron/local-whisper`). Só proponha algo
  diferente quando houver motivo técnico explícito (ex.: captura de áudio Linux),
  e documente o desvio.
- **R13.** Ao transpor, mantenha **nomes e valores** observáveis do Perssua
  (constantes, limiares, ordem de etapas) para facilitar a auditoria, adaptando
  apenas o que não existe no Linux.

## 7. Proibições

- Copiar código/asset/marca proprietária de terceiros (BR-008).
- Adicionar backend próprio, telemetria ou login (BR-002, BR-004).
- Introduzir dependência concreta dentro de `domain`/`application`.
