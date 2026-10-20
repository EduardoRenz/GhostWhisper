# 04 — UI do diário: lista, streaming, estados e composer

- **Status:** CONCLUDED
- **Depende de:** 02, 03
- **Habilita:** 05, 08
- **Issues:** — · **BR:** BR-025, BR-040, BR-052
- **Refs:** [`../prd.md`](../prd.md) US-01, US-02;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §8

## Objetivo

Construir a **aba Sessão**: o composer da Busca em cima e o **diário**
abaixo, com streaming, estados de erro e o botão de retry.

## Entregáveis

- `src/core/domain/journal.ts`: tipos (`JournalEntry`, `JournalStatus`,
  `JournalRequestSource`) e o contrato do IPC.
- `src/core/application/useJournal.ts`:
  - carrega o diário da sessão (`journal_list`);
  - assina o evento `journal` e aplica o texto acumulado na entrada certa;
  - `ask(question, images, source)`;
  - `retry(entryId)`;
  - `cancel(entryId)`.
- `src/adapters/ipc/journal.ts`: `journalList`, `journalAsk`,
  `journalSearch`, `journalDelete`, `journalImage`.
- `src/features/journal/JournalView.tsx`: a aba — **composer** em cima,
  **diário** embaixo, **busca** no meio (a busca em si é da task 08; aqui o
  campo e o estado).
- `src/features/journal/AskComposer.tsx`: campo multilinha
  (`Enter` envia, `Shift+Enter` quebra linha), botão de print (task 06),
  anexos, botão **Perguntar**.
- `src/features/journal/JournalEntryCard.tsx`: pergunta, resposta em
  `MarkdownView` (BR-025), instante `mm:ss` desde o início da sessão, marca
  de origem, estado "pensando…", erro + **"Tentar de novo"**, print anexado
  como miniatura.
- `src/styles/globals.css`: estilos da aba (tokens do design system —
  BR-035).

## Critérios de aceite

- [x] As entradas aparecem **em ordem cronológica** (mais recente no fim).
- [x] A resposta chega **em streaming**, substituindo o texto a cada delta.
- [x] Entrada com erro mostra a mensagem e o botão de retry; o retry
      **reenvia a mesma pergunta** e **não** duplica a entrada.
- [x] Entrada **pending** mostra "pensando…" e **não** mostra resposta vazia
      como se fosse resposta.
- [x] Estado vazio: "Nenhuma pergunta ainda. Use **Perguntar** para perguntar
      à IA." — com o botão de atalho.
- [x] A entrada **nunca** aparece na aba Transcrição (BR-040).
- [x] `pnpm test` verde: `JournalView` (lista vazia, lista com entradas,
      streaming), `JournalEntryCard` (estados), `AskComposer` (Enter/Shift+Enter).

## Verificação

- `pnpm test` ✓ (`JournalView`: 9 testes — lista, vazio, envio, Enter, chips, print, busca)

## Notas

- O `mm:ss` do instante é **relativo ao início da sessão** (BR-019), igual à
  linha temporal — a UI deriva de `started_at_ms + at_ms` quando conhece o
  início (mesma regra de BR-040).
- O diário é **por sessão**: a aba resolve a sessão atual (a ativa ou a mais
  recente) e troca de conteúdo quando ela muda.
