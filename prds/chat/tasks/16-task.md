# 16 — Intercalação por sobreposição e grip padrão

- **Status:** CONCLUDED
- **Depende de:** 11, 15
- **Habilita:** —
- **Issues:** — · **BR:** BR-043
- **Refs:** [`../prd.md`](../prd.md) §9 (US-14),
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3.4
- **Revisão:** a **20** (ISS-075) tornou o corte conservador — só intercala fala
  longa com interrupção real e partes legíveis.

## Objetivo

Corrigir a linha do tempo quando um canal fala **sem parar** e o outro começa
no meio: a fala longa é cortada no início do outro falante, a nova mensagem entra
no meio e o restante continua numa nova bolha logo depois. Vale para o ao vivo e
para o histórico. Também padronizar o **grip de arraste** no canto superior
direito de todas as janelas.

## Entregáveis

- `src/core/domain/chat.ts`:
  - `splitSpeechSpans`: encontra inícios do **outro falante** dentro do intervalo
    de uma fala e divide o texto proporcionalmente ao tempo, preferindo fim de
    frase/palavra;
  - a primeira parte mantém o `at_ms` original; as seguintes entram **1 ms
    depois** do início do outro, para a ordenação colocar a nova fala no meio;
  - cada parte vira uma bolha própria (rótulo/hora repetidos); o resto **não** é
    uma continuação silenciosa;
  - se não houver corte seguro sem perder texto, mantém a fala inteira;
  - aplicado no `buildChatTimeline` (ao vivo) e no `bubblesFromTimeline`
    (histórico, usando `end_ms`).
- `src-tauri/src/application/session_timeline.rs` e TS `TimelineEntry`:
  - a fala do histórico carrega `end_ms`, necessário para o mesmo corte.
- `src/features/panels/PanelFrame.tsx`:
  - `DragHandle` visível no **canto superior direito** de todas as janelas
    (antes do fechar); o heading continua arrastando.
- Testes: intercalação simétrica, sem sobreposição, histórico com `end_ms`.

## Critérios de aceite

- [x] “Outros” falando por 20 s e “Microfone” começando aos 10 s → a fala de
      Outros vira duas bolhas com a fala do Microfone **entre** elas.
- [x] O mesmo vale no sentido inverso e no histórico.
- [x] Nenhuma parte vazia/perdida; se o corte não for seguro, a fala fica
      inteira.
- [x] O grip de arraste aparece no topo direito de Chat, Sugestões e Histórico.
- [x] `pnpm test` 265 ✓ · `cargo test --lib` 378 ✓ · `tsc`/`build` ✓.
- [x] AppImage/deb reconstruídos (19:47) e app aberto com o novo build.

## Notas

- Sem timestamps por palavra, o corte do texto é **proporcional ao tempo** com
  preferência por fim de frase/palavra. O alinhamento é visual/chronológico, não
  uma transcrição por palavra.
- O backend guarda a fala inteira; o corte é de **exibição** e não altera o
  áudio/ASR nem o export.
EOF