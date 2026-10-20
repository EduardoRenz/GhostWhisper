# 07 — Multimodal: `CompletionRequest` com imagens e o print na pergunta

- **Status:** CONCLUDED
- **Depende de:** 06
- **Habilita:** —
- **Issues:** — · **BR:** BR-005, BR-053, BR-056
- **Refs:** [`../prd.md`](../prd.md) US-03;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §6

## Objetivo

Tornar o pedido à IA **multimodal** e ligar o **print** à pergunta: a IA
precisa **ver** a imagem anexada.

## Entregáveis

- `src-tauri/src/domain/llm.rs`:
  - `CompletionImage { mime: String, data: Vec<u8> }`;
  - `CompletionRequest.images: Vec<CompletionImage>` (com `Default` para não
    quebrar os construtores existentes).
- `src-tauri/src/adapters/ai/openai_compatible.rs`:
  - quando há imagens, `content` vira lista:
    `[{"type":"text","text":…},{"type":"image_url","image_url":{"url":"data:image/jpeg;base64,…"}}]`;
  - sem imagens, o corpo **não muda** (compatibilidade com os providers
    atuais).
- `src-tauri/src/adapters/ai/gemini_chat_bot_provider.rs`:
  - `parts` ganha `{"inline_data":{"mime_type":…,"data":<base64>}}`.
- `src-tauri/src/application/ask_service.rs`: a imagem anexada entra no
  pedido e é gravada em `journal_images` (task 01).
- `src/features/journal/JournalEntryCard.tsx`: mostra o print anexado como
  miniatura (ícone de imagem) na entrada do diário.

## Critérios de aceite

- [x] Um pedido **sem** imagem produz exatamente o mesmo corpo HTTP de hoje
      (nenhuma regressão nos providers).
- [x] Um pedido **com** imagem manda `image_url` (OpenAI-compatível) e
      `inline_data` (Gemini), com o `mime` e o base64 corretos.
- [x] A imagem enviada é a **comprimida** (≤1280 px, JPEG q0.86), não a
      original do display.
- [x] Provider que rejeita `image_url` → erro **claro** na entrada do
      diário ("esse modelo não aceita imagem"), sem retry cego (R-04).
- [x] O print fica **gravado** no diário (miniatura recuperável depois).
- [x] `cargo test --lib` verde: construção do corpo com e sem imagem, nos
      dois formatos.

## Verificação

- `cargo test --lib` ✓ · `CompletionRequest.images` + `image_url`/`inline_data`

## Notas

- O `CompletionRequest` é usado por resumo, sugestão e tradução: as **imagens
  são opcionais** e o caminho de texto fica **intacto**.
- Privacidade (BR-005): a imagem sai **só** para a fonte escolhida pelo
  usuário, como o texto.
- O caminho da OpenAI é `chat/completions` (não a Responses API): o formato
  multimodal é o `content` em lista, suportado por OpenAI, OpenRouter e
  compatíveis.
