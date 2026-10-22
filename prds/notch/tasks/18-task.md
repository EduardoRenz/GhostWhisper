# 18 — Dwell do "Sim" contínuo ao mover dentro do botão

- **Status:** CONCLUDED
- **Depende de:** 10, 11; BR-036
- **Issues:** ISS-048 · **BR:** BR-036 · **Rules:** R16, R17
- **Refs:** [`../issues.md`](../issues.md),
  [`../architecture/drill-hover.md`](../architecture/drill-hover.md) §2.1/§8

## Objetivo

Impedir que o segundo estágio do drill-hover se perca quando o cursor se move
entre os elementos internos do botão **"Sim"** (ícone, texto e badge da
duração), mantendo a confirmação por permanência previsível.

## Diagnóstico

- `useDwellTimer.start()` sempre limpava o timeout e recomeçava a contagem.
- `DwellPopover` só iniciava no `pointerenter`; qualquer reentrância ou
  `leave` transitório durante a travessia do botão reiniciava/parava o estágio.
- Os filhos do botão podiam ser o alvo do ponteiro, dificultando a consistência
  do hover no WebKitGTK.
- Os testes existentes cobriam apenas uma entrada/saída limpa.

## Entregáveis

- `useDwellTimer`: `start()` idempotente enquanto o timer está ativo.
- `DwellPopover`: `pointermove` retoma o dwell se um `leave` transitório parar;
  `pointerleave`/`cancel` do botão têm debounce e checagem de `:hover`.
- `DwellPopoverWindow`: `pointerleave` da janela também tem debounce.
- A checagem pós-debounce usa o último ponto (`clientX/clientY`) com
  `document.elementFromPoint`, para distinguir filho/evento sintético de saída
  real.
- O início do segundo estágio usa **somente** a posição global do cursor; não
  usa `:hover`, que podia ficar obsoleto e iniciar o Sim com o cursor no
  Resumo/Sugestões.
- `DwellPopover` remontado por `token`, sem paradas pendentes de outra abertura.
- CSS: `pointer-events: none` nos filhos de `.dwell-target`; o botão é o alvo.
- Testes de regressão dos caminhos relatados (texto, badge, leave transitório e
  saída real).
- BR-036 revisada, issue/task/memory/arquitetura atualizados.

## Critérios de aceite

- [x] Mover o cursor de "Sim" para o ícone/texto/badge não reinicia o tempo.
- [x] Um `leave` transitório não zera a contagem; `pointermove` retoma e a
  parada só acontece quando o cursor realmente sai do botão/janela.
- [x] Manter o cursor no gatilho abre a confirmação mas **não** inicia o timer
  do "Sim"; é preciso mover para o botão.
- [x] Clique no "Sim" continua confirmando imediatamente.
- [x] O gatilho (Resumo/Sugestões) também não reinicia ao cruzar seus filhos.
- [x] `Escape` e o grace continuam cancelando corretamente.

## Verificação

- `pnpm test` **394 ✓** · `tsc --noEmit` ✓.
- Testes novos: timer idempotente; travessia texto→badge; retomada por
  `pointermove`; travessia no gatilho.
- AppImage (21:20) validado manualmente: manter o cursor no Resumo abre a
  confirmação, mas **não** inicia o timer do "Sim"; mover texto ↔ badge mantém o
  preenchimento e confirma no tempo configurado.
- Pacote final em `src-tauri/target/release/bundle` (21:20).
