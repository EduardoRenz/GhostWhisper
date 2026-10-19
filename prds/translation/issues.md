# Issues — PRD Translation

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-029**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-029 | Painel de tradução duplicava a conversa embaixo | Alta | FIXED |
| ISS-030 | Origem `Auto` + destino pt traduzia pt→pt (idioma detectado ignorado) | Média | OPEN |

---

## ISS-029 — Painel de tradução duplicava a conversa embaixo

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** um "segundo chat" aparecia abaixo da conversa, repetindo o texto
  original com a tradução ao lado.
- **Causa raiz:** `TranslationView` renderizava todas as falas traduzidas num
  painel fixo abaixo das bolhas, mesmo sem tradução (com placeholder).
- **Correção:** painel removido; a tradução passou a ser **sob demanda** dentro da
  bolha (botão no canto), com cache e switch de volta (BR-030).
- **Arquivos:** `Overlay.tsx`, `ChatView.tsx`, `ChatBubble.tsx`,
  `useTranslation.ts`, `globals.css`; `TranslationView.*` removidos.
- **Task:** [`tasks/01-task.md`](tasks/01-task.md)

## ISS-030 — Origem `Auto` + destino pt traduzia pt→pt

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** com "Idioma de origem" em **Auto** e fala em português, o app
  traduzia mesmo com o destino também português.
- **Causa raiz:** o gate era estático (`target !== source`); com `source = "auto"`
  ele nunca reconhecia que a fala já estava no idioma de destino, porque o idioma
  detectado pelo Whisper não chegava à UI.
- **Correção:** T2 — expor o idioma do passe final do ASR e gatear a tradução por
  ele (normalizado), de forma conservadora.
- **Arquivos:** `whisper_speech_to_text.rs`, `transcription_pipeline.rs`,
  `domain/transcription.rs`, `transcript.ts`, `useTranslation.ts`, `ChatBubble.tsx`
- **Task:** [`tasks/02-task.md`](tasks/02-task.md)

---

<!--
Modelo:

## ISS-0NN — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`tasks/NN-task.md`](tasks/NN-task.md)
-->
