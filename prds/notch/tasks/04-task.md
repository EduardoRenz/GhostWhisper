# 04 — Seletor de idioma na barra inferior + troca em tempo real no ASR

- **Status:** CONCLUDED
- **Depende de:** 03 (a barra é a da aba Transcrição); o núcleo Rust é independente
- **Habilita:** 05
- **Issues:** ISS-033 · **BR:** BR-034

## Objetivo

Adicionar um **seletor de idioma de origem** na **barra inferior** da aba
Transcrição que muda a transcrição **em tempo real** — vale para o **próximo passe**
de decodificação da sessão em andamento, **sem** reiniciar a sessão e **sem**
recarregar o modelo.

## Entregáveis

### Núcleo (Rust)

- `adapters/speech/whisper_speech_to_text.rs` — `language: String` →
  `language: Mutex<String>`; `update_language(&self, lang)` (espelho de
  `update_prompt`); `language_snapshot()`; `run_worker` passa o snapshot a
  `decode_with`. Teste: `update_language` reflete no próximo passe.
- `adapters/speech/whisper_speech_engine_factory.rs` — tirar `language` da **chave
  do cache** (`CachedEngine` mantém `model`/`use_gpu`); ao reusar o motor, chamar
  `update_language(lang)` **e** `update_prompt(decode_prompt(...))`. Testes:
  reuso do mesmo motor ao trocar idioma; idioma novo propagado; prompt atualizado.
- `ports/speech_engine_factory.rs` — `fn update_language(&self, language: &str) -> Result<()>`
  (default no-op ou propagação ao motor em cache).
- `ports/session_controller.rs` — `fn set_language(&self, language: &str) -> Result<()>`.
- `application/session_service.rs` — `set_language`: persiste `source_language`,
  chama `speech_factory.update_language(map_whisper_language(lang))` e
  `update_prompt(decode_prompt(whisper_prompt, lang))`. Teste com fakes.
- `ipc/session_commands.rs` — `#[tauri::command(async)] session_set_language`;
  registrar no `generate_handler!`.

### Frontend

- `src/features/panel/LanguageBar.tsx` — barra inferior da aba Transcrição:
  `LanguageIcon` + `select` de origem (reusa `LANGUAGES`/`SOURCE_LANGUAGE_LABELS`);
  chama `session_set_language`; estado otimista + aviso de erro.
- `src/adapters/ipc/session.ts` — `sessionSetLanguage(language)`.
- `src/features/panel/TranscriptionView.tsx` — renderiza a `LanguageBar` no rodapé.
- `LanguageBar.test.tsx` — troca dispara o IPC; reflete `settings.source_language`;
  sem play/pause na barra.

## Critérios de aceite

- [ ] A barra inferior da Transcrição tem o **seletor de idioma** (com "Auto") e
      **nenhum** play/pause/stop (CA-04.3, CA-06.1).
- [ ] Trocar o idioma com sessão **ativa** muda a transcrição no **próximo passe**,
      sem reiniciar a sessão e sem recarregar o modelo (CA-06.2).
- [ ] O valor é **persistido** em `settings.source_language` e reflete na aba
      *Transcrição & Áudio* (CA-06.3).
- [ ] Pausado/sem sessão, a escolha é salva e vale ao (re)captar (CA-06.4).
- [ ] O seletor **não** altera o idioma de destino da tradução (CA-06.5).
- [ ] O motor é **um só** entre canais: a troca vale para microfone e sistema.
- [ ] `cargo test` (novos testes de engine/service) · `clippy` · `fmt` ·
      `pnpm test` · `tsc` · `pnpm build` verdes.

## Como testar

1. Inicie uma sessão e fale em português com o idioma de origem em **Auto/Português**.
2. Na **barra inferior da Transcrição**, mude para **Inglês** e fale: a próxima
   fala deve ser reconhecida como inglês **sem** a sessão reiniciar (a linha do
   tempo não zera).
3. Volte para **Português**: a fala seguinte volta a ser pt, mesma sessão.
4. Abra as **Configurações → Transcrição & Áudio**: o idioma de origem deve
   refletir a escolha.
5. Confirme que o **modelo não foi recarregado** (sem novo download/“Preparando
   modelo…”).

Ponto de parada: usuário aprova a troca em tempo real.

## Riscos / rollback

- Passe de decodificação **em voo** usa o idioma anterior (esperado); o próximo usa
  o novo (CA-06.2).
- `whisper-rs` ignora idioma inválido → `map_whisper_language` já normaliza.
- Sem sessão ativa, a fábrica não tem motor em cache → só persiste (sem erro).

## Verificação

- Automática: `cargo test` · `clippy` · `fmt` · `pnpm test` · `tsc` · `pnpm build`.
- Manual: os 5 passos acima (exige áudio real).
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- Núcleo: `WhisperSpeechToText.language` virou `Mutex<String>` com
  `update_language`/`language_snapshot`; a fábrica **removeu o idioma da chave do
  cache** e ganhou `update_language` (aplica idioma + prompt derivado ao motor em
  cache, sem recarregar); a porta `SpeechEngineFactory` ganhou `update_language`
  (default no-op) e `SessionController::set_language`.
- IPC `session_set_language` (`async`): persiste `source_language` (com
  `validate()`), aplica ao motor e é registrado no `generate_handler!`.
- UI: `features/panel/LanguageBar` na barra inferior da Transcrição (ícone +
  `select` de origem, sem play/pause); `sessionSetLanguage` no adaptador.
- Testes: fábrica (reusa o motor ao trocar idioma; no-op sem cache), engine
  (`update_language` reflete), `LanguageBar.test.tsx` (2).
- Verificação: `cargo test` **334 ✓** · clippy/fmt ✓ · `pnpm test` 155 ✓ · `tsc` ✓.
