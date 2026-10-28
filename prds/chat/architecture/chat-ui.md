# Arquitetura — Chat (bolhas e transporte)

Especificação técnica da conversa em bolhas e dos controles de transporte.
Paridade com o **Perssua 0.27.5** (BR-015): valores e comportamento extraídos do
`app.asar` — evidência no §6. Desvios conscientes no §5.

---

## 1. Componentes

```
features/chat/ChatWindow.tsx         # janela própria da conversa (task 07)
features/chat/ChatHeaderActions.tsx  # cabeçalho: "Sugerir" + atalhos Resumo/Sugestões
features/chat/ChatView.tsx        # scroller, bolhas + em andamento
features/chat/ChatBubble.tsx      # uma bolha (rótulo, texto, tempo, copiar opcional)
features/history/HistoryWindow.tsx # janela própria do histórico (task 08)
features/history/HistoryPanel.tsx  # lista/detalhe/export (reusado pela janela)
features/panels/PanelFrame.tsx     # moldura comum (cabeçalho, corpo, alças de resize)
components/TransportIcons.tsx     # SVG próprios: play, pause, stop, faísca
core/domain/chat.ts               # linha temporal (puro)
core/application/useLiveTranscription.ts  # eventos + histórico da sessão (já existe)
```

`ChatView` recebe `segments` (de `useLiveTranscription`) e `suggestions` (do
`useInsight` + evento do atalho) e não conhece IPC nem portas — só tipos de
domínio. Desde a task 07 ele vive na janela **`chat`**, não mais no painel do
overlay.

Estado do transporte em `features/session/SessionControls.tsx`: **um só** controle
que alterna start/pause/retomar, ao lado do seletor de canais e de "Nova sessão".
O encerrar é **só** a pílula de sessão (`SessionPill`, com `mm:ss` e confirmação),
dentro do `features/overlay/ActionBar.tsx` — que virou uma barra sem painel, com os
toggles **Chat** e **Histórico** (tasks 07–09).

## 2. Linha temporal (puro)

`core/domain/chat.ts` espelha `application/session_timeline.rs` no cliente:

