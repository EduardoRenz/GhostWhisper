# 08 — Notch ativo no padrão Obsidian HUD

- **Status:** CONCLUDED
- **Depende de:** 07 (tokens/fontes e re-skin base)
- **Habilita:** —
- **Issues:** — · **BR:** BR-032 (revisão 2026-10-19), BR-035

## Objetivo

Levar o **notch no estado ativo** (sessão gravando/pausada) para o layout do
protótipo Obsidian HUD: **switcher segmentado** com a aba ativa destacada,
**waveform animado** ao lado de "Transcrição", **play/pause contextual**, timer
mono e ações compactas. O estado **repouso** mantém a estrutura de hoje (só ganha
o re-skin da task 07).

## Análise do protótipo (o que adotar)

| Elemento do protótipo | Decisão no GhostWhisper |
|---|---|
| Grip + marca + dot | Adotar (grip existente; dot cyan de presença; nome em `headline-md`) |
| Switcher **Transcrição / Histórico / Resumo** | Adotar; **Histórico** = janela; **Resumo** = aba do painel; **Transcrição** = aba viva |
| Waveform animado ao lado de "Transcrição" | Adotar, **dirigido pelo nível de áudio** (`audio_level`), sem re-render da barra |
| Botão circular de **play/pause** após a aba ativa | Adotar: grava → `pause`; pausado → `play` |
| Timer `00:14:32` mono | Adotar (`HH:MM:SS`, tabular, dot esmeralda) |
| `mic_off` (trailing) | Mapear para **Canais** (popover Ambos/Mic/Sistema + "Nova sessão"); ícone `mic-off` quando só sistema |
| `close_fullscreen` | Mapear para **Ocultar** |
| Tailwind/Material/Google Fonts via CDN | **Não** adotar: CSS tokens + SVG próprios + fontes locais (BR-008/BR-035) |
| "Sessão" no comentário do HTML | **Não** existe; o item é **Histórico** (como na imagem) |
| OCR / contexto de tela / captura | **Fora de escopo** (áudio-only) |
| Sugestões (ausente no protótipo) | **Preservar** como ícone trailing (janela própria) |
| Stealth/engrenagem (ausentes no protótipo) | **Preservar** como ícones trailing; a **engrenagem é o último botão redondo da direita** |

## Estrutura do notch

```
Repouso (como hoje, só re-skin):
  [grip] ● GhostWhisper   [▶ Começar a ouvir  Ctrl Shift S]   [Histórico] [⌄] [⚙]

Ativo:
  [grip] ● GhostWhisper │ [🎙 Transcrição ▁▃▅▂] [⏸/▶] [🕘 Histórico] [📄 Resumo] │
  [● 00:14:32]  [🎙 canais] [✦ Sugestões] [👁 stealth] [⌄ ocultar] [⚙ configurações]
```

- **Switcher** (segmented, `role="tablist"`): três itens + o toggle de transporte
  circular ao lado do item ativo.
- **Timer** clicável: abre a confirmação de encerramento (BR-022 preservada).
- **Canais**: popover compacto (Ambos / Microfone / Sistema) + **Nova sessão**;
  trocar canal ao vivo continua permitido (BR-019).
- **Trailing**: Sugestões, stealth, ocultar e — **por último** — **Configurações**
  (engrenagem). A engrenagem é o **último botão redondo da direita** (correção do
  protótipo, que a omitiu).

## Comportamento

### 8.1 Aba ativa

- **Transcrição** ativa quando a sessão está gravando (ou pausada) **ou** o painel
  está aberto na aba Transcrição. Clicar abre/foca o painel na aba **Transcrição**.
- **Resumo** ativa quando o painel está aberto na aba **Resumo**. Clicar abre/foca
  o painel na aba **Resumo**.
- **Histórico** ativa quando a janela de histórico está visível. Clicar
  mostra/esconde.
