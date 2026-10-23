# Regras de Engenharia — GhostWhisper

Regras vinculantes para todo código, PR e agente que atua neste repositório.
Resumo humano em [`../AGENTS.md`](../AGENTS.md). Decisões de produto registradas
em [`../prds/business-rules.md`](../prds/business-rules.md) — cite o `ID`
(`BR-015`, …), não copie o texto (R14).

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
- **R16. Teste anti-regressão obrigatório:** todo bug corrigido ganha um teste
  que reproduz o cenário do relato e **falharia no código anterior**. O teste
  deve citar a issue/BR no nome ou comentário (ex.: `automatic_ask_position_…`,
  `ISS-057`). Correções de geometria/empilhamento/persistência de janela não
  podem ser aceitas apenas com validação manual.
- **R17. Isole a decisão do ambiente:** quando a causa depende de WM, X11,
  timing ou integração externa, extraia a decisão para uma função/estado puro
  testável (ex.: `apply_ask_position(position, source)`) e teste as entradas que
  representam o ambiente (posição automática do WM vs. arraste). Se o efeito
  visual não puder ser automatizado, a task registra o teste manual e o limite
  explicitamente.
- Ferramentas: Rust `cargo test`/`mockall`; frontend `vitest` +
  `@testing-library/react`; integração de HTTP com `wiremock`; DB com SQLite
  in-memory; áudio com WAVs de fixture.

## 5. Erros e logs

- Erros tipados (`thiserror` no Rust), sem `unwrap()`/`expect()` em caminho de
  produção (permitido em testes/build).
- Nunca logar segredos/áudio. Chaves vivem só na porta `SecretStore`.

## 5.1 Localização (l10n) do frontend

- **R18. Texto visível é sempre traduzido.** É proibido hardcodar copy de UI em
  JSX, atributos (`title`, `aria-label`, `placeholder`, `alt`) ou mensagens de
  status. Use `t("chave_em_snake_case")` do `src/core/l10n` (ou um helper que
  devolva `TranslationKey`, ex.: `speakerLabelKey`).
- **R19. Chaves semânticas em `snake_case`.** A chave descreve o papel da
  mensagem (`settings_save_changes`, `overlay_start_listening`), nunca o texto
  nem um número. Toda chave existe em **pt** e **en**, com os mesmos
  placeholders (`{count}`, `{name}`); o teste de catálogo valida paridade.
- **R20. Dois idiomas, um store.** O idioma vem de `Settings.ui_language`
  (BR-085) e é aplicado pelo store global (`useTranslation`); não crie estado de
  idioma local por componente. Texto de dados (nomes de agentes, transcrições,
  respostas do LLM, títulos de sessão) **não** passa por `t`.
- **R21. Detecção só na primeira inicialização.** O frontend não detecta idioma
  sozinho: o core resolve o locale do sistema uma vez (`UiLanguageMigration`) e
  persiste. Depois disso, só a escolha explícita do usuário troca o idioma.
- **R22. Teste de regressão de UI.** Componente novo/alterado com copy deve ter
  teste no idioma de origem e, quando o fluxo for global, um caso em inglês
  provando que a troca re-renderiza. Nunca deixe texto solto "passar" no teste
  só porque o português coincide com o fallback (R16).

## 6. Processo (spec-driven)

- Toda mudança relevante começa em `prds/`: PRD → arquitetura → task.
- Atualize `prds/mvp/tasks/NN-task.md` e o status em `_tasks.md`.
- Registre decisões em `prds/business-rules.md` e o histórico no `memory.md`
  **do PRD** (ver R15).
- **R14.** Documentação é **fonte única de verdade**: não duplique conteúdo já
  documentado. Referencie o arquivo com link relativo ou cite o ID canônico
  (`BR-015`, `ISS-011`, `12-task`, doc de arquitetura) — sempre que julgar útil
  para retomar contexto. Informação nova é escrita **uma vez** no doc dono e
  referenciada a partir dos demais. IDs canônicos: `BR-NNN`
  ([`business-rules.md`](../prds/business-rules.md)), `ISS-NNN`
  (`prds/<prd>/issues.md`), `NN-task` (`prds/<prd>/tasks/NN-task.md`).
- **R15.** O histórico (`memory.md`) é **por PRD**:
  [`prds/mvp/memory.md`](../prds/mvp/memory.md); não existe memory global.
  Novo PRD/tech spec ⇒ novo `memory.md` e `issues.md`.
  [`business-rules.md`](../prds/business-rules.md) continua global.

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

## 6.2 Formato do `memory.md` (histórico por PRD, buscável)

Layout fixo para permitir busca por palavra-chave, ID e **período** conforme o
arquivo cresce (R15, BR-017):

- Uma entrada = **um** título `## YYYY-MM-DD — Título curto`; nunca usar `##`
  dentro de uma entrada (`###` para subseções). Entradas separadas por `---`.
- Ordem **cronológica** (append; a entrada mais recente fica no fim).
- Linhas de contexto logo após o título: `- **IDs:** ISS-011, T12` e
  `- **Refs:** BR-015, prds/mvp/tasks/12-task.md`.
- **Sem índice de títulos** no arquivo (duplicaria conteúdo — R14); a busca é
  via `rg`.
- Cada `memory.md` traz essa legenda de busca no topo do arquivo.

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'whisper' prds/mvp/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-17' prds/mvp/memory.md` |
| Período | `rg -n '^## 2026-10-(1[0-9]\|2[0-9])' prds/mvp/memory.md` |
| Entrada inteira | `awk '/^## 2026-10-17 —/,/^## /' prds/mvp/memory.md` |
| Por ID | `rg -n 'ISS-011\|BR-015' prds/mvp/memory.md` |

## 7. Proibições

- Copiar código/asset/marca proprietária de terceiros (BR-008).
- Adicionar backend próprio, telemetria ou login (BR-002, BR-004).
- Introduzir dependência concreta dentro de `domain`/`application`.
