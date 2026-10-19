# 11 — Bolhas em largura total: Microfone/Outros, hora e Streaming

- **Status:** CONCLUDED
- **Depende de:** 09, 10
- **Habilita:** 12
- **Issues:** — · **BR:** BR-040, BR-031
- **Refs:** [`../prd.md`](../prd.md) §9, [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3

## Objetivo

Trocar a leitura de "bolha alinhada à esquerda/direita, com 75% de largura" pela
leitura do print: **todo cartão ocupa a largura da janela**, e a identidade de
quem falou vem do **rótulo colorido + nome + hora**:

- **Microfone** — cor primária/cyan;
- **Outros** — violeta (a outra cor do print);
- **hora da mensagem** ao lado do nome;
- fala em andamento com **borda cyan**, "Falando agora…" e selo **Streaming**.

Continua valendo o teto de 180 caracteres com continuação (BR-031), agora em
cartões de largura total.

## Entregáveis

- `src/core/domain/transcript.ts`:
  - `speakerLabel`: `ME → "Microfone"`, `THEY → "Outros"`;
  - `formatMessageClock(atMs, startedAtMs?)`: hora de parede `HH:MM:SS` a partir
    de `started_at_ms + at_ms`; sem `started_at_ms`, cai em `formatClock(atMs)`
    (`mm:ss`). O dado persistido **continua relativo à sessão** (BR-019).
- `src/core/domain/chat.ts`:
  - `SpeechBubble` ganha `source_text` (o texto **inteiro** do segmento, antes do
    split de exibição), para a ação de IA enviar a mensagem completa mesmo quando
    o print mostra mais de um cartão.
- `src/features/chat/ChatView.tsx`:
  - novas props `sessionStartedAtMs` e `onAskAi?` (o botão será ligado na 12);
  - calcula o rótulo de hora e passa para a bolha;
  - a bolha em andamento usa o estado de streaming e não mostra ação de IA.
- `src/features/chat/ChatBubble.tsx`:
  - remove alinhamento por lado e `max-width`; o cartão é `width: 100%`;
  - linha superior: rótulo colorido + hora; grupo de ações no canto superior
    direito (tradução existente + espaço para a IA);
  - fala em andamento: "Falando agora…" no rótulo, selo **Streaming** com ponto
    cyan à direita, caret na última parte;
  - continuação (`continues`): sem rótulo/hora/ações repetidos, colada na de
    cima;
  - a bolha de sugestão da IA mantém Markdown/copiar, agora também full-width.
- `src/features/history/HistoryPanel.tsx`: passa `current.started_at_ms` ao
  `ChatView`; o histórico usa as mesmas bolhas, **sem** ação de IA (não há sessão
  ao vivo).
- `src/styles/tokens.css` e `src/styles/globals.css`:
  - tokens de identidade `--oh-speaker-me` (accent/cyan) e
    `--oh-speaker-they` (violeta, reusando `--oh-agent-violet` como valor);
  - remoção de `.chat__row--me/--they` com `justify-content` e do
    `max-width: 75%`;
  - cartão neutro, borda hairline, raio 12; rótulo 12/600 **sem uppercase**
    (o print usa "Microfone"/"Outros" em title case); hora mono 10;
  - estado pendente com fundo/borda cyan; selo Streaming; caret.
- Testes: `transcript.test.ts` (rótulos/hora/fallback), `ChatView.test.tsx`
  (largura, rótulos, hora, streaming), `ChatBubble` (ações/continuação) e
  `HistoryPanel.test.tsx` (mesmo visual + hora).

## Critérios de aceite

- [ ] Nenhuma bolha tem `max-width: 75%` nem alinhamento à direita/esquerda: todas
      preenchem a largura da lista, independente do tamanho do texto.
- [ ] **Microfone** aparece em cyan e **Outros** em violeta, com a hora ao lado
      (`HH:MM:SS`; fallback `mm:ss`).
- [ ] A fala em andamento mostra "Falando agora…" + **Streaming**, borda cyan e
      caret; não mostra a ação de IA.
- [ ] Continuação de fala longa não repete rótulo/hora e permanece colada.
- [ ] O histórico renderiza o mesmo contrato visual e a mesma hora.
- [ ] O botão de tradução continua no canto superior direito, ao lado do espaço
      da IA, sem sobrepor o texto (BR-030).
- [ ] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Verificação

- Sessão com microfone e sistema: conferir as duas cores, a hora e a ordem.
- Falar sem pausas: conferir a continuação e o selo Streaming na última parte.
- Abrir uma sessão antiga no Histórico e conferir a hora de parede.
- Redimensionar a janela e conferir que o cartão acompanha a largura.

## Notas

- O print mostra hora de parede (10:42:15). Mantemos o **armazenamento** em tempo
  relativo (BR-019) e derivamos a hora na UI; sem migração.
- O teto de 180 caracteres (BR-031) é de **exibição**; o `source_text` garante que
  a sugestão manual receba a fala inteira.

## Resultado (2026-10-19)

- `speakerLabel` passou a devolver **Microfone/Outros** e `formatMessageClock`
  mostra a hora de parede `HH:MM:SS` (fallback `mm:ss`).
- `ChatBubble` virou cartão de **largura total**, com rótulo colorido + hora no
  topo e ações no canto direito; a prévia mostra “Falando agora…” e
  **Streaming**; a continuação não repete rótulo/hora.
- `SpeechBubble.source_text` carrega o texto inteiro antes do split; o botão de
  IA recebe a mensagem completa.
- `HistoryPanel` usa a mesma hora de parede e o mesmo componente.
- `pnpm test` 261 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓.
