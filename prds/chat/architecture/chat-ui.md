# Arquitetura — Chat (bolhas e transporte)

Especificação técnica da conversa em bolhas e dos controles de transporte.
Paridade com o **Perssua 0.27.5** (BR-015): valores e comportamento extraídos do
`app.asar` — evidência no §6. Desvios conscientes no §5.

---

## 1. Componentes

```
features/chat/ChatView.tsx        # scroller, cabeçalho ("Sugerir"), bolhas + em andamento
features/chat/ChatBubble.tsx      # uma bolha (rótulo, texto, tempo, copiar opcional)
components/TransportIcons.tsx     # SVG próprios: play, pause, stop, faísca
core/domain/chat.ts               # linha temporal (puro)
core/application/useLiveTranscription.ts  # eventos + histórico da sessão (já existe)
```

`ChatView` recebe `segments` (de `useLiveTranscription`) e `suggestions` (do
`useInsight` + evento do atalho) e não conhece IPC nem portas — só tipos de
domínio.

Estado do transporte continua em `features/session/SessionControls.tsx` (pílula de
ícones) dentro do `features/overlay/ActionBar.tsx` (bolinha de status, pílula de
sessão com tempo e confirmação).

## 2. Linha temporal (puro)

`core/domain/chat.ts` espelha `application/session_timeline.rs` no cliente:

```ts
type ChatBubble =
  | { kind: "speech"; id: string; speaker: Speaker; text: string; at_ms: number; end_ms: number }
  | { kind: "suggestion"; id: string; text: string; at_ms: number };

splitChatTimeline(segments) -> { committed, pending }   // pending: não finalizados
buildChatTimeline(segments, suggestions) -> { bubbles, pending }
```

Regras (testadas):

1. **Finalizado** (`final: true`) entra na linha temporal; **prévia** (`final:
   false`) vai para `pending` — é o que elimina o "preenchendo uma frase antiga"
   (ISS-018).
2. Dedupe por `id` (o último vence) — prévia e final do mesmo turno compartilham o
   id; na dúvida, o **final** vence (`mergeSegment` não deixa uma prévia
   sobrescrever um final).
3. Ordem: `at_ms` crescente; empate resolvido por `id` (falas antes das sugestões
   no mesmo instante).
4. `pending`: no máximo **uma por falante** (a mais recente), na ordem
   `ME`, `THEY` — como o Perssua (`gn(me)` antes de `gn(they)`).
5. Sugestões: dedupe por `message_id` (o atalho empurra a mesma sugestão que o
   banco devolve).

## 3. Contrato visual (paridade medida no Perssua)

| Elemento | Valor | Origem |
|---|---|---|
| Lista | `flex:1; overflow-y:auto; display:flex; flex-direction:column; gap:12px; padding:8px 4px` | `.unified-messages-container` |
| Linha | `display:flex; width:100%` | `.unified-transcript-message` |
| Você | `justify-content:flex-end` | `.unified-message-me` |
| Interlocutor | `justify-content:flex-start` | `.unified-message-they` |
| Bolha | `max-width:75%; padding:10px 14px; border-radius:12px; gap:4px` | `.unified-message-bubble` |
| Bolha "Você" | fundo `rgba(59,130,246,.2)`, borda `rgba(59,130,246,.3)`, canto inferior direito `4px` | `.unified-message-me .unified-message-bubble` |
| Bolha "Interlocutor" | fundo `rgba(255,255,255,.12)`, borda `rgba(255,255,255,.15)`, canto inferior esquerdo `4px` | `.unified-message-they .unified-message-bubble` |
| Rótulo | `11px; font-weight:600; rgba(255,255,255,.5); uppercase; letter-spacing:.5px` | `.unified-message-label` |
| Texto | `14px; rgba(255,255,255,.9); line-height:1.5; word-wrap:break-word` | `.unified-message-text` |
| Em andamento | bolha `opacity:.7`, `…` piscando 1.4s, texto `opacity:.8` | `.unified-message-current`, `.unified-typing-indicator` |
| Rolagem | auto-scroll suave **só** se o usuário está a ≲40 px do fim | `scrollIntoView` condicional |
| Blur por bolha | **não usar** (N camadas de blur re-renderizando por token) | comentário no CSS do Perssua |
| Pílula de controle | `gap:2px; padding:2px; border-radius:999px`; botão `28×28` redondo; ícone `16px`; `:disabled{opacity:.4}`; stop com hover vermelho | `.unified-session-controls`, `.unified-control-button` |
| Pílula de sessão | fundo `rgba(239,68,68,.15)`, borda `rgba(239,68,68,.4)`, texto `rgba(239,68,68,1)`, `radius:50vh`, tempo `tabular-nums` | `.actionbar-session-pill`, `.actionbar-session-timer` |
| Confirmação | texto + botões em pílula (`padding:3px 10px`) | `.actionbar-end-confirmation`, `.actionbar-end-btn` |
| Bolinha de status | 8px, vermelha pulsando (1.6s) / âmbar pausado | `.transcription-preview-dot.live/.paused` |

