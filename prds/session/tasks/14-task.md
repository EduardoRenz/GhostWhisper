# 14 — Persistência da barra Perguntar só por arraste explícito

- **Status:** CONCLUDED
- **Depende de:** 12, 13; BR-072
- **Issues:** ISS-057 · **BR:** BR-072 · **Rules:** R16, R17
- **Refs:** [`../issues.md`](../issues.md),
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §8

## Objetivo

Impedir que a posição automática imposta pelo WM (ex.: `[0,32]` ao remapear)
sobrescreva a última posição que o usuário deixou para a barra “Perguntar”.
Só **arraste explícito** pode persistir uma nova posição.

## Diagnóstico

- `ask_position` estava correto em `[567,32]`; depois do teste virou `[0,32]`.
- O tracker de 700 ms e o `persist_ask_position` antes de `hide`/captura
  gravavam a posição atual sem saber a origem. Uma posição transitória do WM
  era tratada como escolha do usuário.
- A ISS-055 corrigiu o `show` e adicionou a posição persistida, mas não isolou
  a **fonte** da observação.

## Entregáveis

- `domain/settings.rs`:
  - `AskPositionSource` (`Automatic`, `UserDrag`, `InitialPlacement`);
  - `Settings::apply_ask_position` — `Automatic` nunca persiste; `UserDrag`
    substitui; `InitialPlacement` só preenche vazio;
  - testes anti-regressão das duas regras.
- `ipc/ask_commands.rs`:
  - `AskDragState` (Tauri managed state) com expiração de 2,5 s;
  - `ask_start_drag`, usado pela alça da barra;
  - tracker/persistência classificam a origem e reafirmam a posição salva
    depois do `show`.
- `src/adapters/ipc/ask.ts` + `AskWindow.tsx`: alça usa `askStartDrag`.
- `lib.rs`: registra o estado e o comando; tracker usa `observe_ask_position`.
- Regras R16/R17 em `.agents/rules.md`.

## Critérios de aceite

- [x] Movimento automático do WM não altera `ask_position`.
- [x] Arraste explícito continua persistindo a posição.
- [x] Primeira colocação centraliza e grava somente se não houver posição.
- [x] Fechar/capturar imediatamente após arrastar não perde a posição.
- [x] Reabrir restaura a última posição; o WM não deixa a barra no canto.
- [x] Teste de regressão falha se a posição automática voltar a ser aceita.

## Verificação

- `cargo test --lib` **600 ✓** (10 ignorados) · clippy `-D warnings` ✓ ·
  rustfmt dos arquivos tocados ✓.
- `pnpm test` **388 ✓** · `tsc --noEmit` ✓.
- Regressão simulada no AppImage: barra em `[567,32]`, movida para `[0,32]`
  **sem arraste**; após 1,8 s o `settings.json` continuou `[567,32]`; reabrir
  restaurou `[567,32]`.
- Arraste explícito no AppImage: alça em `596,60` → `300,300`; a janela foi
  para `271,272` e `ask_position` acompanhou; fechar, reabrir e capturar
  mantiveram `271,272` (e o `_NET_WM_STATE_ABOVE` foi reafirmado).
- Estado do usuário restaurado do backup (`ask_position = [567,32]`).
- Pacotes finais em `src-tauri/target/release/bundle` (15:52).
