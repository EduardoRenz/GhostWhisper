# 07 — UI do overlay (transcrição ao vivo + ActionBar)

- **Status:** CONCLUDED
- **Depende de:** 01, 04, 05, 06
- **Habilita:** 08

## Objetivo
Construir a UI do overlay: transcrição ao vivo, medidores de áudio e uma ActionBar
com ações essenciais, integrada aos eventos do core e ao click-through.

## Entregáveis (implementados)
- Core (frontend): `core/domain/transcript.ts`; hooks
  `core/application/{useLiveTranscription,useAudioLevel,useSessionState}.ts`;
  `useInteractiveRegions.ts` reescrito (MutationObserver + rAF).
- Adaptadores IPC: `adapters/ipc/{events.ts,session.ts,overlay.ts}`.
- Features: `features/overlay/{Overlay,ActionBar,LevelMeters}.tsx`,
  `features/transcript/TranscriptView.tsx`,
  `features/session/SessionControls.tsx`; tema em `styles/globals.css`.
- Core (Rust): eventos `audio_level`/`session_state` (novos) e
  `transcript_segment`; `EventPublisher` estendido; pipeline emite nível;
  `SessionService` emite estado; `OverlayWindowController::set_surface`
  (actionbar/expanded/hidden) + IPC `overlay_set_surface`.

## Critérios de aceite
- [x] Transcrição aparece e atualiza em tempo real (assinatura de
  `transcript_segment`; lista limitada a 60 itens).
- [x] Medidores de nível (canal ativo) funcionam (`audio_level`).
- [x] ActionBar realiza start/stop de sessão (`session_start/stop`).
- [x] Regiões não interativas são click-through (regiões publicadas a cada
  mudança de DOM; core decide).
- [x] Tema legível sobre qualquer fundo (fundo translúcido + bordas claras).

## Evidências
- `cargo test` → **66 testes** (4 novos no controlador de click-through;
  pipeline emite nível; sessão emite estado). `clippy`/`fmt` limpos.
- `pnpm test` → **6 testes** (render da transcrição com ME/THEY).
- `pnpm build` ok; smoke run: app inicia com as duas janelas.
- Fluxo de eventos já validado de ponta a ponta na Tarefa 04 (ASR → evento).

## Notas / decisões
- **ISS-001 (clique perdido) corrigido** — ver `../issues.md`: entrada na região
  é imediata, saída é debounced (3 amostras), margem de 8px e polling a 30ms;
  janela começa interativa até a UI publicar regiões.
- Um único medidor de nível: o core emite `audio_level` do canal ativo (mic ou
  sistema), evitando misturar fontes.
- Superfícies: `actionbar` (900×72) e `expanded` (900×380), reancorando pela
  base; `hidden` oculta a janela.
- Validação visual em sessão real (latência/legibilidade) é manual e fica
  registrada como recomendação de teste.
