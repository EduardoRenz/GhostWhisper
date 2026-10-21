# 09 — Configurações: aba "Gravação"

- **Status:** PENDING
- **Depende de:** 01, 05
- **Habilita:** 10
- **Issues:** ISS-049 · **BR:** BR-066, BR-070
- **Refs:** [`../prd.md`](../prd.md) US-04 (CA-04.1…CA-04.8);
  [`../architecture/recording.md`](../architecture/recording.md) §10

## Objetivo

Dar controle de custo e qualidade da gravação, com padrões que já funcionam na
máquina de referência e sem quebrar configurações antigas.

## Entregáveis

- `src-tauri/src/domain/settings.rs`:
  - campos `recording_mode`, `recording_target`, `recording_auto_start`, `recording_fps`,
    `recording_resolution`, `recording_quality`, `recording_encoder`,
    `recording_audio_kbps`, `recording_container`, `recording_dir`,
    `recording_keep_segments`, todos com `#[serde(default = …)]`;
  - `normalize_recording_settings(&mut Settings)` (função pura): FPS no degrau
    mais próximo, kbps clampado, allowlists, pasta padrão
    (`$XDG_VIDEOS_DIR/GhostWhisper` → `~/Vídeos/GhostWhisper`).
- `src/core/domain/settings.ts`: espelho dos campos + `defaultSettings()`.
- `src/features/settings/RecordingSettings.tsx` + entrada em `SettingsPage.tsx`
  (**6ª aba**, entre "Transcrição & Áudio" e "Assistente").
- `src/adapters/ipc/settings.ts`: `recordingCapabilities()` (encoder detectado,
  `ffmpeg` encontrado, motivo).
- `src/core/domain/recording.ts`: `estimateSizePerHour(settings)` puro, usado
  no card de custo.
- `src/features/settings/RecordingSettings.test.tsx`.

## Critérios de aceite

- [ ] A aba existe na posição definida e sobrevive ao reload (`PanelTab`/settings).
- [ ] Cards: **O que gravar**, **Vídeo** (oculto no modo áudio), **Áudio**,
      **Arquivo**, **Custo estimado**.
- [ ] Defaults exatamente os de CA-04.3 (janela ativa · 1080p nativo · 8 fps ·
      CRF 27 · veryfast · AAC 96k · MP4 · pasta XDG).
- [ ] "Somente áudio" esconde o card de vídeo e mostra `FLAC sem perda · mono
      48 kHz`.
- [ ] Faixas válidas (CA-04.4) e normalização de valores inválidos no backend
      (CA-04.5).
- [ ] Configuração antiga (sem os campos) carrega com os defaults, sem erro.
- [ ] A estimativa muda ao alterar FPS/qualidade/modo (`≈ 40–200 MB/h`).
- [ ] Se o encoder não estiver disponível, o card mostra o motivo e o Salvar
      mantém, caindo para x264 com aviso (CA-04.7).
- [ ] Alterar settings com sessão ativa mostra "aplica-se à próxima sessão".
- [ ] `pnpm test` cobre defaults, faixas e estimativa.

## Verificação

- `cargo test --lib` (normalização) · `pnpm test` · `pnpm build`

## Notas

- Não criar aba "vídeo" separada: gravação é mídia, e a aba fica ao lado de
  "Transcrição & Áudio", que é o assunto mais próximo.
