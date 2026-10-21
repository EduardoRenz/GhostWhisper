# 05 — Prompt como contexto: default vazio e aviso nas configurações

- **Status:** IN PROGRESS (código e testes prontos; falta validar a UI no app)
- **Depende de:** 01 (prompt/idioma) e 03 (engine)
- **Issues:** ISS-059 · **BR alterada:** BR-027 (BR-034 ajustada)
- **Refs:** BR-015, BR-027, BR-034, R16, R17,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md) §2

## Objetivo

Impedir que o `initial_prompt` vire **texto inventado**. O whisper trata o
prompt como *fala anterior*: uma instrução (“não invente”, “transcreva só o que
estiver claro”) é continuada quando o áudio é incerto — foi a causa das “frases
sem sentido” no microfone.

## Entregáveis

- `src-tauri/src/domain/settings.rs` — `DEFAULT_WHISPER_PROMPT` **vazio**;
  migração em `sanitized()` limpa quem ainda tem o default antigo (instrucional),
  preservando contexto real.
- `src-tauri/src/domain/transcription.rs` — `decode_prompt(base)` só sanitiza
  (trim + NUL); a **regra de idioma saiu** (o `set_language` +
  `set_translate(false)` já fixam idioma e não-tradução).
- `src-tauri/src/adapters/speech/whisper_speech_engine_factory.rs` — prompt não
  depende mais do idioma; `update_language` não recomputa prompt.
- `src/core/domain/settings.ts` — default vazio.
- `src/features/settings/SettingsPage.tsx` — campo renomeado para **“Contexto da
  transcrição (opcional)”**, placeholder de nomes/jargões e aviso de que
  instruções **atrapalham** (podem ser repetidas) e que o recomendado é vazio.
- `prds/business-rules.md` — BR-027 reescrita; BR-034 ajustada.

## Critérios de aceite

- [x] Default vazio (backend e frontend) e migração limpa o texto antigo.
      **Testes:** `whisper_prompt_defaults_to_empty`,
      `sanitized_clears_the_legacy_instruction_prompt`.
- [x] Prompt sem regra de idioma e apenas como contexto.
      **Testes:** `decode_prompt_keeps_only_context_and_drops_instructions`,
      `decode_prompt_handles_empty_and_null_bytes`.
- [x] Idioma continua sendo aplicado no motor em tempo real (BR-034).
      **Teste:** `update_language_switches_the_cached_engine_without_reloading`.
- [x] UI com aviso explícito de cautela (texto revisado em
      `SettingsPage.tsx`).
- [ ] Teste manual: abrir Configurações e conferir rótulo/aviso; sessão curta
      com o campo vazio.

## Como testar

1. `cargo test --lib` (backend) e `pnpm test`/`tsc` (frontend).
2. Abrir Configurações → Transcrição: o campo deve estar **vazio**, com o rótulo
   “Contexto da transcrição (opcional)” e o aviso de que instruções atrapalham.
3. Repetir uma fala curta e conferir que não há repetição/alucinação na prévia
   nem no final.
4. (Opcional) cadastrar apenas “Grafana, Kubernetes” e confirmar que a
   transcrição segue normal; cadastrar uma instrução e observar a degradação —
   o aviso existe justamente para desencorajar isso.

## Verificação automatizada

- `cargo test --lib` **619 ✓** (10 ignorados) · clippy `-D warnings` ✓.
- `pnpm test` **388 ✓** · `tsc --noEmit` ✓.
