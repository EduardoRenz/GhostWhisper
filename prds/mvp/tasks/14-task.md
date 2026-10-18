# 14 — Sensibilidade do microfone (ganho) e noise gate de entrada

- **Status:** CONCLUDED
- **Depende de:** 02, 13
- **Habilita:** 15 (monitor atrasado)

## Objetivo
O usuário relatou **sensibilidade muito baixa** no microfone: a fala quase não
aparecia e o nível exibido era baixo. Causa: o GhostWhisper não aplicava **nenhum
ganho** e o gate de fala é um limiar **absoluto** no sinal cru
(`SPEECH_RMS_THRESHOLD`), então microfones quietos nunca cruzavam o limiar.
Fecha a BR-018 (padrão do Perssua), com o controle pedido pelo usuário.

## Entregáveis
- `domain/audio.rs` — `apply_gain` (satura em ±1), `clamp_mic_gain`
  (1×–12×, padrão 8×), `NoiseGate` (mudo + *hangover*), `SPEECH_RMS_THRESHOLD`
  como **fonte única** do limiar de fala.
- `adapters/audio/microphone_front_end.rs` — decorador `MicrophoneFrontEnd`
  sobre `AudioSource`: **ganho → gate** antes do pipeline, só no microfone;
  limiar/hangover do gate com override por ambiente
  (`GHOSTWHISPER_MIC_NOISE_GATE_RMS` — `0` desliga — e
  `GHOSTWHISPER_MIC_NOISE_GATE_HANGOVER_MS`).
- `domain/settings.rs` — `mic_gain` persistido (padrão 8×), `sanitized()` limita
  a faixa, config antiga sem o campo herda o padrão.
- `lib.rs` — configurations antes do áudio; microfone passa pelo front-end.
- `features/settings/SettingsPage.tsx` — controle **“Sensibilidade do
  microfone”** (1×–12×, passo 0,5, marca o padrão) na aba *Transcrição & Áudio*.
- `features/settings/AudioTestPanel.tsx` — mostra o ganho aplicado e explica o
  silêncio do gate.

## Referência (BR-015, REA do Perssua 0.27.5)
- Ganho (`dy`, `main.f180d025.js`): `clamp(cfg local_whisper_user_mic_gain,
  padrão 8, mínimo 1, máximo 12)`, **só** quando o falante é o mic do usuário.
- Gate (`Kv`): passa se `rms >= threshold`; senão passa em *hangover*
  (`speaking && Δ ≤ hangover_ms`); senão **mudo** (`muteInsteadOfDrop`, buffer
  zerado). Defaults do local-whisper: `threshold 0.025`, `hangover 600 ms`,
  `muteSilenceDurationMs 1200`.
- Ver [`architecture/transcription-streaming.md`](../architecture/transcription-streaming.md) §6.

## Critérios de aceite
- [x] Ganho padrão 8× (faixa 1×–12×) aplicado **antes** do gate e do ASR, só no mic.
- [x] Saturação em ±1,0 (nunca estoura a faixa do whisper) e identidade em 1×.
- [x] Ruído abaixo do limiar é **silenciado** (não descartado), preservando a
      linha do tempo que o pipeline usa para fechar o turno.
- [x] Fala fraca passa a cruzar o gate de fala depois do ganho (teste).
- [x] Config antiga sem `mic_gain` não quebra (herda 8×).
- [x] `cargo test` 162 ✓ · `clippy`/`fmt` ✓ · `pnpm test` 23 ✓ · `tsc` ✓.

## Verificação
- Testes novos (Rust): ganho (amplifica/satura/1× é identidade/valores inválidos),
  ganho levanta fala fraca acima do limiar, gate (passa fala, silencia ruído,
  hangover de pausa curta, mudo antes da primeira fala, desligável, limiar
  inválido), front-end (ganho antes de repassar, gate, delegação, leitura das
  configurações a cada `start`, defaults/env do gate), settings (padrão 8×,
  `sanitized` limita, round-trip e config antiga).
- Teste novo (frontend): o slider existe com min/max/padrão e atualiza o valor.

## Pendências
- **AGC** (ajuste automático ao RMS alvo): descartado por ora — o ganho manual
  resolve o caso relatado com menos risco.
- O limiar do gate usa o limiar de fala do app, não os 0,025 do Perssua (ver §6
  da tech spec: 0,025 silenciaria fala que hoje é transcrita).
