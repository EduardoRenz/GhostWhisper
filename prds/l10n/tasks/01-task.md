# Task 01 — l10n: catálogo, escolha de idioma e aba Geral & HUD

- **PRD:** [`../prd.md`](../prd.md)
- **Arquitetura:** [`../architecture/l10n.md`](../architecture/l10n.md)
- **Status:** CONCLUDED
- **IDs:** BR-010, BR-085, BR-086, R18–R22
- **Data:** 2026-10-22

## Objetivo

Traduzir todo o frontend para um catálogo l10n pt/en, permitir a escolha do
idioma em Configurações → Geral & HUD e resolver o idioma do sistema **uma
única vez**.

## Entregas

1. **Core**
   - `Settings.ui_language: Option<String>` + validação/sanitização.
   - `ui_language_from_locale` (pt* → pt, resto → en).
   - Porta `LocaleSource` + adapter `EnvLocaleSource` + dobra `FakeLocaleSource`.
   - `UiLanguageMigration` (idempotente) e wiring no `lib.rs`.
   - Testes de domínio, migração e locale.
2. **Frontend**
   - `src/core/l10n`: tipos, store, catálogos pt/en, `useTranslation`,
     `labels.ts` (enums/formatadores).
   - 100% das janelas e componentes com copy via `t("chave_snake_case")`.
   - `useSettings`/`App` sincronizam o idioma; troca reflete em todas as janelas.
   - `LanguageSettings` na aba Geral & HUD.
   - `SettingsPage`: **Geral & HUD** vira a primeira aba e o default.
3. **Regra/documentação**
   - `.agents/rules.md` §5.1 (R18–R22); AGENTS.md com regra rápida.
   - BR-085/BR-086 e revisão da BR-010.
   - PRD/arquitetura/task/memory/issues em `prds/l10n/`.

## Critérios de aceite cobertos

- CA-01.1 a CA-01.4 (escolha, primeira aba, persistência, restaurar padrões).
- CA-02.1 a CA-02.3 (detecção única, fallback e imutabilidade automática).
- CA-03.1 a CA-03.4 (copy via l10n, paridade pt/en, dados não traduzidos,
  re-render).

## Testes

- Rust: `cargo test ui_language` → domínio + migração (8 testes).
- Frontend: `pnpm test` → catálogo/paridade, store, `LanguageSettings`,
  `SettingsPage` (aba inicial, idioma), todos os componentes existentes.
- `pnpm exec tsc --noEmit` sem erros.

## Limites

- Erros do core e menu da bandeja não entram nesta entrega (documentado em
  [`../architecture/l10n.md`](../architecture/l10n.md) §6).
