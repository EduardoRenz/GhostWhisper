# 16 — Escala de fonte global + bônus do texto de IA

- **Status:** CONCLUDED
- **Depende de:** 15; BR-062/BR-064
- **Issues:** — · **BR:** BR-087
- **Refs:** [`../prd.md`](../prd.md) §12,
  [`../../notch/architecture/design-tokens.md`](../../notch/architecture/design-tokens.md) §3–4,
  [`../../business-rules.md`](../../business-rules.md) BR-087

## Objetivo

A aba **"Geral & HUD"** ganha a seção **"Aparência & Fonte"** com dois
controles: a **escala relativa da fonte de toda a UI** (75–160 %) e um
**bônus só para o texto de IA** (100–150 %) — o problema real a resolver é a
fonte pequena das respostas geradas pela IA.

A escala é um **zoom de raiz** (`font-size` no `<html>`): como tokens,
fontes, entrelinhas e espaçamento estão em `rem`, **todos os elementos e
botões crescem junto** e o layout não estoura. Aplicação em tempo real nas
janelas de conteúdo, com autosave — mesmo desenho da opacidade (task 15).

## Entregáveis

- **Domínio:** `font_scale` (0,75–1,6) e `ai_font_scale` (1,0–1,5) em
  `domain::settings` (Rust) e `core/domain/settings` (TS), com
  `#[serde(default)]`, constantes `MIN/MAX/DEFAULT`, `validate` (recusa) e
  `sanitized` (clamp); helpers `clampFontScale`/`clampAiFontScale` no TS.
- **IPC:** `settings_preview_font_scales` (valida e emite
  `font_scales_preview`) e `settings_set_font_scales` (grava **só** esses
  campos, emite preview + `settings_changed`); `settings_set` (save global)
  também publica `font_scales_preview`. Registrados em `lib.rs`.
- **Hook** `useFontScale(enabled)` (App de cada webview): carrega as
  escalas, escuta `font_scales_preview`/`settings_changed` e aplica
  `font-size` (zoom) + `--oh-ai-font-scale` no `<html>`; captura a base do
  sistema **uma vez** (o zoom nunca se acumula) e remove a propriedade em
  100 %. Chamado com `enabled = route !== "overlay" && route !== "ask"`.
- **CSS:** tokens de tipografia `px` → **`rem`**; `--oh-ai-font-scale: 1` em
  `tokens.css`; no fim de `globals.css`, bloco "Escala extra do texto de IA"
  que multiplica as superfícies de IA (`.chat__row--ai .chat__text`,
  `.chat__translation`, `.markdown`/`--compact` + headings/tabela,
  `.summary-view__subject`, `.suggestion-card__title`,
  `.suggestion-block__text`, `.history__summary`, `.insights__text`);
  `font-size` fixos em `px` → `rem` e alturas de controles **com texto**
  (`.session-pill`, `.composer__input`, `.composer__remove`) → `rem`.
- **UI:** seção "Aparência & Fonte" no topo de `HudSettings` com dois cards
  (badge `%`, slider 5 p.p., legendas); `SettingsPage` trata os dois campos
  como a opacidade (aplica local, publica preview, autosave com debounce de
  200 ms, flush ao desmontar/fechar).
- **l10n:** 13 chaves novas (`hud_ai_font_scale*`, `hud_appearance_font*`,
  `hud_font_scale*`, `settings_failed_save_font_scale_error`) em **pt** e
  **en** (BR-086).

## Critérios de aceite

- [x] Slider global de 75 % a 160 % (passo 5) com badge ao vivo; 100 %
  remove o zoom e volta exatamente ao tamanho anterior.
- [x] Textos, botões, paddings e entrelinhas crescem **juntos** (nada de
  botão grande com texto pequeno) em Configurações, Chat, Resumo, Sugestões
  e Histórico.
- [x] Slider de IA (100–150 %) aumenta **só** conteúdo gerado pela IA;
  transcrições ME/THEY e demais elementos não mudam.
- [x] Notch e barra "Perguntar" permanecem com o tamanho/densidade próprios.
- [x] Preview em tempo real em todas as janelas de conteúdo sem salvar;
  autosave persiste os dois campos ao soltar o slider.
- [x] Fora da faixa: `validate` recusa, `sanitized` traz ao limite; NaN cai
  no default; settings antigas sem os campos carregam em 100 %.
- [x] Todo texto visível passa por `t()` (BR-086).

## Verificação

- `pnpm test` **453 ✓** (70 arquivos) · `tsc --noEmit` ✓.
  Inclui `src/styles/font-scale.test.ts` (guarda: tokens em `rem`, sem
  `font-size` em `px`, superfícies de IA com `--oh-ai-font-scale` **depois**
  da regra original, alturas de controle em `rem`) e
  `useFontScale.test.tsx` (aplica/remove, preview, escopo, clamp).
- `cargo test` **657 ✓** (10 ignorados) · `cargo clippy --all-targets` sem
  avisos · `cargo fmt --check` ✓.
- Limites travados em testes dos dois lados: 0,75–1,6 / 1,0–1,5.
