# 22 — Interface responsiva: IPC fora da main thread e menos re-render

- **Status:** CONCLUDED
- **Depende de:** 01, 07, 11
- **Habilita:** —

## Objetivo

Eliminar o travamento/engasgo da interface relatado em uso: *"a main-thread sendo
segurada, não conseguia clicar nos botões, algumas vezes o app até congelava"*
(ISS-020).

## Causas (medidas no código, não deduzidas)

1. **Comando de IPC síncrono roda na main thread.** O IPC chega na main thread (loop
   GTK) e `#[tauri::command]` **sem `async`** chama a função ali mesmo
   (`tauri-macros`: `body_blocking`). Comandos que demoram segundos: ciclo de sessão
   (`pipeline.stop()` espera o passe final — `FLUSH_WAIT = 20 s`), LLM/validação de
   chave (`ureq` **bloqueante**, timeout **60 s**), áudio (`cpal`, `pactl`), keyring
   (DBus), banco (concorrente com o ASR) e export (arquivo).
2. **Tempestade de IPC pelo input-shape.** `useInteractiveRegions` observava
   `class`/`style` no `body` inteiro; a barra de nível muda de largura ~10×/s por
   canal ⇒ ~20 publicações/s de regiões para o X11 (cada uma um comando síncrono).
3. **Re-render a 20 Hz.** `useAudioLevel` no `Overlay` fazia cada evento de nível
   re-renderizar a conversa inteira (até 300 bolhas).
4. **Atalhos/bandeja.** O menu da bandeja roda na main thread (GTK) e o listener de
   atalhos do X11 na thread dele; ambos entram no mesmo `dispatch_shortcut`, que
   chama `session.stop()` (flush ≤20 s) e `insight.generate()` (HTTP ≤60 s).

## Entregáveis

- **37 comandos** com `#[tauri::command(async)]` (sessão, áudio/sistema, LLM,
  settings/chave, export, banco, `overlay_set_interactive_regions`) — executam no
  runtime do Tauri, não na main thread. Síncronos apenas os instantâneos (`ping`,
  getters) e os que exigem a main thread (`settings_open`, operações de janela).
- `dispatch_shortcut`: `ToggleSession` e `RequestInsight` vão para uma **thread
  própria**; `ToggleOverlay`/`ToggleStealth` (instantâneos) seguem na thread do
  evento.
- `HistoryPanel`: conversão da linha temporal em bolhas memoizada (`useMemo`).
- `useInteractiveRegions`: mutações dentro de `.levels-group` **não** republicam as
  regiões (a barra de nível não é região clicável).
- `LevelMeters` assina `audio_level` internamente e é memoizado; `ChatView` é
  memoizada e o cabeçalho ("Sugerir") também, para o `Overlay` poder re-renderizar
  sem reconstruir a conversa.
- Regra escrita uma vez em
  [`architecture/engineering-guidelines.md`](../architecture/engineering-guidelines.md) §8
  (R14) e referenciada pelo issues/task.
- `dispatch_shortcut` (bandeja/atalho): sessão e insight em thread própria.

## Critérios de aceite

- [x] Nenhum comando que possa demorar roda na main thread.
- [x] Ações demoradas do atalho/bandeja não rodam na thread que recebeu o evento.
- [x] A barra de nível mudando **não** republica as regiões interativas.
- [x] Mudança real em uma região clicável continua republicando (click-through vivo).
- [x] Evento de nível não re-renderiza a conversa.
- [x] Comportamento funcional preservado (mesmos comandos, mesmos retornos).

## Verificação (2026-10-18)

- `cargo check --all-targets` + `cargo test` (225) + clippy/fmt limpos.
- Frontend: 6 testes novos — `useInteractiveRegions` (publica ao montar; **não**
  republica quando só a barra muda; republica quando uma região interativa muda) e
  `LevelMeters` (nível por canal, zerado com captura parada, sem canais). Suíte: 80;
  `tsc --noEmit` limpo.

## Pendências

- Se surgirem comandos longos **concorrentes**, trocar o corpo para
  `async_runtime::spawn_blocking` (pool dedicada) — hoje o corpo `(async)` ocupa um
  worker do runtime.
- Medir em uso real (perfil com `RUST_LOG`/DevTools) para confirmar o ganho percebido.