```ts
type ChatBubble =
  | { kind: "speech"; id: string; speaker: Speaker; text: string; at_ms: number; end_ms?: number }
  | { kind: "suggestion"; id: string; text: string; at_ms: number };

splitChatTimeline(segments) -> { committed, pending }   // pending: não finalizados
buildChatTimeline(segments, suggestions) -> { bubbles, pending }
splitSpeechSpans(spans) -> SpeechSpan[]                // só sobreposição (BR-043)
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
6. **Uma fala é uma bolha** (BR-102, que revogou a BR-031/ISS-031): não há teto
   de caracteres nem divisão por tamanho. A fala finalizada entra inteira e a
   prévia é **uma** bolha por falante, com Streaming. O único corte é o de
   **sobreposição** dos canais (BR-043/task 16), e ele é **conservador**
   (ISS-075/task 20): fala ≥ 80 caracteres e ≥ 6 s, interrupção ≥ 1,5 s e todas
   as partes ≥ 20 caracteres/3 palavras.

## 3. Contrato visual (paridade medida no Perssua)

> **Revisão 0.2 (2026-10-19, BR-040):** o painel **ao vivo** passou a seguir o
> print de referência do usuário — cartões em **largura total**, rótulos
> **Microfone/Outros**, hora ao lado do nome e ação de IA por mensagem. O
> contrato vigente está em
> [`transcription-panel.md`](transcription-panel.md) §2–§3; a tabela abaixo é a
> **evidência histórica** da paridade Perssua 0.27.5 (v0.1) e não deve ser usada
> como especificação do ao vivo. O histórico continua usando o mesmo componente
> de bolha, com os valores novos.

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
| Fala em andamento | bolha `opacity:.7`, `…` piscando 1.4s, texto `opacity:.8` | `.unified-message-current`, `.unified-typing-indicator` |
| Rolagem | auto-scroll suave **só** se o usuário está a ≲40 px do fim | `scrollIntoView` condicional |
| Blur por bolha | **não usar** (N camadas de blur re-renderizando por token) | comentário no CSS do Perssua |
| Pílula de controle | `gap:2px; padding:2px; border-radius:999px`; botão `28×28` redondo; ícone `16px`; `:disabled{opacity:.4}`; **um só botão** que alterna `play_arrow ↔ pause` (o encerrar vive na pílula de sessão) | `.unified-session-controls`, `.unified-control-button` |
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
| "Iniciar escuta" cobre "Start Listening" + "Restart" | não temos a janela pós-sessão do Perssua; um controle só evita dois botões com a mesma ação |
| `mm:ss` em cada bolha (o Perssua não mostra tempo no ao vivo) | BR-019: toda mensagem tem seu tempo na linha temporal |
| Ícones SVG próprios (triângulo, barras, quadrado) | BR-008: não copiar assets de terceiros (a fonte Material Icons é de terceiros) |
| `cross-spawn` de busca/label: rótulos em PT-BR ("Você", "Interlocutor", "Sugestão da IA") | i18n do produto (BR-010); o Perssua usa "You"/"Others"/"Assistant" |
| **Uma fala = uma bolha** (BR-102) | o Perssua também não limita a bolha (`.unified-message-bubble` sem `max-height`): lá o *commit* em nuvem é de ~2 s e a bolha não cresce. No whisper **local** o commit é por silêncio (1 s) e o teto do turno é 25 s. O teto de 180 (BR-031) foi tentado e **revogado** (ISS-073): o corte era só de exibição, não um commit, então a fala longa virava várias bolhas "abertas" parecendo processamento múltiplo. Cortar o **turno** por tempo degradaria a transcrição (ISS-058); a preferência passou a ser a bolha final inteira. |

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
| **Sem teto de bolha no Perssua** (evidência da BR-102) | `build/static/css/4598.dc3b562d.css` | `.unified-message-bubble` (só `max-width:75%` — nenhum `max-height`/`line-clamp`; o `line-clamp` do bundle é de onboarding/legendas) |

> As **sugestões da IA** nas bolhas agora renderizam **Markdown** com código
> colorido (BR-025), pelo `MarkdownView`; os contratos de resumo/agente e as
> janelas separadas estão em
> [`../../assistant/architecture/assistant-windows.md`](../../assistant/architecture/assistant-windows.md).

Reproduzir: `python3 /tmp/opencode/asar.py cat <caminho>` ou ler os arquivos já
extraídos nesse diretório.

## 7. Janelas próprias (chat e histórico) — tasks 07/08

A conversa e o histórico deixaram o painel do overlay e viraram janelas da mesma
família de `summary`/`suggestions` (BR-026):

- **`chat`** — `ChatWindow`: `PanelFrame` + medidores de nível + `ChatView`; o
  cabeçalho (`ChatHeaderActions`) traz "Sugerir", "Resumo" e "Sugestões".
- **`history`** — `HistoryWindow`: `PanelFrame` + `HistoryPanel` (lista, detalhe em
  bolhas, renomear/exportar/excluir). "Continuar sessão" volta a capturar e traz a
  janela de `chat` para frente.
- Ambas: `resizable: true` (mín. 360×320 no chat, 320×240 no histórico), sem
  moldura, sempre no topo, `skipTaskbar`; arraste pelo cabeçalho e resize por
  `ResizeHandles` → `startResizeDragging` (X11/XWayland; Wayland nativo pode não
  pegar, BR-007).
- Posição/tamanho persistem em `Settings.*_position`/`*_size` (px físicos); o boot
  reaplica e o tracker do composition root salva enquanto a janela está visível.
  `domain::settings::PANEL_LABELS` é a lista única usada por boot, tracker, o
  fechar-da-janela e `panel_open`/`panel_hide`.
- O overlay virou **só a ActionBar** (`Surface` = `actionbar | hidden`): saíram o
  painel expansível, as abas Conversa/Histórico e a superfície `expanded-tall`.

### 7.1 Stop único e toggle único (task 09)

- `SessionControls` tem **um** botão: parado → `▶ Iniciar escuta`; gravando → `⏸`;
  pausado → `▶`. Um clique = start/pause/retomar.
- Encerrar existe **só** na pílula de sessão (`SessionPill`), com "Encerrar sessão?
  Sim / Não" — `onRequestStop` saiu do transporte.
- Evidência (R12): `unified-session-controls` no bundle tem exatamente
  `[play_arrow ↔ pause]` (o ícone alterna com o estado) + `[stop]`, e o "End
  session?" vive na pílula do actionbar (`.actionbar-session-pill`).

> **Evolução (2026-10-18, PRD [`notch`](../../notch/prd.md) — BR-033):** a janela
> `chat` virou o **painel da sessão** com abas **Transcrição | Resumo** (a conversa
> é a aba Transcrição) e a janela `summary` foi **aposentada**. O transporte e o
> comportamento das bolhas descritos aqui **não mudam**; a nota evita duplicação
> (BR-016).