Cores próprias do GhostWhisper (tema escuro) substituem variáveis de flavor do
Perssua; a **geometria e a hierarquia** são as da tabela.

## 4. Fluxo de dados

```
                                 ┌─ transcript_segment (prévia/final) ─┐
SessionService → pipelines ──────┤                                     ├→ useLiveTranscription
                                 └─ session_transcriptions (sessão) ───┘        │
useInsight(sessionId) ──── db_session_messages ──┐                              │
useInsightFeed() ──── insight (atalho) ──────────┴→ dedupe por message_id       │
                                                                                ▼
                                            buildChatTimeline(segments, suggestions)
                                                       │                │
                                                  bubbles            pending
                                                       ▼                ▼
                                                  ChatView (linha temporal + bolhas em andamento)
```

- Mesma conversa para o **ao vivo** e o **histórico**: o detalhe do histórico usa
  `db_session_timeline` (já ordenado no backend) e o mesmo `ChatView`.
- `SessionStateEvent` passa a carregar `started_at_ms` (opcional) para a pílula de
  sessão calcular `mm:ss` no cliente (`Date.now() - started_at_ms`).

## 5. Desvios conscientes do Perssua (R12)

| Desvio | Por quê |
|---|---|
| Play e pause como **dois botões** na pílula | pedido do usuário; mais descobrível que o toggle do Perssua |
| ⏹ **também** pede confirmação (e não só a pílula de sessão) | não existir caminho que encerra sessão sem confirmação |
| "Iniciar escuta" cobre "Start Listening" + "Restart" | não temos a janela pós-sessão do Perssua; um controle só evita dois botões com a mesma ação |
| `mm:ss` em cada bolha (o Perssua não mostra tempo no ao vivo) | BR-019: toda mensagem tem seu tempo na linha temporal |
| Ícones SVG próprios (triângulo, barras, quadrado) | BR-008: não copiar assets de terceiros (a fonte Material Icons é de terceiros) |
| `cross-spawn` de busca/label: rótulos em PT-BR ("Você", "Interlocutor", "Sugestão da IA") | i18n do produto (BR-010); o Perssua usa "You"/"Others"/"Assistant" |

## 6. Evidência no bundle do Perssua

Extração: `/tmp/opencode/perssua/asar` (asar de `Perssua-0.27.5-x86_64.AppImage`).

| Assunto | Arquivo | Âncora |
|---|---|---|
| Bolhas da transcrição + bolhas "em andamento" | `build/static/js/4598.1a6df089.js` | `unified-transcript-message`, `unified-message-current` |
| CSS das bolhas / rótulo / texto / indicador de digitação | `build/static/css/4598.dc3b562d.css` | `.unified-message-*` |
| Pílula play/pause/stop | `build/static/js/5684.18f69cc0.chunk.js` | offset ≈ 46793 (`unified-session-controls`) |
| Estilo da pílula e do botão stop | `build/static/css/4598.dc3b562d.css` | `.unified-control-button`, `.unified-stop-button` |
| Pílula de sessão (tempo) + confirmação de encerramento | `build/static/js/main.f180d025.js` | offset ≈ 164200–168100 |
| CSS da pílula de sessão, confirmação e bolinha de status | `build/static/css/3558.e4862c6d.css` | `.actionbar-session-*`, `.actionbar-end-*` |
| Bolhas do histórico | `build/static/css/2748.65782e3f.chunk.css` | `.transcript-message`, `.message-bubble` |

Reproduzir: `python3 /tmp/opencode/asar.py cat <caminho>` ou ler os arquivos já
extraídos nesse diretório.