- A aba ativa usa `surface-container-highest` + texto **cyan** + borda hairline
  (tokens §6); as inativas ficam em `--oh-text-muted` com hover suave.

### 8.2 Waveform (gráfico de áudio detectado)

- Componente `NotchWaveform` (4 barras de 2px, raio full, gaps 2px, cyan).
- **Dirigido pelo áudio**: assina `audio_level` internamente e escreve **custom
  properties** (`--oh-wave-*`) em `refs` — **sem `setState`** e sem re-render da
  `ActionBar` (protege a ISS-020). Altura = `4px + nível × 10px` (máx. 14px),
  suavizada; em silêncio fica baixa.
- **Pausado:** barras estáticas em 4px e opacidade ~0.45 (sem animação).
- **Sem áudio/eventos:** permanece baixa (não inventa movimento).
- Fallback documentado: se o custo do evento for perceptível no teste manual,
  trocar por animação CSS (`pulseWave`) enquanto gravando.

### 8.3 Play/Pause contextual

- **Gravando** → ícone `pause`, `aria-label="Pausar"`; clique = `pause`.
- **Pausado** → ícone `play`, `aria-label="Retomar"`; clique = `resume`.
- `busy` desabilita; o **encerrar** continua **só** na pílula/timer (BR-022).

### 8.4 Timer

- `HH:MM:SS` em JetBrains Mono (`code-sm`), `font-variant-numeric: tabular-nums`.
- Dot **esmeralda** pulsando gravando; **outline** pausado (sem âmbar — BR-035).
- Clique abre a confirmação inline ("Encerrar sessão? Sim / Não").

### 8.5 Abrir o painel em uma aba específica

- Estender `panel_open` com `tab: Option<String>` e emitir o evento `panel_tab`
  (o mesmo que o tray já usa). O adaptador ganha `panelOpenTab(label, tab)`.
- `PanelWindow` já escuta `panel_tab`; nenhuma mudança de contrato nova.

## Componentes e arquivos

- `features/overlay/ActionBar.tsx` — composição dos dois estados.
- `features/overlay/NotchTabs.tsx` — switcher (reutiliza `IconButton`/ícones).
- `features/overlay/NotchWaveform.tsx` — barras + assinatura de `audio_level`.
- `features/overlay/ChannelMenu.tsx` — popover de canais + Nova sessão.
- `features/session/SessionPill.tsx` — re-skin para o timer (`HH:MM:SS`).
- `components/TransportIcons.tsx` — play/pause (já existem); `MicIcon`/`MicOffIcon`
  em `components/icons.tsx`.
- `core/domain/transcript.ts` — `formatClockLong(ms)` (puro, testado).
- `core/domain/audio.ts` (ou `transcript.ts`) — `waveformLevel(level)` (puro).
- `src-tauri/src/ipc/panel_commands.rs` + `src/adapters/ipc/panels.ts` — `tab`
  opcional em `panel_open`.
- `src/styles/globals.css` — classes do switcher/waveform/timer (tokens `--oh-*`).

## Critérios de aceite

- [ ] Notch ativo na ordem do protótipo: grip · marca · switcher · timer ·
      ações trailing (CA-10.1).
- [ ] "Transcrição" destacada quando gravando/na aba; waveform ao lado (CA-10.2).
- [ ] Waveform **reage ao áudio** e **não re-renderiza** a barra (refs/CSS vars);
      pausado → estático (CA-10.3).
- [ ] Play/pause **muda conforme o contexto** e chama pause/resume (CA-10.4).
- [ ] Timer `HH:MM:SS` mono; clique abre a confirmação de encerramento (CA-10.5).
- [ ] Clicar Transcrição/Resumo abre o painel na aba certa; Histórico alterna a
      janela (CA-10.6).
- [ ] Canais + "Nova sessão" preservados (popover), troca ao vivo (BR-019).
- [ ] Sugestões, stealth, ocultar e engrenagem preservados; a **engrenagem é o
      último botão redondo da direita** e abre as Configurações.
