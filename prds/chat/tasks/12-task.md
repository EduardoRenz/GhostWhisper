# 12 — Ação de IA por mensagem e handoff de contexto para Sugestões

- **Status:** PENDING
- **Depende de:** 11
- **Habilita:** 13
- **Issues:** — · **BR:** BR-041
- **Refs:** [`../prd.md`](../prd.md) §9, [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §4–§5

## Objetivo

Cada mensagem **finalizada** da Transcrição ganha um botão de **IA** no canto
superior direito. O clique **não chama o LLM**: ele entrega a mensagem (sessão,
falante, texto inteiro e instante) ao núcleo, abre/foca a janela de **Sugestões**
e deixa o contexto pronto para o usuário gerar uma **sugestão manual** lá.

O handoff é explícito e resistente a janela fechada:

```
ChatBubble [✦]
  └─ suggestion_prepare_manual(ctx) ──▶ ManualSuggestionContextStore
  └─ panel_open("suggestions") ───────▶ evento panel_opened
                                            └─ SuggestionsWindow busca o contexto
```

Nada de variável global de UI, URL ou evento efêmero que se perde. O **caso de
uso** recebe o contexto como **parâmetro** (`SuggestionRequest.manual_context`,
task 13); o store é só o correio entre as duas janelas.

## Entregáveis

**Núcleo / arquitetura**

- Novo `src-tauri/src/ports/manual_suggestion_context.rs`:
  ```rust
  pub struct ManualSuggestionContext {
      pub session_id: String,
      pub speaker: Speaker,
      pub text: String,
      pub at_ms: i64,
  }

  pub trait ManualSuggestionContextStore: Send + Sync {
      fn put(&self, context: ManualSuggestionContext) -> Result<()>;
      fn get(&self, session_id: &str) -> Result<Option<ManualSuggestionContext>>;
      fn clear(&self, session_id: &str) -> Result<()>;
  }
  ```
  - guarda **um** contexto por vez (a sessão ativa); `put` substitui, `get`
    devolve só se `session_id` bater; `clear` zera.
- `src-tauri/src/adapters/persistence/memory/in_memory_manual_suggestion_context_store.rs`
  (dobra de teste do próprio port; `Mutex<Option<...>>`).
- `AppState.manual_context: Arc<dyn ManualSuggestionContextStore>` + wiring em
  `lib.rs` (composition root).
- `Speaker` ganha `Deserialize` (o IPC recebe `"ME"`/`"THEY"`; o enum já tem
  `Serialize`).

**IPC**

- `suggestion_prepare_manual(session_id, speaker, text, at_ms)`:
  - valida sessão existente e **ativa** (`SessionStatus::Active`; pausada conta);
  - `text.trim()` não vazio, teto `MANUAL_CONTEXT_MAX_CHARS = 6_000`;
  - `speaker` inválido → erro de configuração;
  - grava no store e devolve o contexto normalizado;
  - **não** chama LLM, **não** grava no banco.
- `suggestion_manual_context(session_id)` → `Option<ManualSuggestionContext>`.
- `suggestion_clear_manual_context(session_id)` → `()`.
- Nenhum evento novo no `EventPublisher`: a janela de Sugestões reaproveita o
  `panel_opened` que o `panel_open` já emite (e busca o store ao montar).

**Frontend**

- `src/adapters/ipc/suggestions.ts`: `prepareManualSuggestion`,
  `manualSuggestionContext`, `clearManualSuggestionContext` + tipos TS.
- `src/features/chat/ChatBubble.tsx`:
  - botão `[✦]` no grupo superior direito, `data-interactive`, `aria-label`
    "Sugerir com IA a partir desta mensagem", título;
  - só em fala **finalizada** e **primeira parte** (`!continues`); oculto em
    prévia/streaming e em sugestão da IA;
  - estado *busy* enquanto prepara; erro exibido de forma discreta (não engole).
- `src/features/chat/ChatView.tsx`: prop `onAskAi?: (input: { id; speaker; text;
  atMs }) => Promise<void> | void`; passa o `source_text` inteiro do segmento.
- `src/features/panel/TranscriptionView.tsx`: handler que chama
  `prepareManualSuggestion` e, em seguida, `panelOpen("suggestions")`; sem sessão
  ativa, o botão não aparece.
- Testes: store Rust (`put/get/clear`, sessão divergente), `ChatBubble`/`ChatView`
  (ação só em finalizada, texto completo, sem ação em prévia/continuação) e
  `TranscriptionView` (prepara + abre; erro não abre uma janela vazia).

## Critérios de aceite

- [ ] Toda fala finalizada mostra o botão de IA no canto superior direito; a
      prévia em andamento, a continuação e a bolha de sugestão não mostram.
- [ ] O clique envia `session_id`, `speaker`, o **texto inteiro** do segmento e
      `at_ms`, abre/foca a janela de Sugestões e **não** chama o LLM.
- [ ] Janela fechada: o contexto continua disponível quando ela abre. Janela
      aberta: um novo clique substitui o contexto e a janela é focada.
- [ ] Sem sessão ativa, o botão não aparece; sessão encerrada é recusada.
- [ ] Texto vazio/estourado é recusado sem quebrar a UI.
- [ ] `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`,
      `tsc --noEmit` e `pnpm build` verdes.

## Verificação

- Clicar em uma fala do microfone e em uma de "Outros"; conferir o contexto
  recebido em Sugestões (ainda sem gerar) e o foco da janela.
- Fechar Sugestões, clicar de novo em outra fala e reabrir: o contexto deve ser o
  **último** clicado.
- Trocar/encerrar a sessão: o contexto antigo não pode ser devolvido.
- Testar o botão com teclado (`Tab` + `Enter`).

## Notas

- O botão é uma **ação de contexto**, não de geração: a geração manual e o rótulo
  "Sugestão Manual" ficam na task 13.
- A janela de Sugestões só busca o store ao montar e ao receber `panel_opened`;
  por isso não há corrida entre preparar o contexto e abrir a janela.
