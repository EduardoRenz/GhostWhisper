# 12 — Janelas não se movem sozinhas; barra “Perguntar” preserva posição

- **Status:** CONCLUDED
- **Depende de:** 02, 06; BR-052/BR-055
- **Issues:** ISS-055 · **BR:** BR-052, BR-072
- **Refs:** [`../prd.md`](../prd.md) US-00,
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §8

## Objetivo

Garantir que **nenhuma janela do app se reposicione sozinha**. A barra
“Perguntar” era o caso visível: depois da captura de tela — que esconde e
restaura todas as janelas — ela voltava para o topo centralizado, mesmo tendo
sido arrastada. A posição só pode mudar por **arraste explícito** da alça (ou
pela colocação inicial/âncora configurada, nas exceções da BR-072).

## Diagnóstico

- `show_ask_window` chamava `center_ask_window` **antes e depois** do `show`
  sempre que a janela estava escondida (revisão 5 da BR-052).
- `screen_capture` esconde todas as janelas visíveis e restaura ao final; o
  `AskWindow` então chama `askOpen()` para garantir a barra de volta/focada —
  isso disparava o recentramento.
- A barra não tinha posição persistida: não havia `ask_position` em `Settings`
  nem tracker, então a posição real só vivia no processo atual.

## Entregáveis

- `src-tauri/src/ipc/ask_commands.rs`:
  - `show_ask_window` reaplica a `ask_position` salva **antes e depois** do
    `show` (o WM mapeia a janela oculta em `0,0`); só centraliza quando ainda
    não existe posição salva;
  - `restore_ask_window_position(saved)` restaura a posição no boot;
  - `persist_ask_position` grava o último ponto antes de `ask_hide`/captura.
- `src-tauri/src/domain/settings.rs`:
  - novo `ask_position: Option<(i32, i32)>` com `#[serde(default)]`;
  - `preserve_window_geometry` inclui `ask_position`;
  - teste de geometria preservada.
- `src-tauri/src/lib.rs`:
  - restaura `ask_position` no setup;
  - `spawn_ask_position_tracker` persiste a posição enquanto a barra está
    visível (sem reposicioná-la).
- `src-tauri/src/ipc/screen_commands.rs`:
  - guarda `(posição, tamanho)` de cada janela visível antes do `hide` da
    captura e reaplica antes/depois do `show`, mantendo todas no mesmo lugar.
- `src/core/domain/settings.ts`: campo `ask_position` no contrato e no
  `defaultSettings`.
- `prds/business-rules.md`: BR-052 revisão 6 + BR-072 (regra geral).
- `prds/session/issues.md` ISS-055, PRD/arquitetura/memory e esta task.

## Critérios de aceite

- [x] Arrastar a barra e capturar a tela **não** altera a posição dela.
- [x] Esconder/mostrar a barra (Esc/atalho/lupa) mantém a posição.
- [x] Salvar settings de qualquer outra janela não reposiciona a barra
  (`preserve_window_geometry`).
- [x] Reiniciar o app restaura a última posição arrastada.
- [x] Sem posição salva, a primeira colocação continua centralizada no topo.
- [x] O WM pode mapear a janela oculta em `0,0`; a posição salva é reaplicada
  antes/depois do `show` e a barra não fica no canto.
- [x] A captura de tela restaura a geometria de todas as janelas, sem
  centralizar nem usar a posição transitória do WM.
- [x] `cargo test`, clippy e `rustfmt` nos arquivos tocados; UI sem regressão.

## Como testar

1. Abrir a barra pelo atalho/lupa, arrastar pela alça para longe do centro.
2. Capturar a tela (Alt+C ou botão) e esperar a barra voltar.
3. Confirmar que ela permaneceu no ponto arrastado.
4. Fechar/reabrir a barra e salvar settings em outra janela; ela não se move.
5. Reiniciar o app; ela volta ao último ponto do passo 1.

## Verificação

- `cargo test --lib` **596 ✓** (10 ignorados) · `cargo clippy --lib -D warnings` ✓
  · `rustfmt` nos arquivos Rust tocados ✓.
- `pnpm test` **387 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- AppImage (Xvfb): barra aberta em `440,12`, movida para `300,200`; captura de
  tela esconde/restaura e a barra volta em `300,200` (sem recentralizar); após
  reiniciar, abre em `300,200` — `ask_position` persistido. Estado do usuário
  restaurado do backup ao final.
- **Follow-up:** o teste do usuário no WM real mostrou o mapeamento em `0,0` ao
  reabrir. No AppImage corrigido:
  - barra em `398,343`, escondida e movida para `0,0` enquanto oculta;
  - ao reabrir, voltou para `398,343` (posição salva reaplicada antes/depois
    do `show`);
  - captura de tela manteve `398,343` e a prévia apareceu no canto;
  - arraste para `250,450` seguido de `Esc` imediato persistiu `250,450` e
    reabriu nesse ponto (não depende do tracker de 700 ms).
- Pacotes finais em `src-tauri/target/release/bundle` (15:04).
