# 02 — Gate por idioma detectado · Entregável T2

- **Status:** PENDING
- **Depende de:** 01 (botão na bolha)
- **Issues:** ISS-030 · **BR:** BR-030

## Objetivo

Não oferecer tradução quando a fala já está no idioma de destino — usando o
idioma que o próprio Whisper detecta (origem `auto`), sem heurística no cliente.

## Entregáveis

- `src-tauri/src/domain/transcription.rs` — o passe final carrega o idioma
  detectado (`Option<String>`, `#[serde(default)]`).
- `src-tauri/src/adapters/speech/whisper_speech_to_text.rs` — lê
  `state.full_lang_id_from_state()` + `get_lang_str(id)` no passe **final**.
- `src-tauri/src/application/transcription_pipeline.rs` — propaga o idioma do
  turno; o evento de segmento o carrega.
- `src/core/domain/transcript.ts` — espelha `language?: string | null`.
- `src/features/chat/ChatView.tsx`/`ChatBubble.tsx`/`Overlay.tsx` — não oferecer o
  botão quando `normalize(segment.language) === normalize(target)` (conservador:
  só omite quando o idioma detectado é conhecido e igual).
- `prds/business-rules.md` — **BR-030** (tradução sob demanda + gate por idioma).

## Critérios de aceite

- [ ] O idioma detectado chega à UI no segmento final (origem `auto`).
- [ ] Fala em pt com destino pt **não** mostra o botão de tradução.
- [ ] Origem fixa (ex.: pt) e destino diferente (en) continua traduzindo.
- [ ] Sem detecção confiável (ou idioma `None`), o botão continua disponível.
- [ ] Nenhuma chamada de tradução automática em background (BR-005).
- [ ] `cargo test`, `clippy`, `fmt`, `pnpm test`, `tsc`, `build` ✓.

## Como testar

1. Idioma de origem **Auto**, destino **Português**; fale português: o ícone de
   tradução **não** deve aparecer (ou ficar desabilitado).
2. Idioma de origem **Auto**, destino **English**; fale português: o ícone
   aparece e traduz ao clicar.
3. Checar no log/evento que o segmento final carrega o idioma (`pt`).

Ponto de parada: usuário aprova; PRD encerrada.

## Riscos / rollback

- Modelo `tiny` detecta mal em áudio curto → gate conservador e guarda de
  identidade (T1); melhora com `large-v3-turbo` (PRD precision-and-performance/E3).
- Persistência: o idioma é do **evento** (efêmero) e não altera o schema (RNF-03 do
  PRD chat).

## Verificação (preencher ao concluir)

- Testes: …
- Observações do usuário: …
