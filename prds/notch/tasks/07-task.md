# 07 — Padronizar a UI com o design system "Obsidian HUD"

- **Status:** CONCLUDED
- **Depende de:** 02, 03, 05 (notch, painel em abas e nomes já existem)
- **Habilita:** 08 (notch ativo)
- **Issues:** — · **BR:** BR-035 (revisa o verde de BR-032)

## Objetivo

Aplicar o design system **Obsidian HUD** — tokens em
[`../architecture/design-tokens.md`](../architecture/design-tokens.md) — em toda a
interface: notch, painel (Transcrição/Resumo), Configurações, Histórico e
Sugestões. O resultado deve ser **um só vocabulário visual** (vidro, cyan de foco,
violeta de IA, esmeralda de gravação, coral de erro), mantendo leitura e desempenho.

## Escopo

**Dentro:**
- Tokens CSS `--oh-*` (cor, tipografia, raio, espaço, vidro, sombra/glow) e
  **fontes Geist + JetBrains Mono empacotadas localmente** (sem CDN).
- Re-skin dos componentes listados em
  [`design-tokens.md`](../architecture/design-tokens.md) §6.
- Fallbacks de acessibilidade/desempenho (§7): transparência, contraste,
  `prefers-reduced-motion`, `forced-colors`.
- Substituição dos hex/rgba soltos por tokens (exceto nos gradientes declarados em
  `tokens.css`).

**Fora:**
- Novas telas, mudança de layout/fluxo ou de comportamento.
- Tema claro (o sistema é dark-only).
- Ícones novos (o pacote da task 01 permanece).
- Aba "Sessão" (continua reservada).
- Reduzir transparência configurável pelo usuário (fica para uma PRD futura).

## Entregáveis

### Tokens e fontes
- `src/styles/tokens.css` (novo): `:root` com todos os `--oh-*` do documento de
  tokens, `@font-face` (Geist/JetBrains Mono) e `@supports`/media queries.
- `src/assets/fonts/` (woff2 + licenças OFL) — `font-display: swap`, fallbacks do
  sistema.
- `src/styles/globals.css` importa `tokens.css` e passa a **consumir** os tokens
  (sem novos hex/rgba fora dos tokens).

### Componentes
- `features/overlay/ActionBar.tsx` + `IconButton`/`ShortcutBadges`:
  dock de vidro, borda hairline em gradiente, ação primária **esmeralda** (BR-032),
  ícones 28–32px, foco cyan.
- `features/panel/{PanelTabs,PanelWindow,SummaryView,LanguageBar}.tsx`:
  segmented switcher com destaque deslizante, painel de leitura quase opaco,
  pill de resumo violeta, input/select inset.
- `features/chat/{ChatView,ChatBubble}.tsx`: rótulos/tempos em mono, superfícies
  de leitura; **sem `backdrop-filter` por bolha**.
- `features/settings/*`, `features/history/*`, `features/suggestions/*`:
  grupos/cartões, botões, inputs e tabelas nos tokens.
- `components/MarkdownView.tsx` + `styles/globals.css`: código em JetBrains Mono,
  tabelas/`blockquote`/links nos tokens.
- `features/overlay/LevelMeters.tsx`: trilha e preenchimento cyan→violeta.

## Critérios de aceite

- [ ] Nenhum hex/rgba novo fora de `tokens.css`; componentes usam `var(--oh-*)`.
- [ ] Geist e JetBrains Mono carregam **offline** (sem requisições de rede) com
      fallback; licenças presentes.
- [ ] Notch, painel, abas, botões, badges, status, medidores, bolhas, Markdown,
      inputs e Configurações/Histórico/Sugestões seguem o §6 do doc de tokens.
- [ ] **Blur só nas cascas**; nenhuma bolha/item com `backdrop-filter` (ISS-020).
- [ ] Fallback opaco sem `backdrop-filter`; `prefers-reduced-motion` sem pulsos e
      com spinner estático; `forced-colors` legível.
- [ ] Contraste: texto padrão ≥ 4,5:1 e microcopy ≥ 3:1 sobre o painel.
- [ ] Ação primária em **esmeralda**; cor de "pausado" sem âmbar (usa outline).
- [ ] Sem regressão de layout nos tamanhos mínimos (janela 360×320; notch estreito).
- [ ] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes; `cargo test/clippy/fmt`
      sem regressão (o core não deve mudar).

## Como testar

1. **Notch em repouso:** pill de vidro, marca, engrenagem, ação esmeralda com
   badges em mono e Histórico/ocultar; borda hairline e sombra difusa.
2. **Notch em sessão:** status com ponto esmeralda pulsando, transporte e ícones;
   foco por `Tab` em cyan; pausado em outline (sem âmbar).
3. **Painel:** abas com destaque deslizante; Transcrição e Resumo legíveis sobre o
   painel quase opaco; pill de resumo em violeta com spinner; código em mono.
4. **Configurações/Histórico/Sugestões:** cartões, inputs inset com foco cyan,
   botões ghost/primário.
5. **Sem compositor/blur:** desligar o compositor (ou usar `@supports`) e conferir
   o fallback opaco.
6. **Acessibilidade:** navegação por teclado, contraste, `prefers-reduced-motion`
   e `forced-colors`.
7. **Desempenho:** sessão com áudio contínuo — a conversa não deve re-renderizar a
   casca por evento de nível (ISS-020).

Ponto de parada: revisão visual do usuário nos 4 cenários principais (notch,
painel, settings, sugestões).

## Riscos / rollback

| Risco | Mitigação |
|---|---|
| Custo de `backdrop-filter` em GPU fraca | blur só em cascas; fallback opaco; medir com áudio ativo |
| Vidro demais prejudica leitura | superfícies de leitura quase opacas (`--oh-read-surface`) |
| Fontes aumentam o pacote | woff2 variável + `font-display: swap`; só os pesos usados |
| Contraste insuficiente sobre desktop claro | fundo de leitura opaco; checar contraste do §7 |
| Regressão visual ampla | aplicar por camada (tokens → cascas → componentes) e revisar por tela |

**Rollback:** `tokens.css` mantém os tokens antigos como aliases de compatibilidade
até a revisão; se necessário, reverter o import e manter o CSS anterior.

## Verificação

- Automática: `pnpm test` · `tsc --noEmit` · `pnpm build` · `cargo test`/`clippy`/`fmt`.
- Manual: os 7 passos acima + revisão visual do usuário.
- Observações do usuário: (aguardando)

## Resultado (2026-10-19)

- `src/styles/tokens.css`: tokens `--oh-*` (cores, tipografia, raio, espaço,
  vidro, sombra/glow), `@font-face` e utilitários `.oh-*`; importado antes de
  `globals.css`.
- Fontes **locais**: Geist (latin/latin-ext) e JetBrains Mono (latin/latin-ext) em
  `src/assets/fonts/`, com as licenças OFL.
- `globals.css` re-skin: notch de vidro + borda hairline, ação primária
  **esmeralda**, sinais cyan/violeta/esmeralda/coral, painel em superfície de
  leitura, abas/botões/inputs em pill, bolhas e Markdown em tokens, scrollbars
  finas e fallback opaco sem `backdrop-filter`.
- Verificação: `pnpm test` **155 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓ (fontes
  copiadas para `dist/assets`).
- Pendente: revisão visual do usuário.