- [ ] O estado **repouso** mantém a estrutura atual (CA-01.1) — **sem**
      switcher/waveform/timer — com a **engrenagem como último** botão; só o re-skin.
- [ ] Responsivo: em largura estreita, rótulos do switcher somem e ficam só os
      ícones (com `aria-label`).
- [ ] Acessibilidade: `role=tablist`/`aria-selected` (ou `aria-current`), foco
      visível, teclado; sem âmbar.
- [ ] `pnpm test`, `tsc --noEmit`, `pnpm build`; `cargo test/clippy/fmt`.

## Como testar

1. **Repouso:** igual a hoje (marca, engrenagem, "Começar a ouvir" esmeralda com
   atalho, Histórico, ocultar) — só com o visual novo.
2. **Iniciar:** o notch muda para o estado ativo; "Transcrição" fica destacada; o
   waveform começa a reagir à fala; o timer sobe.
3. **Pausar:** o botão vira **play**, o waveform congela/esmaece; **Retomar**
   volta ao estado gravando.
4. **Clicar Resumo:** o painel abre/foca na aba Resumo e o item Resumo fica ativo;
   **Transcrição** volta para a aba correspondente.
5. **Histórico:** abre/fecha a janela e reflete o estado no item.
6. **Canais:** trocar para "Sistema" mostra `mic-off`; "Nova sessão" continua
   funcionando.
7. **Encerrar:** clicar no timer mostra "Encerrar sessão? Sim / Não".
8. **Engrenagem:** é o **último** botão redondo da direita e abre as Configurações.
9. **Desempenho:** durante fala contínua, a barra não re-renderiza por evento de
   nível; a conversa segue fluida (ISS-020).

Ponto de parada: revisão visual do usuário nos estados repouso/gravando/pausado.

## Riscos / rollback

| Risco | Mitigação |
|---|---|
| Excesso de itens na largura de 900px | padding compacto; labels somem em largura estreita; medir no mínimo |
| Waveform reintroduzir custo por evento | refs + CSS vars, sem estado; fallback CSS-only |
| Dois "sistemas de abas" (notch × painel) divergirem | notch é **navegação**; `panel_tab` continua a fonte da aba do painel |
| Popover de canais quebrar click-through | `data-interactive` só no popover/botões |
| `panel_open` com `tab` quebrar chamadas antigas | parâmetro opcional (`None` = comportamento atual) |

## Verificação

- Automática: `pnpm test` · `tsc --noEmit` · `pnpm build` · `cargo test`/`clippy`/`fmt`.
- Manual: os 8 passos acima.
- Observações do usuário: (aguardando)

## Resultado (2026-10-19)

- `NotchTabs` (Transcrição + waveform · play/pause · Histórico · Resumo),
  `NotchWaveform` (4 barras dirigidas por `audio_level` em refs/CSS vars, **sem
  re-render**), `ChannelMenu` (popover Ambos/Mic/Sistema + Nova sessão) e
  `ActionBar` recomposta; timer `HH:MM:SS` com dot esmeralda no `SessionPill`.
- Puros: `formatClockLong`, `waveformLevel`/`waveformHeight`; ícones
  `MicIcon`/`MicOffIcon`.
- IPC: `panel_open` aceita `tab` opcional e emite `panel_tab`; `panelOpenTab` no
  frontend; tray usa `open_panel_tab`.
- Transcrição destacada enquanto capta; pausado → play + waveform estático; a
  **engrenagem é o último botão**; repouso sem switcher/waveform/timer.
- `SessionControls` removido (substituído por `NotchTabs`/`ChannelMenu`).
- Verificação: `pnpm test` **155 ✓** · `tsc` ✓ · `pnpm build` ✓ · `cargo test`
  **343 ✓** · clippy/fmt ✓.
- Pendente: revisão visual com áudio real (waveform/estados).
